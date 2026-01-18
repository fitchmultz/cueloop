RUST_WORKSPACE := .
PREFIX ?= $(HOME)/.local
BIN_DIR ?= $(PREFIX)/bin
BIN_NAME ?= ralph

.PHONY: install update lint type-check format clean test generate build build-release ci

install: build-release
	@bin_dir="$(BIN_DIR)"; \
	if [ ! -w "$$bin_dir" ]; then \
		bin_dir="$(CURDIR)/.local/bin"; \
		echo "install: $(BIN_DIR) not writable; using $$bin_dir"; \
	fi; \
	mkdir -p "$$bin_dir"; \
	install -m 0755 target/release/$(BIN_NAME) "$$bin_dir/$(BIN_NAME)"; \
	"$$bin_dir/$(BIN_NAME)" --help >/dev/null

update:
	cargo update

lint:
	cargo clippy --workspace --all-targets -- -D warnings

type-check:
	cargo check --workspace --all-targets

format:
	cargo fmt --all

clean:
	cargo clean
	find . -name '*.log' -type f -delete

test:
	cargo test --workspace --all-targets -- --include-ignored
	cargo test --workspace --all-targets -- --include-ignored --test-threads=1
	RUSTDOCFLAGS="-D warnings" cargo test --workspace --doc -- --include-ignored
	cargo test --workspace --all-targets --release -- --include-ignored
	cargo build --workspace --release

generate:
	@echo "No API type generation configured."

build:
	cargo build --workspace

build-release:
	cargo build --workspace --release

ci: generate format type-check lint build test install
