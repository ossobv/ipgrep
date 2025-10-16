#FROM rust:1.75 AS builder
FROM ghcr.io/rust-cross/rust-musl-cross:x86_64-musl AS builder

USER root
# Unfortunately also giving write perms to /srv/ipgrep so we can locally
# alter Cargo.toml for the build.
RUN mkdir -p /src/ipgrep/target /usr/local/cargo && \
    chown nobody: /src/ipgrep /src/ipgrep/target /usr/local/cargo

ENV CARGO_HOME=/usr/local/cargo

USER nobody
WORKDIR /src/ipgrep

RUN cargo --version
RUN cargo install cargo-auditable cargo-deb

# Copy prerequisites for cargo update/fetch
COPY Cargo.lock Cargo.toml /src/ipgrep/
COPY src/lib.rs /src/ipgrep/src/lib.rs

# Update/fetch
RUN cargo update --dry-run --locked
RUN cargo fetch --locked --verbose

# Waiting on https://github.com/rust-lang/cargo/issues/2644
# Then we could do a pre-build before adding most of our sources.
#RUN GIT_VERSION=irrelevant cargo auditable build --locked --features=version-from-env \
#      --release --target x86_64-unknown-linux-musl

# Copy the rest of the source
COPY . /src/ipgrep
RUN sed -i -e 's@target/release@target/docker@;s@#USED_BY_DOCKER#@@' Cargo.toml
#RUN cargo update --dry-run --locked
#RUN cargo fetch --locked --verbose
#RUN rustup target add x86_64-unknown-linux-musl

ARG GIT_VERSION
#RUN cargo build --features=version-from-env
#RUN cargo test --features=version-from-env
#RUN cargo bench --features=version-from-env
RUN cargo auditable build --locked --features=version-from-env \
      --release --target x86_64-unknown-linux-musl
RUN test "$(echo $(ldd target/x86_64-unknown-linux-musl/release/ipgrep))" = "statically linked"

# Record build tools in build-info.json, move relevant files to target/docker/.
RUN mkdir -p target/docker/ && \
    printf '%s\n' >target/docker/build-info.json \
      '{' \
      "  \"cargo\": \"$(cargo --version)\"," \
      "  \"cargo-auditable\": \"$(cargo auditable --version)\"," \
      "  \"cargo-deb\": \"$(cargo deb --version)\"," \
      "  \"rustc\": \"$(rustc --version)\"" \
      '}' && \
    cp -a /src/ipgrep/target/x86_64-unknown-linux-musl/release/ipgrep target/docker/ && \
    true
RUN grep "^${GIT_VERSION#v} (" CHANGES.rst
RUN cargo deb --no-build -o target/docker/

FROM scratch
COPY --from=builder /src/ipgrep/target/docker/* /
