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
	cargo auditable build --release
	@objcopy --dump-section .dep-v0=/dev/stdout target/release/ipgrep | \
	  python3 -c "$$(printf '%s\n' 'import zlib,sys' \
	    'd=zlib.decompress(sys.stdin.buffer.read()).decode("utf-8")' \
	    'print("(embedded SBOM) " + d[0:60] + "...")')"

test:
	cargo test
