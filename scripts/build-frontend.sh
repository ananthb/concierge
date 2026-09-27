#!/usr/bin/env bash
# Compile the Elm app to public/app.js.
#
# Runs in two environments and must behave the same in both:
#   - the nix devShell, which has `elm` from nixpkgs on PATH
#   - Cloudflare Workers Builds, which has node but no elm, so the `elm`
#     npm package (a thin wrapper over the same binary) is used instead
#
# Both are elm 0.19.2. A mismatch would be a hard failure rather than a
# subtle one: elm refuses to build against an elm.json pinned to another
# version, which is the behaviour we want.
set -euo pipefail

cd "$(dirname "$0")/.."

if command -v elm >/dev/null 2>&1; then
    ELM=elm
else
    # `npx --no-install` so a missing devDependency fails loudly instead of
    # silently pulling an arbitrary version off the network mid-build.
    ELM="npx --no-install elm"
fi

# --optimize turns on Elm's dead-code elimination and refuses Debug.* calls,
# which is the check we actually want in CI: a stray Debug.log can't reach
# production. Local iteration uses `dev` below.
MODE="${1:-optimize}"
case "$MODE" in
    optimize) FLAGS=(--optimize) ;;
    dev)      FLAGS=() ;;
    *) echo "usage: $0 [optimize|dev]" >&2; exit 2 ;;
esac

cd frontend
# shellcheck disable=SC2086
$ELM make src/Main.elm "${FLAGS[@]}" --output=../public/app.js

cd ..
echo "built public/app.js ($(wc -c < public/app.js) bytes)"
