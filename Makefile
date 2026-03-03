.PHONY: build dev

build:
	cd cg-arena-ui && npm ci && npm run build
	cargo build --release

dev:
	cargo run -- run test_arena