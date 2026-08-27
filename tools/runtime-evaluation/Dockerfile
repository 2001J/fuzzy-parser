# Isolated #11 evaluation only; not a production adapter or deployment image.
FROM rust:1.96.0-bookworm@sha256:5e2214abe154fe26e39f64488952e5c991eeed1d6d6da7cc8381ae83927f0cfc AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY crates ./crates
RUN rustc -Vv && cargo -V && cargo build --release --locked -p parser-cli

FROM node:22-bookworm-slim@sha256:83f487e0a63425e5b4d146fb5e5be574bcbe1b7b843d3ebafdd95eaf7767a7e5
WORKDIR /evaluation
COPY --from=build /src/target/release/parser-cli /opt/parser-cli
COPY tools/runtime-evaluation/evaluate.mjs ./tools/runtime-evaluation/evaluate.mjs
COPY fixtures ./fixtures
USER node
ENTRYPOINT ["node", "/evaluation/tools/runtime-evaluation/evaluate.mjs", "/opt/parser-cli"]
