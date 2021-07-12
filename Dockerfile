# dockerfile for the route3 server application
FROM debian:buster-slim as basesystem
RUN apt-get update && \
    apt-get install --no-install-recommends -y libgdal20 && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /usr/local/bin/

FROM rust:1-buster as builder
RUN apt-get update && \
    apt-get install --no-install-recommends -y cmake libgdal20 make clang git libgdal-dev && \
    rustup component add rustfmt
COPY . /build
RUN cd /build/route3 && \
    cargo install --path .

FROM basesystem
COPY --from=builder /usr/local/cargo/bin/route3 /usr/bin/
COPY ./route3/server-config.example.toml /server-config.toml
EXPOSE 7888
USER 7888
ENTRYPOINT ["/usr/bin/route3"]
