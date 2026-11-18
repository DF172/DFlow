# DraftFlow CAD — daily build recipes.
#
# Profiles:
#   dev           opt-level 0, deps at 2   — fastest inner loop
#   fast-release  opt 2, no LTO, cgu 16    — run the real UI / hunt bugs
#   release       opt 3, fat LTO, cgu 1    — shipping build (slow on purpose)
#
# Run `just` alone to list the recipes.

set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

# Type-check the whole workspace without producing binaries.
check:
    cargo check --workspace --all-targets

# Full test suite (nextest for the 2 500 lib tests, cargo for doctests).
test:
    cargo nextest run --workspace
    cargo test --doc

# Run the real app with fast-release — the everyday "does it look right" loop.
run:
    cargo run --profile fast-release

# Shipping build: fat LTO, single codegen unit, stripped.
ship:
    cargo build --release
    Write-Host ""
    Write-Host "exe: D:\cargo-target\x86_64-pc-windows-msvc\release\DraftFlowCAD.exe"

# Delete incremental leftovers older than seven days (cargo-sweep).
sweep:
    cargo sweep --time 7
