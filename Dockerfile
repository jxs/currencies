ARG VERSION=1.48

FROM rust:$VERSION as planner
WORKDIR app
# We only pay the installation cost once,
# it will be cached from the second build onwards
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:$VERSION as cacher
WORKDIR app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:$VERSION as builder
WORKDIR app
COPY . .
# Copy over the cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --release

FROM registry.fedoraproject.org/fedora-minimal as runtime
WORKDIR /srv/currencies
COPY --from=builder /app/target/release/currencies /usr/local/bin

ENV PORT=3030
ENV RUST_LOG=info
ENV DB_LOCATION=db
CMD ["/usr/local/bin/currencies"]

