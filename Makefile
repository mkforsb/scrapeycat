.PHONY: clean check test nextest coverage

clean:
	cargo clean -p scrapeycat

check:
	cargo fmt --check --all
	cargo clippy --all-targets

test:
	cargo test --features testutils

nextest:
	cargo nextest run --features testutils

coverage:
	cargo +nightly llvm-cov --features testutils --branch --html
