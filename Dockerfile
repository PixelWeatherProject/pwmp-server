FROM rust:1.94.0-alpine AS build

WORKDIR /app

COPY . /app

RUN apk update
RUN apk add git
RUN cargo build --release

FROM alpine:3.23.3 AS runner

WORKDIR /app

COPY --from=build /app/target/release/pwmp-server /app/pwmp-server

RUN mkdir /app/data

VOLUME ["/app/data"]
EXPOSE 55300

ENTRYPOINT ["/app/pwmp-server", "--config", "/app/data/config.yml"]