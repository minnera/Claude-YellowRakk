FROM rust:1.97-trixie AS build
WORKDIR /src
ENV SQLX_OFFLINE=true
COPY . .
RUN cargo build --release --locked --bin yellowrakk

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 rakk \
    && mkdir /data && chown rakk /data
COPY --from=build /src/target/release/yellowrakk /usr/local/bin/yellowrakk
USER rakk
WORKDIR /home/rakk
ENV DATABASE_URL=sqlite:///data/yellowrakk.db \
    BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info
VOLUME /data
EXPOSE 8080
CMD ["yellowrakk"]
