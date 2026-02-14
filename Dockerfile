# ── Stage 1: Build ────────────────────────────────────────────
FROM rust:1.83-slim AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release --locked && \
    strip target/release/zeroclaw

# ── Stage 2: Runtime (distroless — no shell, no OS, tiny) ────
FROM gcr.io/distroless/cc-debian12

COPY --from=builder /app/target/release/zeroclaw /usr/local/bin/zeroclaw
COPY LICENSE /usr/local/share/licenses/zeroclaw/LICENSE

# Default workspace
VOLUME ["/workspace"]
ENV ZEROCLAW_WORKSPACE=/workspace

ENTRYPOINT ["zeroclaw"]
CMD ["gateway"]
