.PHONY: coverage

coverage:
	cargo +nightly llvm-cov --branch --html
