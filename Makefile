.PHONY: build
build:
	cargo build

.PHONY: lint
lint:
	cargo clippy --all-features --all-targets

.PHONY: format,fmt
fmt:
	cargo fmt
format: fmt

.PHONY: test
test: 
	cargo test

# Deletes all executables matching the directory names in cmd/
.PHONY: clean
clean:
	rm -r target/
