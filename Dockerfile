ARG RUST_VERSION=1.76.0
FROM rust:${RUST_VERSION}-slim-bookworm AS build
WORKDIR /app
COPY . /app
RUN apt-get update && apt install -y openssl libssl-dev pkg-config libpq-dev
RUN cargo build --locked --release

FROM debian:bookworm-slim AS final
COPY --from=build /app/target/release/stop-piracy-shield /bin/stop-piracy-shield
RUN apt-get update && apt install -y openssl libpq5 ca-certificates
CMD ["/bin/stop-piracy-shield"]
