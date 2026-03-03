.PHONY: build

build:
	cd cg-arena-ui && npm ci && npm run build
	cargo build --release
