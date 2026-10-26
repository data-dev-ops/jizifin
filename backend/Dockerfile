# Rust backend. Multi-arch: linux/arm64, linux/amd64.
FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /src
# Linker workaround for libduckdb-sys; present in every stage so cook and build use identical flags.
COPY .cargo .cargo

FROM chef AS planner
COPY Cargo.toml Cargo.lock clippy.toml ./
COPY crates crates
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock clippy.toml ./
COPY crates crates
RUN cargo build --release --locked -p jizifin-api

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /src/target/release/jizifin-api /usr/local/bin/jizifin-api
ENV JIZIFIN_BIND=0.0.0.0:8000 JIZIFIN_DB_PATH=/data/finance.duckdb
VOLUME /data
EXPOSE 8000
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 CMD ["jizifin-api", "--healthcheck"]
ENTRYPOINT ["jizifin-api"]
