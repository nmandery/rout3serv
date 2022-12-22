# dockerfile for the route3 server application
FROM docker.io/rust:1-slim-bullseye as builder

RUN apt-get update && \
    apt-get install --no-install-recommends -y make clang git python3-toml pkg-config libgdal-dev protobuf-compiler

COPY . /build
RUN cd /build && \
    python3 docker-cargo-profile.py && \
    cd /build/crates/rout3serv && \
    export RUSTFLAGS='-C target-feature=+fxsr,+sse,+sse2,+sse3,+ssse3,+sse4.1,+sse4.2,+popcnt,+avx,+fma' && \
    PATH=$PATH:$HOME/.cargo/bin cargo install --path . --root /usr/local && \
    PATH=$PATH:$HOME/.cargo/bin cargo clean && \
    strip /usr/local/bin/rout3serv

FROM docker.io/debian:bullseye-slim
RUN apt-get update && \
    apt-get install --no-install-recommends -y libgdal28 && \
    apt-get clean

# "0" -> let rayon determinate how many threads to use. Defaults to one per CPU core
ENV RAYON_NUM_THREADS="0"
ENV RUST_BACKTRACE=1
ENV RUST_LOG="rout3serv=info,s3io=info,tower_http::trace=debug"
COPY --from=builder /usr/local/bin/rout3serv /usr/bin/
COPY ./crates/rout3serv/config.example.yaml /config.yaml
COPY ./crates/rout3serv/proto /
EXPOSE 7088
ENTRYPOINT ["/usr/bin/rout3serv"]
