#!/usr/bin/env bash
# wrangler [build].command — wraps `worker-build --release` with a
# rustup + cargo bootstrap for environments that don't have a Rust
# toolchain on PATH (e.g. Cloudflare Workers Builds, which runs
# `npx wrangler versions upload` directly without going through the
# `npm run deploy` script that does the same bootstrap inline).
#
# Local dev hits the short-circuit at the top of this script because
# the nix devShell already provides `worker-build`; CI flows through
# the full path on a cold cache (~30–60s once, then the cargo cache
# keeps it instant).
set -euo pipefail

if ! command -v worker-build >/dev/null 2>&1; then
    if ! command -v cargo >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
            | sh -s -- -y --profile minimal --default-toolchain none
    fi
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
    # Must track the `worker` major in Cargo.toml. worker-build
    # refuses an older pin outright: "Unsupported version
    # worker@0.7.4, expected at least worker@0.8.6". The nix
    # devShell pins the same pair through flake.lock.
    cargo install -q worker-build --version ^0.8
fi

exec worker-build --release
