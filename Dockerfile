FROM rust:alpine

# create a new empty shell project
RUN USER=root cargo new app
WORKDIR /app

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build # --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm -f ./target/debug/deps/app*
RUN cargo build # --release

# set the startup command to run your binary
CMD ["./target/release/currencies"]
