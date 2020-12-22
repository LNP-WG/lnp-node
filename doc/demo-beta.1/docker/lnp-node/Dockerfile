FROM rust:1.47.0-slim-buster as builder

RUN apt-get update -y \
    && apt-get install -y \
        libsqlite3-dev \
        libssl-dev \
        libzmq3-dev \
        pkg-config \
        cmake \
        g++

ENV SRC=/usr/local/src/lnpnode

WORKDIR ${SRC}

RUN cargo install --locked --root "${SRC}" lnp_node --version 0.1.0-beta.3 --all-features


FROM debian:buster-slim

RUN apt-get update -y \
    && apt-get install -y \
        libzmq3-dev \
        libsqlite3-dev \
        tini \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

ARG SRC=/usr/local/src/lnpnode
ENV APP_DIR=/srv/app
ENV USER=lnpnode
ENV CONF=${APP_DIR}/config.toml

RUN adduser --home ${APP_DIR} --shell /bin/bash --disabled-login \
        --gecos "${USER} user" ${USER}

COPY --from=builder --chown=${USER}:${USER} ${SRC}/bin/* /usr/local/bin/

USER ${USER}

WORKDIR ${APP_DIR}

RUN mkdir ${APP_DIR}/.lnp_node

EXPOSE 9666 9735

