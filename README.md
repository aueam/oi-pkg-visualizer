# oi-pkg-visualizer

Is website for visualizing dependencies between packages from OpenIndiana.

![example.png](example.png)

## How to

### Build

1. First of all, you need to run [oi-pkg-checker](https://github.com/aueam/oi-pkg-checker) to get `data.bin`.
2. Then just run `make`, it will download [cytoscape.js](https://github.com/cytoscape/cytoscape.js) and compile server.

### Use

- Run server with `target/release/oi-pkg-visualizer 127.0.0.1:2310 /tmp/data.bin`
    - `127.0.0.1:2310` is listening address and port of server
    - `/tmp/data.bin` is the data from the [oi-pkg-checker](https://github.com/aueam/oi-pkg-checker)
- if necessary, change the server address and/or port and/or transfer protocol in the `website/js/cy.js` (first line)
- Visit `website/index.html`

## Style legend

### Nodes

- default = green
- obsoleted = black
- obsoleted (but with older not obsoleted version) = yellow
- renamed = blue

### Edges

- default = green
- build = blue
- test = grey
- system-build = dashed blue
- system-test = dashed grey