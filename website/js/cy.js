const NODES_URL = "http://127.0.0.1:2310/nodes";
const DEFAULT_LAYOUT = {
    name: 'circle',
    nodeSpacing: 5,
    edgeLengthVal: 45,
    animate: true,
    randomize: false,
    maxSimulationTime: 1500
};

Promise.all([
    fetch('cy-style.json')
        .then(function (res) {
            return res.json();
        })
])
    .then(function (dataArray) {
        const cy = window.cy = cytoscape({
            container: document.getElementById('cy'),
            style: dataArray[0],
            elements: [],
            layout: {name: 'circle'}
        });

        // const starter = "system/library";
        // cy.add([{
        //     "group": "nodes",
        //     "data": {
        //         "id": starter,
        //         "name": starter,
        //         "score": 1,
        //         "query": true,
        //         "gene": true
        //     },
        //     "selectable": true,
        //     "grabbable": true,
        // }]).layout(DEFAULT_LAYOUT).run();

        cy.on('dblclick', 'node', function (evt) {
            const node = evt.target;
            const node_id = node.id();
            // console.log('tapped ' + node_id);

            let x = evt.position.x;
            let y = evt.position.y;
            // console.log(x, y);

            sendPostRequest(node_id).then(function (result) {
                // console.log(result);

                toNodes(result).then(function (elements) {
                    let elements_id = cy.add(elements);

                    elements_id.layout({
                        name: 'circle',
                        nodeSpacing: 1,
                        edgeLengthVal: 45,

                        // fit: true, // whether to fit the viewport to the graph
                        boundingBox: {x1: x, y1: y, x2: x, y2: y}, // constrain layout bounds; { x1, y1, x2, y2 } or { x1, y1, w, h }
                        avoidOverlap: true, // prevents node overlap, may overflow boundingBox and radius if not enough space
                        nodeDimensionsIncludeLabels: false, // Excludes the label when calculating node bounding boxes for the layout algorithm
                        spacingFactor: undefined, // Applies a multiplicative factor (>0) to expand or compress the overall area that the nodes take up
                        radius: undefined, // the radius of the circle
                        startAngle: 3 / 2 * Math.PI, // where nodes start in radians
                        // sweep: undefined, // how many radians should be between the first and last node (defaults to full circle)
                        animate: false, // whether to transition the node positions
                        transform: function (node, position) {
                            // console.log(position);
                            return position;
                        } // transform a given node position. Useful for changing flow direction in discrete layouts
                    }).run();

                })

                toEdges(node_id, result).then(function (elements) {
                    cy.add(elements);
                })

            });
        });
    });

async function sendPostRequest(pcg) {
    const response = await fetch(NODES_URL, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(pcg)
    });

    return await response.json();
}

async function toNodes(json) {
    const elements = [];

    for (let element of json) {
        let color = "#008b02";
        if (element[2] === "obsoleted") {
            color = "#000000";
        } else if (element[2] === "partly-obsoleted") {
            color = "#fccb00";
        } else if (element[2] === "renamed") {
            color = "#004dcf";
        }

        elements.push({
            "group": "nodes",
            "data": {
                "id": element[0],
                "name": element[0],
                "score": 1,
                "query": true,
                "gene": true
            },
            "style": {
                "background-color": color,
            },
            "background-color": color,
            "selected": true,
            "selectable": true,
            "grabbable": true,
        });
    }

    return elements;
}

async function toEdges(from, to_array) {
    const edges = [];


    for (let to of to_array) {

        // console.log(to[1])

        let color = "#008b02"; // runtime
        let line_style = "solid"; // runtime
        if (to[1] === "build") {
            color = "#004dcf";
        } else if (to[1] === "test") {
            color = "#abb8c3";
        } else if (to[1] === "system-build") {
            color = "#004dcf";
            line_style = "dashed";
        } else if (to[1] === "system-test") {
            color = "#abb8c3";
            line_style = "dashed";
        }

        edges.push({
            "data": {
                "id": from + to[0],
                "source": from,
                "target": to[0],
                "weight": 0.1,
                "arrow": "triangle"
            },
            "group": "edges",
            "selectable": true,
            "grabbed": false,
            "grabbable": true,
            "style": {
                "line-color": color,
                "line-style": line_style,
            }
        });
    }

    return edges;
}

function spawnPackage() {
    const userInput = document.getElementById("spawn-input").value;

    window.cy.add([{
        "group": "nodes",
        "data": {
            "id": userInput,
            "name": userInput,
            "score": 1,
            "query": true,
            "gene": true
        },
        "selectable": true,
        "grabbable": true,
    }]).layout(DEFAULT_LAYOUT).run();
}