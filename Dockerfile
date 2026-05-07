# ---- Stage 1: Build mp4decrypt from Bento4 ----
FROM debian:bookworm-slim AS bento4-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    git cmake g++ make python3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN git clone --depth 1 https://github.com/axiomatic-systems/Bento4.git /bento4

WORKDIR /bento4
RUN mkdir build && cd build \
    && cmake -DCMAKE_BUILD_TYPE=Release .. \
    && make -j$(nproc) mp4decrypt

# ---- Stage 2: Build Rust API ----
# 1.91+ required by aws-smithy-types and friends pulled via aws-sdk-s3.
FROM rust:1-bookworm AS api-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY crunchy-cli/Cargo.toml crunchy-cli/Cargo.lock ./
COPY crunchy-cli/crates ./crates
COPY crunchy-cli/src ./src

RUN cargo build --release -p crunchy-api

# ---- Stage 3: Install Next.js dependencies ----
FROM oven/bun:1 AS web-deps

WORKDIR /app
COPY crunchy-web/package.json crunchy-web/bun.lock ./
RUN bun install --frozen-lockfile

# ---- Stage 4: Build Next.js app ----
FROM oven/bun:1 AS web-builder

WORKDIR /app
COPY --from=web-deps /app/node_modules ./node_modules
COPY crunchy-web/ .

ENV NEXT_PUBLIC_API_URL=http://localhost:8080

RUN bun run build

# ---- Stage 5: Final runtime image ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    ca-certificates \
    curl \
    && curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user and directories
RUN useradd -m -s /bin/bash crunchy \
    && mkdir -p /home/crunchy/.config/crunchy-cli /downloads /widevine /data /app \
    && chown -R crunchy:crunchy /home/crunchy /downloads /widevine /data /app

# Copy API binary and mp4decrypt.
# mp4decrypt is from Bento4 (https://github.com/axiomatic-systems/Bento4),
# distributed under GPL v2. Source available at the URL above; full notice
# in /NOTICES.md.
COPY --from=bento4-builder /bento4/build/mp4decrypt /usr/local/bin/mp4decrypt
COPY --from=api-builder /build/target/release/crunchy-api /usr/local/bin/crunchy-api

# Bundle NOTICES so the image carries third-party license info.
COPY --chown=crunchy:crunchy NOTICES.md /NOTICES.md

# Copy Next.js standalone app
COPY --from=web-builder --chown=crunchy:crunchy /app/public /app/public
COPY --from=web-builder --chown=crunchy:crunchy /app/.next/standalone /app/
COPY --from=web-builder --chown=crunchy:crunchy /app/.next/static /app/.next/static

# Copy entrypoint
COPY --chown=crunchy:crunchy entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

USER crunchy
WORKDIR /app

EXPOSE 8080 3000

ENTRYPOINT ["/app/entrypoint.sh"]
