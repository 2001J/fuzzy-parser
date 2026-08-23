FROM rust:1-bookworm AS builder

WORKDIR /src
COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY crates ./crates
COPY src ./src

RUN cargo build --release -p parser-cli

FROM debian:bookworm-slim AS runtime

RUN groupadd --system --gid 10001 parser \
    && useradd --system --uid 10001 --gid parser --create-home parser

COPY --from=builder /src/target/release/parser-cli /usr/local/bin/parser-cli

USER parser
ENTRYPOINT ["/usr/local/bin/parser-cli"]
