FROM rust:1.93-bookworm@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31
ENV CARGO_BUILD_JOBS=2
ENV container=docker
RUN apt-get update && apt-get install -y --no-install-recommends openssl openssh-server openssh-client systemd dbus libpam-systemd dbus-user-session \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add clippy rustfmt
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --locked
COPY . .
RUN cargo test --locked --all-targets --no-run
CMD ["cargo", "test", "--locked", "--test", "keycloak", "--", "--ignored", "--nocapture"]
