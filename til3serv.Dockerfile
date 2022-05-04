FROM rust:1-bullseye as builder

RUN apt-get update && \
    apt-get install --no-install-recommends -y wget xz-utils python3-toml git clang libssl-dev

# cmake >3.20 is required, so we install from source
RUN cd /tmp && \
    curl -L -o cmake.tgz https://github.com/Kitware/CMake/releases/download/v3.22.2/cmake-3.22.2.tar.gz && \
    tar xf cmake.tgz && \
    cd cmake-3.22.2 && \
    ./bootstrap && \
    make -j3 && \
    make install

ENV NODE_VERSION=16.13.0
RUN cd /tmp/ && \
    wget https://nodejs.org/dist/v$NODE_VERSION/node-v$NODE_VERSION-linux-x64.tar.xz &&\
    tar xf node-v$NODE_VERSION-linux-x64.tar.xz && \
    cp -r node-v$NODE_VERSION-linux-x64/* /

COPY . /build
RUN cd /build && \
    python3 docker-cargo-profile.py && \
    cd /build/crates/til3serv && \
    export RUSTFLAGS='-C target-feature=+fxsr,+sse,+sse2,+sse3,+ssse3,+sse4.1,+sse4.2,+popcnt,+avx,+fma' && \
    cargo install --path . --root /usr/local

run strip /usr/local/bin/til3serv

FROM debian:bullseye-slim
ENV RUST_BACKTRACE=1
ENV RUST_LOG="til3serv=info,s3io=info,tower_http::trace=debug"
COPY --from=builder /usr/local/bin/til3serv /usr/bin/
COPY ./crates/til3serv/config.example.yaml /config.yaml
EXPOSE 9001
ENTRYPOINT ["/usr/bin/til3serv"]
