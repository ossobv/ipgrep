.PHONY: all build check deb help rel release test

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

release:
	# The debian releases get a version without the "v"-prefix.
	GIT_VERSION=$$(git describe --always --dirty=-modified) && \
	  ./build-docker.sh "$${GIT_VERSION#v}"
	./ipgrep --version

test:
	cargo test
	input=$$(printf '%s\n' \
	  1 2 '3 192.168.1.1' 4 5 6 '7 192.168.1.1' \
	  '8 192.168.1.2' '9 192.168.1.1' 10 11 12 \
	  1 2 '3 192.168.1.1' 4 5 6 '7 192.168.1.1' \
	  '8 192.168.1.2' '9 192.168.1.1' 10 11 12) && \
	grep_out=$$(echo "$$input" | \
	  grep 192.168.1.1 -C1 -n /dev/stdin /dev/null) && \
	ipgrep_out=$$(echo "$$input" | \
	  ./target/debug/ipgrep 192.168.1.1 -C1 -n /dev/stdin /dev/null) && \
	grep_out=$$grep_out ipgrep_out=$$ipgrep_out bash -c \
	  'diff -pu <(echo "$$grep_out") <(echo "$$ipgrep_out")'
