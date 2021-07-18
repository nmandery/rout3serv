# dockerfile for the route3 server application
FROM nmandery/gdal-minimal:3-bullseye as basesystem

FROM basesystem as builder
RUN apt-get update && \
    apt-get install --no-install-recommends -y cmake curl make clang git python3-toml pkg-config
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y \
        --profile minimal \
        --default-toolchain stable
COPY . /build
RUN cd /build && \
    python3 docker-cargo-profile.py && \
    cd /build/route3_road && \
    PATH=$PATH:$HOME/.cargo/bin cargo install --path . --root /usr/local

FROM basesystem
ENV RUST_BACKTRACE=1
ENV RUST_LOG="route3_road=info,route3_core=info"
COPY --from=builder /usr/local/bin/route3_road /usr/bin/
COPY ./route3_road/server-config.example.toml /server-config.toml
EXPOSE 7088
ENTRYPOINT ["/usr/bin/route3_road"]
