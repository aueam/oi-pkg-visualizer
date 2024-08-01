build: download_cytoscape build_release

build_release:
	cargo build --release

download_cytoscape:
	curl -o website/js/cytoscape.min.js https://cdnjs.cloudflare.com/ajax/libs/cytoscape/3.26.0/cytoscape.min.js

clean:
	rm website/js/cytoscape.min.js
	rm oi-pkg-visualizer
	cargo clean