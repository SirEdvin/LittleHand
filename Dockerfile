FROM rust as builder
WORKDIR /usr/src/little_hand
ADD "src/" "src/"
ADD "Cargo.toml" "Cargo.toml"
RUN cargo install --path .

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/little_hand /usr/local/bin/little_hand
CMD ["little_hand"]