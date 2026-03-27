###
### Build stage
###

FROM rust:1.94.0-alpine AS build

WORKDIR /app
COPY . /app
RUN apk update && apk add git && cargo build --release --locked

##
## Runner
##

FROM alpine:3.23.3 AS runner

RUN apk update && apk add tini
WORKDIR /app
COPY --from=build /app/target/release/pwmp-server /app/pwmp-server
RUN mkdir /app/data

VOLUME ["/app/data"]
EXPOSE 55300
STOPSIGNAL SIGINT

ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/app/pwmp-server", "--config", "/app/data/config.yml"]