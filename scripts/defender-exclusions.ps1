# Adds DraftFlow CAD's build and cache directories to Windows Defender
# exclusions. Real-time scanning of the tens of thousands of object files that
# `cargo build` writes is a measurable drag on compile time.
#
# Requires an elevated PowerShell:
#   Start-Process powershell -Verb RunAs -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','D:\app\DraftFlowCAD\source\scripts\defender-exclusions.ps1'
#
# Safe to re-run: Add-MpPreference is idempotent for the same path.

$paths = @(
    'D:\cargo-target',          # terminal builds (CARGO_TARGET_DIR)
    'D:\cargo-target-ra',       # rust-analyzer builds
    'D:\cargo-target-install',  # cargo install scratch
    'D:\sccache',               # sccache object cache
    'D:\.cargo',                # crate registry + git checkouts
    'D:\.rustup',               # toolchains
    'D:\app\DraftFlowCAD\source'
)

if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "Run this script from an elevated PowerShell (Administrator)."
    exit 1
}

foreach ($p in $paths) {
    if (Test-Path -LiteralPath $p) {
        Add-MpPreference -ExclusionPath $p -ErrorAction Stop
        Write-Host "excluded: $p"
    } else {
        Write-Host "skipped (missing): $p"
    }
}

Write-Host ""
Write-Host "Current exclusions:"
Get-MpPreference | Select-Object -ExpandProperty ExclusionPath
