FROM rust:1.76-buster
# FROM rust:1.76-alpine3.18

RUN apt-get update -yqq && apt-get install -yqq cmake g++

COPY ./ /app
WORKDIR /app

RUN cargo clean
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release

EXPOSE 8080

ENTRYPOINT [ "/app/target/release/rinhaback2401" ]
