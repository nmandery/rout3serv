FROM rust:1-bullseye as builder

RUN apt-get update && \
    apt-get install --no-install-recommends -y wget xz-utils python3-toml cmake git clang libssl-dev

ENV NODE_VERSION=16.13.0
RUN cd /tmp/ && \
    wget https://nodejs.org/dist/v$NODE_VERSION/node-v$NODE_VERSION-linux-x64.tar.xz &&\
    tar xf node-v$NODE_VERSION-linux-x64.tar.xz && \
    cp -r node-v$NODE_VERSION-linux-x64/* /

COPY . /build
RUN cd /build && \
    python3 docker-cargo-profile.py

# build typescript as - somehow - npm does not install a lot
# of dependencies when running in docker and being triggered by
# `build.rs`
# not this issue, but maybe related?
# https://github.com/nodejs/docker-node/issues/1005
#RUN cd /build/crates/til3serv/ui && \
#    npm i && \
#    npm run build

WORKDIR /build/crates/til3serv
#ENV SKIP_NPM=1
RUN cargo install --path . --root /usr/local

FROM debian:bullseye-slim
ENV RUST_BACKTRACE=1
ENV RUST_LOG="til3serv=info,h3io=info,tower_http::trace=debug"
COPY --from=builder /usr/local/bin/til3serv /usr/bin/
COPY ./crates/til3serv/config.yaml /config.yaml
EXPOSE 9001
ENTRYPOINT ["/usr/bin/til3serv"]
