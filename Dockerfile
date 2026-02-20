# Stage 1: Build Frontend (WASM)
FROM rust:1.80 as frontend-builder
WORKDIR /app
RUN cargo install --locked trunk
RUN cargo install --locked wasm-bindgen-cli
RUN rustup target add wasm32-unknown-unknown

COPY . .
# Build the WASM output to /app/dist
RUN trunk build --release

# Stage 2: Build Backend (Server)
FROM rust:1.80 as backend-builder
WORKDIR /app
COPY . .
# Build the server binary
RUN cargo build --release --bin server

# Stage 3: Runtime (Small Image)
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy the server binary
COPY --from=backend-builder /app/target/release/server /app/server
# Copy the WASM static files
COPY --from=frontend-builder /app/dist /app/dist

# Expose port
ENV PORT=8033
EXPOSE 8033

CMD ["./server"]
