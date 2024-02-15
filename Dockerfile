FROM rust:1.76-bookworm as build

WORKDIR /app
RUN apt-get update -yqq && apt-get install -yqq cmake g++
RUN mkdir src && echo 'fn main() { println!("Build failed"); std::process::exit(1); }' > src/main.rs
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
COPY . .
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
EXPOSE 9999
COPY --from=build /app/target/release/rinhaback2401 /app
ENTRYPOINT [ "/app/rinhaback2401" ]
