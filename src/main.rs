use std::{
    env::args,
    net::SocketAddr
};
use axum::{
    extract::State,
    routing::{get, post},
    http::StatusCode,
    Json, Router, Server
};
use fmri::FMRI;
use serde::{Deserialize, Serialize};
use tokio::signal;
use tower_http::cors::CorsLayer;
use tracing_subscriber::fmt::init;
use oi_pkg_checker_core::{Components, DependTypes, DependencyTypes, Package, PackageVersions};

/// Represents package name
#[derive(Deserialize, Debug)]
struct PackageName(String);

/// Represents nodes(package_name, depend_type(Runtime/Build/Test/SystemBuild/SystemTest/None), package_type(obsoleted/partly-obsoleted/renamed/none))
#[derive(Serialize)]
struct Nodes(Vec<(String, String, String)>);

#[tokio::main]
async fn main() {
    init();

    let args: Vec<String> = args().collect();

    if args.len() != 3 {
        panic!("Usage: {} <listening_addr_and_port> <data_path>", args[0]);
    }

    let components = Components::deserialize(&args[2]);

    let addr = match args[1].parse::<SocketAddr>() {
        Ok(socket_addr) => socket_addr,
        Err(e) => {
            panic!("Failed to parse SocketAddr: {}", e);
        }
    };

    let app = Router::new()
        .route("/", get(discover))
        .route("/nodes", post(nodes))
        .route("/package_type", post(package_type))
        .with_state(components)
        .layer(CorsLayer::permissive());

    tracing::debug!("listening on {}", addr);
    Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

/// Basic discover handler for testing
async fn discover() -> &'static str {
    tracing::debug!("discovered");
    "discovered!"
}

/// Handler for getting package type
async fn package_type(
    State(components): State<Components>,
    Json(package): Json<PackageName>,
) -> (StatusCode, String) {

    tracing::debug!("got request on package: {:?}", package);

    for component in components.get_ref() {
        for package_versions in component.get_versions_ref() {
            if package_versions.fmri_ref().get_package_name_as_ref_string() == &package.0 {
                if package_versions.is_obsolete() {
                    return (StatusCode::OK, "partly-obsoleted".to_owned());
                }

                if package_versions.is_renamed() {
                    return (StatusCode::OK, "renamed".to_owned());
                }

                for fmri in components.get_obsoleted_ref().get_ref() {
                    if fmri.get_package_name_as_ref_string() == &package.0 {
                        return (StatusCode::OK, "obsoleted".to_owned());
                    }
                }

                // TODO: add returning Non-existent type
                return (StatusCode::NOT_FOUND, "none".to_owned());
            }
        }
    }

    (StatusCode::NOT_FOUND, "none".to_owned())
}

/// Handler for returning dependencies(nodes) of package
async fn nodes(
    State(components): State<Components>,
    Json(package): Json<PackageName>,
) -> (StatusCode, Json<Nodes>) {
    let nodes: &mut Vec<(String, String, String)> = &mut Vec::new();

    tracing::debug!("got request on package: {:?}", package);

    let mut obsoleted_packages: Vec<String> = Vec::new();
    for fmri in components.get_obsoleted_ref().get_ref() {
        obsoleted_packages.push(fmri.get_package_name_as_ref_string().clone())
    }

    let mut found = false;
    for component in components.get_ref() {
        for package_versions in component.get_versions_ref() {
            if package_versions.fmri_ref().get_package_name_as_ref_string() == &package.0 {
                if package_versions.is_obsolete() {
                    return (StatusCode::OK, Json(Nodes(nodes.clone())));
                }

                let package = package_versions.get_packages_ref().last().unwrap();

                get_nodes_from_dependencies(nodes, components.clone(), &obsoleted_packages, DependencyTypes::Runtime, package);
                get_nodes_from_dependencies(nodes, components.clone(), &obsoleted_packages, DependencyTypes::Build, package);
                get_nodes_from_dependencies(nodes, components.clone(), &obsoleted_packages, DependencyTypes::Test, package);
                get_nodes_from_dependencies(nodes, components.clone(), &obsoleted_packages, DependencyTypes::SystemBuild, package);
                get_nodes_from_dependencies(nodes, components.clone(), &obsoleted_packages, DependencyTypes::SystemTest, package);

                found = true;
                break;
            }
        }

        if found { break; }
    }

    if !found {
        tracing::error!("package: {:?} not found", package);
        return (StatusCode::NOT_FOUND, Json(Nodes(nodes.clone())));
    }

    tracing::debug!("sending on package: {:?} packages: {:?}", package, nodes);
    (StatusCode::OK, Json(Nodes(nodes.clone())))
}

