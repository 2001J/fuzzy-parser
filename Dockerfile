FROM rust:1.96.0-bookworm@sha256:5e2214abe154fe26e39f64488952e5c991eeed1d6d6da7cc8381ae83927f0cfc AS builder

WORKDIR /src
COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY crates ./crates
COPY src ./src

RUN cargo build --release --locked -p parser-cli

FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171 AS runtime

ARG VERSION=0.1.0
ARG REVISION=unknown
LABEL org.opencontainers.image.title="Fuzzy Parser CLI" \
      org.opencontainers.image.description="Domain-neutral batch parser CLI" \
      org.opencontainers.image.version="$VERSION" \
      org.opencontainers.image.revision="$REVISION" \
      org.opencontainers.image.source="https://github.com/2001J/fuzzy-parser" \
      org.opencontainers.image.licenses="Apache-2.0"

RUN groupadd --system --gid 10001 parser \
    && useradd --system --uid 10001 --gid parser --create-home parser

COPY --from=builder /src/target/release/parser-cli /usr/local/bin/parser-cli

USER parser
ENTRYPOINT ["/usr/local/bin/parser-cli"]
