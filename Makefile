.PHONY: build run release clean deploy test

build:
	cargo build --workspace

release:
	@cargo build --release --workspace

run: release
	./target/release/pii-engineer-server

test:
	cargo test --workspace

clean:
	cargo clean

deploy:
	bash deploy.sh
