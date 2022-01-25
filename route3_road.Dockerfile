# dockerfile for the route3 server application
FROM nmandery/gdal-minimal:3-bullseye as basesystem

FROM basesystem as builder
RUN apt-get update && \
    apt-get install --no-install-recommends -y cmake curl make clang git python3-toml pkg-config libssl-dev
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y \
        --profile minimal \
        --default-toolchain stable
COPY . /build
RUN cd /build && \
    python3 docker-cargo-profile.py && \
    cd /build/crates/route3_road && \
    PATH=$PATH:$HOME/.cargo/bin cargo install --path . --root /usr/local && \
    strip /usr/local/bin/route3_road

FROM basesystem
# "0" -> let rayon determinate how many threads to use. Defaults to one per CPU core
ENV RAYON_NUM_THREADS="0"
ENV RUST_BACKTRACE=1
ENV RUST_LOG="route3_road=info,s3io=info,tower_http::trace=debug"
COPY --from=builder /usr/local/bin/route3_road /usr/bin/
COPY ./crates/route3_road/server-config.example.yaml /server-config.yaml
COPY ./crates/route3_road/proto /
EXPOSE 7088
ENTRYPOINT ["/usr/bin/route3_road"]
