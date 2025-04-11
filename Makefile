.PHONY: check test coverage

check:
	cargo fmt --check --all
	cargo clippy --all-targets

test:
	cargo test --features testutils

coverage:
	cargo +nightly llvm-cov --features testutils --branch --html
