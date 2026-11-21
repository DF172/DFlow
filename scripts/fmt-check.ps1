# Per-file rustfmt gate for DraftFlow CAD.
# `cargo fmt` crashes on this tree (0xC0000409) even though single-file
# `rustfmt` works, so the gate drives rustfmt one file at a time and
# deliberately skips vendor/.
param([switch]$Fix)

$ErrorActionPreference = 'Continue'
$files = @()
foreach ($root in @('src', 'benches', 'tests', 'build.rs')) {
    if (Test-Path $root) {
        if ((Get-Item $root).PSIsContainer) {
            $files += Get-ChildItem $root -Recurse -Filter *.rs
        } else {
            $files += Get-Item $root
        }
    }
}
foreach ($crate in @('crates\draftflow_plugin_api', 'crates\draftflow_save', 'crates\dwg-thumbnailer', 'crates\dwg-thumbnailer-win')) {
    if (Test-Path $crate) {
        $files += Get-ChildItem $crate -Recurse -Filter *.rs | Where-Object { $_.FullName -notmatch '\\target\\' }
    }
}

$fail = @()
$unstable = @()
foreach ($f in $files) {
    if ($Fix) {
        rustfmt --edition 2021 $f.FullName
        if ($LASTEXITCODE -ne 0) { Write-Host ("rustfmt failed: {0}" -f $f.FullName); exit 2 }
    } else {
        rustfmt --edition 2021 --check $f.FullName 2> $null > $null
        if ($LASTEXITCODE -eq 1) {
            $fail += $f.FullName
        } elseif ($LASTEXITCODE -ne 0) {
            # rustfmt occasionally aborts under memory pressure; retry once and
            # record the file instead of silently skipping it.
            rustfmt --edition 2021 --check $f.FullName 2> $null > $null
            if ($LASTEXITCODE -eq 1) {
                $fail += $f.FullName
            } elseif ($LASTEXITCODE -ne 0) {
                $unstable += ("{0} (exit {1})" -f $f.FullName, $LASTEXITCODE)
            }
        }
    }
}

if ($Fix) {
    Write-Host ("fmt: formatted {0} file(s)" -f $files.Count)
    exit 0
}
if ($fail.Count -gt 0) {
    Write-Host ("fmt: {0} of {1} file(s) need formatting" -f $fail.Count, $files.Count)
    $fail | Select-Object -First 20 | ForEach-Object { Write-Host ("  " + $_) }
    if ($unstable.Count -gt 0) {
        Write-Host ("fmt: {0} file(s) could not be checked (rustfmt unstable)" -f $unstable.Count)
        $unstable | ForEach-Object { Write-Host ("  " + $_) }
    }
    exit 1
}
if ($unstable.Count -gt 0) {
    Write-Host ("fmt: {0} file(s) could not be checked (rustfmt unstable)" -f $unstable.Count)
    $unstable | ForEach-Object { Write-Host ("  " + $_) }
    exit 3
}
Write-Host ("fmt: clean ({0} files)" -f $files.Count)
exit 0
