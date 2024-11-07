FROM debian:bookworm

ENV TZ="Europe/Berlin"
ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update 
RUN apt-get install -y locales curl libssl-dev uuid sqlite3 build-essential pkgconf  \
    && localedef -i en_US -c -f UTF-8 -A /usr/share/locale/locale.alias en_US.UTF-8

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh \
    && chmod +x rustup.sh \
    && ./rustup.sh --profile minimal -y \
    && rm rustup.sh

RUN . "$HOME/.cargo/env" \
    && cargo install sqlx-cli

ADD entrypoint.sh /bin
RUN chmod +x /bin/entrypoint.sh

ENTRYPOINT [ "/bin/entrypoint.sh" ]


#    && rm -rf /var/lib/apt/lists/* \
