.PHONY: all build check help rel test

all: check build test

check:
	cargo clippy && cargo fmt --check

build:
	cargo build

help:
	ipgrep=$$(ls -t $$(find target/ -name ipgrep) | head -n1) && \
	  $$ipgrep --help

rel:
	cargo build --release

test:
	cargo test
