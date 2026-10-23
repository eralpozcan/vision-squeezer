# HTTP API for workflow tools (n8n, Make, Zapier): docker build -t vision-squeezer .
#                                                  docker run -p 8080:8080 vision-squeezer
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches
RUN cargo build --release --locked --bin vision-squeezer

FROM debian:bookworm-slim
RUN useradd --create-home squeezer
COPY --from=build /src/target/release/vision-squeezer /usr/local/bin/vision-squeezer
USER squeezer
EXPOSE 8080
# 0.0.0.0 inside the container; publish the port only where you need it.
CMD ["vision-squeezer", "serve", "--host", "0.0.0.0", "--port", "8080"]
