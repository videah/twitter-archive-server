FROM clux/muslrust:stable AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS cacher
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .

FROM chef AS builder
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOM

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM zenika/alpine-chrome:latest AS runtime
WORKDIR /app
RUN addgroup -S myuser && adduser -S myuser -G myuser
COPY . .
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/twitter-archive-server .
USER myuser
CMD ["./twitter-archive-server"]