/// Simply extracts package names from dependencies
fn get_nodes_from_dependencies(nodes: &mut Vec<(String, String, String)>, components: Components, obsoleted_packages: &Vec<String>, dependency_type: DependencyTypes, package: &Package) {
    let (d_type, dependencies) = match dependency_type {
        DependencyTypes::Runtime => ("runtime".to_owned(), package.get_runtime_dependencies()),
        DependencyTypes::Build => ("build".to_owned(), package.get_build_dependencies()),
        DependencyTypes::Test => ("test".to_owned(), package.get_test_dependencies()),
        DependencyTypes::SystemBuild => ("system-build".to_owned(), package.get_system_build_dependencies()),
        DependencyTypes::SystemTest => ("system-test".to_owned(), package.get_system_test_dependencies()),
        DependencyTypes::None => panic!()
    };

    let node_type_closure = |
        package_name: &String,
        package: &PackageVersions,
        obsoleted_packages: &[String]
    | -> String {
        if package.is_obsolete() {
            return "partly-obsoleted".to_owned();
        } else if package.is_renamed() {
            return "renamed".to_owned();
        }

        return "none".to_owned();
    };

    for dependency in dependencies {
        let package_name = &match dependency.get_ref() {
            DependTypes::Require(fmri) => fmri.clone().get_package_name_as_string(),
            DependTypes::Optional(fmri) => fmri.clone().get_package_name_as_string(),
            DependTypes::Incorporate(fmri) => fmri.clone().get_package_name_as_string(),
            DependTypes::RequireAny(fmri_list) => {
                for fmri in fmri_list.get_ref() {
                    let package_name = &fmri.clone().get_package_name_as_ref_string().clone();

                    match &components.get_package_versions_from_fmri(&FMRI::parse_raw(package_name)) {
                        None => {
                            // obsoleted or Non-existent
                            for obsoleted_package in obsoleted_packages {
                                if obsoleted_package == package_name {
                                    nodes.push((
                                        package_name.clone(),
                                        d_type.clone(),
                                        "obsoleted".to_owned()
                                    ));
                                }
                            }

                            // todo!("add Non-existent packages");
                            // nodes.push((
                            //     package_name.clone(),
                            //     d_type.clone(),
                            //     "None".to_owned()
                            // ));
                        }
                        Some(package) => {

                            nodes.push((
                                package_name.clone(),
                                d_type.clone(),
                                node_type_closure(&package_name, package, obsoleted_packages)
                            ));
                        }
                    }
                }
                continue;
            }
            DependTypes::Conditional(fmri, _) => fmri.clone().get_package_name_as_string(),
            DependTypes::Group(fmri) => fmri.clone().get_package_name_as_string(),
            _ => unimplemented!()
        };

        println!("{}", package_name);

        match &components.get_package_versions_from_fmri(&FMRI::parse_raw(package_name)) {
            None => {
                // obsoleted or Non-existent
                for obsoleted_package in obsoleted_packages {
                    if obsoleted_package == package_name {
                        nodes.push((
                            package_name.clone(),
                            d_type.clone(),
                            "obsoleted".to_owned()
                        ));
                    }
                }

                // todo!("add Non-existent packages");
                // nodes.push((
                //     package_name.clone(),
                //     d_type.clone(),
                //     "None".to_owned()
                // ));
            }
            Some(package) => {
                nodes.push((
                    package_name.clone(),
                    d_type.clone(),
                    node_type_closure(&package_name, package, obsoleted_packages)
                ));
            }
        }
    }
}

///  For graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::debug!("signal received, starting graceful shutdown");
}