FROM rust:1.76-bullseye
LABEL authors="yangu"
COPY . .
RUN cargo build --release

ENTRYPOINT ["./target/release/pipeline", "--help"]