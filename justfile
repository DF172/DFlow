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
# sccache is bypassed for the same reason as `ship`: Git Bash gives us an
# empty-but-present RUSTC_WRAPPER (PowerShell would delete the variable and
# cargo would fall back to the config-file wrapper).
check: preflight
    D:\app\Git\bin\bash.exe -c "RUSTC_WRAPPER= CARGO_BUILD_JOBS=1 cargo check --workspace --all-targets"

# Full test suite (nextest for the 2 500 lib tests, cargo for doctests).
# Jobs are capped at 1 on this 16 GB box: the main crate's lib and lib-test
# units otherwise compile together and exhaust the commit limit.
test: preflight
    D:\app\Git\bin\bash.exe -c "RUSTC_WRAPPER= CARGO_PROFILE_DEV_CODEGEN_UNITS=256 CARGO_PROFILE_TEST_CODEGEN_UNITS=256 CARGO_BUILD_JOBS=1 cargo nextest run --workspace"
    D:\app\Git\bin\bash.exe -c "RUSTC_WRAPPER= CARGO_PROFILE_DEV_CODEGEN_UNITS=256 CARGO_PROFILE_TEST_CODEGEN_UNITS=256 CARGO_BUILD_JOBS=1 cargo test --doc"

# Run the real app with fast-release — the everyday "does it look right" loop.
# Always preflights the tool chain, keeps backtraces on, and tees the full log to D:\Temp.
run: preflight
    D:\app\Git\bin\bash.exe -c 'RUSTC_WRAPPER= RUST_BACKTRACE=1 RUST_LOG=${RUST_LOG:-info} cargo run --profile fast-release 2>&1 | tee /d/Temp/draftflow-run.log; exit ${PIPESTATUS[0]}'

# Shipping build: fat LTO, single codegen unit, stripped.
# sccache is bypassed on purpose: its rustc spawn exceeds the Windows
# 32 767-char command line limit for this crate (os error 206, 2026-11-19).
# The wrapper is disabled with an empty-but-present RUSTC_WRAPPER, set in
# bash: PowerShell's `$env:RUSTC_WRAPPER=''` deletes the variable (cargo then
# falls back to the config-file sccache), and `--config build.rustc-wrapper=''`
# produces a different cargo fingerprint than the env form, forcing a rebuild.
ship: preflight
    D:\app\Git\bin\bash.exe -c "RUSTC_WRAPPER= cargo build --release"
    Write-Host ""
    Write-Host "exe: D:\cargo-target\x86_64-pc-windows-msvc\release\DraftFlowCAD.exe"

# Delete incremental leftovers older than seven days (cargo-sweep).
sweep:
    cargo sweep --time 7

# Verify every companion tool before build/run/test. Required tools fail; optional report WARN.
preflight:
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\preflight.ps1

# Per-file rustfmt gate (vendor excluded; `cargo fmt` crashes on this tree).
# Pass `-Fix` to the script directly to format in place.
fmt-check:
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\fmt-check.ps1

# Performance gate: run the 2 000-entity scroll probe alone (20 ms p95 threshold, never relaxed).
perf: preflight
    D:\app\Git\bin\bash.exe -c 'RUSTC_WRAPPER= CARGO_PROFILE_TEST_CODEGEN_UNITS=256 CARGO_BUILD_JOBS=1 cargo nextest run -E "test(fps_probe_2000_scroll)"'
