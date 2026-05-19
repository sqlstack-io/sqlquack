.PHONY: format check-format check-structure check-rust test-rust check verify

format:
	cargo fmt

check-format:
	cargo fmt --check

check-structure:
	cargo run -p sqlquack-structure-checker --quiet

check-rust:
	cargo check --all

test-rust:
	cargo test --all

check: check-format check-structure check-rust

verify: check test-rust
