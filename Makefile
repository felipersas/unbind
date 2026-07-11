.DEFAULT_GOAL := help

PORT ?= 3000
BIN := unbind

.PHONY: help build release fmt clippy test check run list json find kill install uninstall clean

help:
	@printf "Unbind commands:\n"
	@printf "  make run              Open the TUI with cargo run\n"
	@printf "  make list             List listening ports\n"
	@printf "  make find PORT=3000   Find who is using a port\n"
	@printf "  make json             List listening ports as JSON\n"
	@printf "  make kill PORT=3000   Kill the process using a port, with confirmation\n"
	@printf "  make install          Install unbind into Cargo's bin directory\n"
	@printf "  make build            Build debug binary\n"
	@printf "  make release          Build release binary\n"
	@printf "  make fmt              Check Rust formatting\n"
	@printf "  make clippy           Run Clippy\n"
	@printf "  make test             Run tests\n"
	@printf "  make check            Run fmt, clippy, tests, and build\n"
	@printf "  make uninstall        Remove installed unbind binary\n"
	@printf "  make clean            Remove build artifacts\n"

build:
	cargo build --locked

release:
	cargo build --release --locked

fmt:
	cargo fmt --check

clippy:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test --locked

check: fmt clippy test build

run:
	cargo run --

list:
	cargo run -- list

json:
	cargo run -- list --json

find:
	cargo run -- find $(PORT)

kill:
	cargo run -- kill $(PORT)

install:
	cargo install --path . --locked
	@printf "\nInstalled. Try: unbind\n"
	@if ! command -v $(BIN) >/dev/null 2>&1; then \
		printf "\nNote: %s is installed, but Cargo's bin directory is not on this shell's PATH.\n" "$(BIN)"; \
		printf "Add this to your shell config:\n"; \
		printf "  export PATH=\"$$HOME/.cargo/bin:$$PATH\"\n"; \
	fi

uninstall:
	cargo uninstall $(BIN)

clean:
	cargo clean
