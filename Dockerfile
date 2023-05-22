FROM rust:1.69.0-buster as builder

WORKDIR /usr/src/curiosity
COPY curiosity curiosity
COPY Cargo.toml . 
COPY Cargo.lock .

RUN mkdir server
COPY server/src server/src
COPY server/Cargo.toml server/Cargo.toml

RUN cargo build --profile production

FROM debian:buster-slim

RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install wget -y

WORKDIR /curiosity
RUN chown -R 1000:1000 /curiosity

USER 1000

COPY --from=builder /usr/src/curiosity/target/production/server .
COPY server/static static

CMD ["./server"]