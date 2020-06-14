FROM rust:slim as builder

RUN apt-get update -y \
    && apt-get install -y \
        libsqlite3-dev \
        libssl-dev \
        libzmq3-dev \
        pkg-config

ENV SRC=/usr/local/src/lnpnode

COPY contrib ${SRC}/contrib
COPY doc ${SRC}/doc
COPY src ${SRC}/src
COPY build.rs Cargo.toml config_spec.toml LICENSE README.md ${SRC}/

WORKDIR ${SRC}

RUN rustup default nightly \
    && cargo install --path .


FROM debian:buster-slim

RUN apt-get update -y \
    && apt-get install -y \
        libzmq3-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

COPY --from=builder /usr/local/cargo/bin/lnp-cli /usr/local/bin/
COPY --from=builder /usr/local/cargo/bin/wired /usr/local/bin/

ENV APP_DIR=/srv/app USER=lnpnode
ENV CONF=${APP_DIR}/config.toml

RUN adduser --home ${APP_DIR} --shell /bin/bash --disabled-login \
        --gecos "${USER} user" ${USER}

USER ${USER}

RUN touch ${CONF}

WORKDIR ${APP_DIR}

EXPOSE 9666 9735

ENTRYPOINT ["bash", "-c", "/usr/local/bin/wired -vvvv --config=${CONF}"]
