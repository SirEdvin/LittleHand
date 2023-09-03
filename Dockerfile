FROM rust as builder
WORKDIR /usr/src/little_hand
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/little_hand /usr/local/bin/little_hand
CMD ["little_hand"]