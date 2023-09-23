# oi-pkg-visualizer

Is website for visualizing dependencies between packages from OpenIndiana.

![example.png](example.png)

## How to

### Build

Just run `make`, it will download [cytoscape.js](https://github.com/cytoscape/cytoscape.js) and compiles server with
cargo.

### Use

- Run server with `./oi-pkg-visualizer 127.0.0.1:2310 /tmp/data.bin`
    - `/tmp/data.bin` represents output data from [oi-pkg-checker](https://github.com/aueam/oi-pkg-checker)
- Visit `website/index.html`