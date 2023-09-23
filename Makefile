build: build_release download_cytoscape

build_release:
	cargo build --release
	mv target/release/oi-pkg-visualizer .

download_cytoscape:
	curl -o website/js/cytoscape.min.js https://cdnjs.cloudflare.com/ajax/libs/cytoscape/3.26.0/cytoscape.min.js

clean:
	rm website/js/cytoscape.min.js
	rm oi-pkg-visualizer
	cargo clean