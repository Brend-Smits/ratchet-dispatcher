# Stage 1: Build Ratchet from the latest tag
FROM golang:1.22 AS ratchet-builder
WORKDIR /go/src/github.com/sethvargo/ratchet
RUN apt-get update && apt-get install -y git && apt clean

# Fetch the latest tag and checkout
RUN git clone https://github.com/sethvargo/ratchet . \
    && latest_tag=$(git describe --tags `git rev-list --tags --max-count=1`) \
    && git checkout "$latest_tag"

RUN go get -d -v ./...
RUN go build -ldflags "-s -w" -o /go/bin/ratchet ./

# Stage 2: Rust dependencies caching
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build Rust application
FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin ratchet-dispatcher

# Stage 4: Create Final Image
FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/ratchet-dispatcher /usr/local/bin
COPY --from=ratchet-builder /go/bin/ratchet /usr/local/bin
ENTRYPOINT ["/usr/local/bin/ratchet-dispatcher"]
