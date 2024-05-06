# syntax=docker/dockerfile:1.2

ARG RUST_VERSION=1.77.1
FROM rust:${RUST_VERSION}-alpine AS build

RUN apk add --no-cache build-base clang lld musl-dev git protobuf-dev

# Setting up the working directory in the Docker container
WORKDIR /usr/src/railyard

# Copy the entire project directory into the Docker container
COPY . .

# Navigate to the specific project (server example)
WORKDIR /usr/src/railyard/examples/server

# Build the project
RUN cargo build --release

# Note the executable's name must match what is specified in your Cargo.toml
ARG APP_NAME=railyard-server
RUN cp target/release/$APP_NAME /usr/local/bin/

# Start a new stage to keep the final image clean and small
FROM alpine:3.18 AS final
RUN apk add --no-cache libgcc

COPY --from=build /usr/local/bin/railyard-server /usr/local/bin/railyard-server

# Non-privileged user setup
RUN adduser -D -g '' appuser
USER appuser

EXPOSE 8000
CMD ["railyard-server"]
