# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.86.0
ARG APP_NAME=pwmp-server
FROM rust:${RUST_VERSION}-slim-bullseye AS build
ARG APP_NAME
WORKDIR /app

# Enable offline SQLx checks
ENV SQLX_OFFLINE=true

# Install the UPX binary compressor
RUN apt update -y && apt install -y upx

# Build the application.
# Leverage a cache mount to /usr/local/cargo/registry/
# for downloaded dependencies and a cache mount to /app/target/ for
# compiled dependencies which will speed up subsequent builds.
# Leverage a bind mount to the src directory to avoid having to copy the
# source code into the container. Once built, copy the executable to an
# output directory before the cache mounted /app/target is unmounted.
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    --mount=type=bind,source=migrations,target=migrations \
    --mount=type=bind,source=queries,target=queries \
    --mount=type=bind,source=.sqlx,target=.sqlx \
    <<EOF
set -e
cargo build --locked --release
upx --best --lzma ./target/release/$APP_NAME
cp ./target/release/$APP_NAME /bin/server
EOF

################################################################################
# Create a new stage for running the application that contains the minimal
# runtime dependencies for the application. This often uses a different base
# image from the build stage where the necessary files are copied from the build
# stage.
#
FROM debian:bullseye-slim AS final

VOLUME ["/config"]

# Copy the executable from the "build" stage.
COPY --from=build /bin/server /bin/

# Expose the port that the application listens on.
EXPOSE 55300

ENTRYPOINT ["/bin/server", "--config", "/config/config.yml"]