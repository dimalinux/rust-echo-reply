.PHONY: build
build:
	cargo build
	cargo build --release

.PHONY: lint
lint:
	cargo clippy --all-targets -- -A clippy::pedantic -A clippy::style

.PHONY: format,fmt
format:
	cargo fmt

.PHONY: test
test:
	cargo test

.PHONY: coverage
coverage:
	command -v grcov >/dev/null 2>&1 || cargo install grcov
	rustup component add llvm-tools-preview
	rm -rf target/coverage
	mkdir -p target/coverage
	CARGO_INCREMENTAL=0 RUSTFLAGS=-Cinstrument-coverage LLVM_PROFILE_FILE=cargo-test-%p-%m.profraw cargo test
	grcov . --binary-path ./target/debug/deps/ -s . -t cobertura,html --branch --ignore-not-existing --ignore '../*' --ignore '/*' -o target/coverage
	rm -f cargo-test*\.profraw
	@printf "\nCoverage is in:\nfile://$(PWD)/target/coverage/html/index.html\n"

.PHONY: clean
clean:
	cargo clean
