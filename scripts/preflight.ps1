# DraftFlow CAD tool-chain preflight.
# One line per companion tool. Required tools fail the recipe; optional tools warn.
$ErrorActionPreference = 'Continue'
$script:missing = @()

function Probe([string]$name, [bool]$required, [scriptblock]$check) {
    $detail = $null
    try { $detail = & $check } catch { $detail = $null }
    if ($detail) {
        Write-Host ("[OK]   {0,-14} {1}" -f $name, $detail)
    } elseif ($required) {
        $script:missing += $name
        Write-Host ("[MISS] {0,-14} required" -f $name)
    } else {
        Write-Host ("[WARN] {0,-14} optional, not found" -f $name)
    }
}

Probe 'rustc' $true { (rustc --version) }
Probe 'cargo' $true { (cargo --version) }
Probe 'nextest' $true { (cargo nextest --version 2>&1 | Select-Object -First 1) }
Probe 'sccache' $true { (sccache --version 2>&1 | Select-Object -First 1) }
Probe 'just' $true { (just --version) }
Probe 'rust-src' $true { if ((rustup component list --installed) -match 'rust-src') { 'installed' } }
Probe 'llvm-tools' $true { if ((rustup component list --installed) -match 'llvm-tools') { 'installed' } }
Probe 'python' $false { (python --version 2>&1 | Select-Object -First 1) }
Probe 'gh' $false { (gh --version 2>&1 | Select-Object -First 1) }
Probe 'rust-analyzer' $false {
    $d = Get-ChildItem (Join-Path $env:USERPROFILE '.trae-cn\extensions') -Directory -Filter 'rust-lang.rust-analyzer*' -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($d) { $exe = Join-Path $d.FullName 'server\rust-analyzer.exe'; if (Test-Path $exe) { (& $exe --version 2>&1 | Select-Object -First 1) } }
}
Probe 'codelldb' $false {
    $d = Get-ChildItem (Join-Path $env:USERPROFILE '.trae-cn\extensions') -Directory -Filter 'vadimcn.vscode-lldb*' -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($d) { $exe = Join-Path $d.FullName 'lldb\bin\lldb.exe'; if (Test-Path $exe) { (& $exe --version 2>&1 | Select-Object -First 1) } }
}

$gui = Get-Process DraftFlowCAD -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle }
if ($gui) {
    Write-Host ("[INFO] DraftFlowCAD GUI already running (pid {0}) - MCP will attach; do not launch a duplicate (wgpu OOM)." -f $gui[0].Id)
} else {
    Write-Host '[INFO] no DraftFlowCAD GUI instance detected'
}

$freeGb = [math]::Round((Get-PSDrive D).Free / 1GB, 1)
if ($freeGb -lt 5) {
    Write-Host ("[WARN] D: free space {0} GB (< 5 GB)" -f $freeGb)
} else {
    Write-Host ("[OK]   D: free space {0} GB" -f $freeGb)
}

if ($script:missing.Count -gt 0) {
    Write-Host ("preflight FAILED: missing {0}" -f ($script:missing -join ', '))
    exit 1
}
Write-Host 'preflight OK: all required tools present'
exit 0
