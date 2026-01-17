RUST_WORKSPACE := .

.PHONY: install update lint type-check format clean test generate build ci

install:
	cargo fetch

update:
	cargo update

lint:
	cargo clippy --workspace --all-targets -- -D warnings

type-check:
	cargo check --workspace

format:
	cargo fmt --all

clean:
	cargo clean
	find . -name '*.log' -type f -delete

test:
	cargo test --workspace

generate:
	@echo "No API type generation configured."

build:
	cargo build --workspace

ci: generate format type-check lint build test
