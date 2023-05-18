FROM rust:1.69.0-buster as builder

WORKDIR /usr/src/curiosity
COPY . .

RUN cargo build --release

FROM debian:buster-slim

RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install wget -y

WORKDIR /curiosity
RUN chown -R 1000:1000 /curiosity


USER 1000

COPY --from=builder /usr/src/curiosity/target/release/curiosity .
COPY static static

RUN mkdir satt_idx

CMD ["./curiosity"]