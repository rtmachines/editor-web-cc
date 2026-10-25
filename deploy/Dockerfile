# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
#
# Image of recipe-server (ADR-0064). The context is the repository root:
#   docker build -f deploy/Dockerfile .
# The file .dockerignore at the root is an allow list of the build inputs.
#
# Every external image goes by digest. The stages chef, planner, and builder share one
# toolchain version, because cargo-chef needs the same Rust version in all stages.
# The runtime image holds the binary only. It has no shell and no package manager.

# --- chef: toolchain, musl C compiler, and cargo-chef ---
# rust:1.99.1-slim-trixie, index digest read with docker buildx imagetools inspect.
FROM rust:1.99.1-slim-trixie@sha256:aab42391a986424e22d6a3ec14f37bc6dd3dacff4d457ca7cccd781e02297cc6 AS chef

# RUSTUP_TOOLCHAIN has a higher precedence than rust-toolchain.toml (channel "stable").
# Only the crate ring (C and assembly) needs musl-gcc.
ENV RUSTUP_TOOLCHAIN=1.99.1 \
    RECIPE_TARGET=x86_64-unknown-linux-musl \
    CC_x86_64_unknown_linux_musl=musl-gcc

RUN apt-get update \
    && apt-get install --no-install-recommends --yes musl-tools \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add "${RECIPE_TARGET}" \
    && cargo install cargo-chef --locked --version 0.1.78

WORKDIR /build

# --- planner: list the dependencies of the workspace ---
FROM chef AS planner

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo chef prepare --recipe-path recipe.json

# --- builder: compile the dependencies, then the workspace ---
FROM chef AS builder

COPY --from=planner /build/recipe.json recipe.json
# This layer changes only when a dependency changes.
RUN cargo chef cook --release --locked --target "${RECIPE_TARGET}" \
    --package recipe-server --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# sqlx::migrate! embeds this directory at compile time.
COPY migrations ./migrations
# The release profile of Cargo.toml keeps overflow-checks and panic = "unwind".
RUN cargo build --release --locked --target "${RECIPE_TARGET}" --package recipe-server

# --- runtime: distroless static, user 65532 ---
# gcr.io/distroless/static-debian13:nonroot, index digest read with docker buildx imagetools inspect.
FROM gcr.io/distroless/static-debian13:nonroot@sha256:d62067879de890a1def9da0e7766b7d20810a5e5555cef8506b752437d7a3e88 AS runtime

LABEL org.opencontainers.image.title="recipe-server" \
      org.opencontainers.image.description="Recipe editor of one production line"

# The binary belongs to root with mode 0755, so the process cannot change its own file.
COPY --from=builder --chown=0:0 --chmod=0755 \
    /build/target/x86_64-unknown-linux-musl/release/recipe-server /usr/local/bin/recipe-server

USER 65532:65532
# http.public_port and http.internal_port (reliability.md section 7.11).
EXPOSE 8080 9000
# No HEALTHCHECK: the runs of migrate and create-admin get none. compose.yaml gives the one of app.
ENTRYPOINT ["/usr/local/bin/recipe-server"]
CMD ["serve"]
