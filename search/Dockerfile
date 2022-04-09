FROM rustlang/rust:nightly-buster-slim as builder

WORKDIR /usr/src/satt
COPY  . .

RUN cargo build --release

FROM debian:buster-slim

RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install wget -y

WORKDIR /search

COPY --from=builder /usr/src/satt/target/release/search-at-the-table .
COPY static /search/static
RUN chown 1000:1000 static

USER 1000
CMD ["./search-at-the-table"]