use axum::http::{header, HeaderValue};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use fmri::FMRI;
use oi_pkg_checker_core::packages::{
    components::Components, depend_types::DependTypes, package::Package,
};
use serde::Serialize;
use std::{
    env::args,
    fmt::Debug,
    net::SocketAddr,
    sync::{Mutex, Weak},
};
use tokio::{net::TcpListener, signal};
use tower_http::cors::CorsLayer;
use tracing_subscriber::fmt::init;

/// Represents nodes(package_name, depend_type(Runtime/Build/Test/SystemBuild/SystemTest/None), package_type(obsoleted/partly-obsoleted/renamed/none))
#[derive(Serialize)]
struct Nodes(Vec<(String, String, PackageType)>);

#[derive(Serialize, Debug, Clone)]
enum PackageType {
    Renamed,
    PartlyObsoleted,
    Obsoleted,
    Normal,
}

macro_rules! html {
    ($p:expr) => {
        get(|| async { Html(include_str!($p)) })
    };
}

macro_rules! content_type {
    ($p:expr, $t:expr) => {
        get(|| async {
            (
                [(header::CONTENT_TYPE, HeaderValue::from_static($t))],
                include_str!($p),
            )
                .into_response()
        })
    };
}

macro_rules! css {
    ($p:expr) => {
        content_type!($p, "text/css")
    };
}

macro_rules! js {
    ($p:expr) => {
        content_type!($p, "application/javascript")
    };
}

macro_rules! json {
    ($p:expr) => {
        content_type!($p, "application/json")
    };
}

#[tokio::main]
async fn main() {
    init();

    let args: Vec<String> = args().collect();

    if args.len() != 3 {
        panic!("Usage: {} <listening_addr_and_port> <data_path>", args[0]);
    }

    let addr = match args[1].parse::<SocketAddr>() {
        Ok(socket_addr) => socket_addr,
        Err(e) => {
            panic!("Failed to parse SocketAddr: {}", e);
        }
    };

    let app = Router::new()
        .route("/", html!("../website/index.html"))
        .route("/index.html", html!("../website/index.html"))
        .route("/style.css", css!("../website/css/style.css"))
        .route("/cy.js", js!("../website/js/cy.js"))
        .route("/cytoscape.min.js", js!("../website/js/cytoscape.min.js"))
        .route("/cy-style.json", json!("../website/cy-style.json"))
        .route("/nodes", post(nodes))
        .route("/package_type", post(package_type))
        .with_state(Components::deserialize(&args[2]).unwrap())
        .layer(CorsLayer::permissive());

    tracing::info!("listening on {}", addr);
    axum::serve(
        TcpListener::bind(addr).await.unwrap(),
        app.into_make_service(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

/// Handler for getting package type
async fn package_type(
    State(components): State<Components>,
    Json(package_name): Json<String>,
) -> (StatusCode, String) {
    tracing::debug!("got request on package: {:?}", package_name);

    (
        StatusCode::OK,
        match type_of_package(&components, &FMRI::parse_raw(&package_name).unwrap()) {
            PackageType::Renamed => "Renamed",
            PackageType::PartlyObsoleted => "PartlyObsoleted",
            PackageType::Obsoleted => "Obsoleted",
            PackageType::Normal => "None",
        }
        .to_owned(),
    )
}

/// Handler for returning dependencies(nodes) of package
async fn nodes(
    State(components): State<Components>,
    Json(package_name): Json<String>,
) -> (StatusCode, Json<Nodes>) {
    tracing::debug!("got request on package: {:?}", package_name);

    let nodes: &mut Vec<(String, String, PackageType)> = &mut Vec::new();

    let package = match components.get_package_by_fmri(&FMRI::parse_raw(&package_name).unwrap()) {
        Ok(p) => p,
        Err(_) => return (StatusCode::NOT_FOUND, Json(Nodes(nodes.clone()))),
    }
    .lock()
    .unwrap();

    let mut add = |f: &FMRI, label: &str| {
        nodes.push((
            f.clone().get_package_name_as_string(),
            label.to_owned(),
            type_of_package(&components, f),
        ));
    };

    for d in package
        .get_versions()
        .first()
        .unwrap()
        .get_runtime_dependencies()
    {
        match d {
            DependTypes::Require(f)
            | DependTypes::Optional(f)
            | DependTypes::Exclude(f)
            | DependTypes::Incorporate(f)
            | DependTypes::Origin(f)
            | DependTypes::Conditional(f, _)
            | DependTypes::Group(f)
            | DependTypes::Parent(f) => {
                add(f, "Runtime");
            }
            DependTypes::RequireAny(f_list) | DependTypes::GroupAny(f_list) => {
                for f in f_list.get_ref() {
                    add(f, "Runtime");
                }
            }
        }
    }

    let component = match package.is_in_component() {
        None => return (StatusCode::OK, Json(Nodes(nodes.clone()))),
        Some(c) => c,
    }
    .lock()
    .unwrap();

    let mut check_deps = |deps: &Vec<Weak<Mutex<Package>>>, label: &str| {
        for p in deps {
            if let Ok(b) = p.upgrade().unwrap().try_lock() {
                add(b.get_fmri(), label)
            }
        }
    };

    check_deps(component.get_build_dependencies(), "Build");
    check_deps(component.get_test_dependencies(), "Test");
    check_deps(component.get_sys_build_dependencies(), "SystemBuild");
    check_deps(component.get_sys_test_dependencies(), "SystemTest");

    (StatusCode::OK, Json(Nodes(nodes.clone())))
}

fn type_of_package(components: &Components, fmri: &FMRI) -> PackageType {
    let package = match components.get_package_by_fmri(fmri) {
        Ok(p) => p,
        Err(_) => return PackageType::Normal,
    }
    .lock()
    .unwrap();

    if package.is_renamed() {
        return PackageType::Renamed;
    }

    if package.is_obsolete() {
        if !package.get_versions().first().unwrap().is_obsolete() {
            return PackageType::PartlyObsoleted;
        }
        return PackageType::Obsoleted;
    }

    PackageType::Normal
}

/// For graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
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
