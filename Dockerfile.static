FROM rust:1.76-alpine3.19 as build

WORKDIR /app
RUN apk add musl-dev
RUN mkdir src && echo 'fn main() { println!("Build failed"); std::process::exit(1); }' > src/main.rs
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
COPY . .
RUN touch src/main.rs && cargo build --release

FROM scratch
WORKDIR /app
EXPOSE 9999
COPY --from=build /app/target/release/rinhaback2401 /
ENTRYPOINT [ "/rinhaback2401" ]
