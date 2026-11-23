<#
.SYNOPSIS
  Orchestrazione della prova di carico: monitoraggio CSV + import headless (non-RAW).

.DESCRIPTION
  Avvia monitor.ps1, poi import_proof sulla libreria di prova senza RAW, con
  PHOTO_CAT_DATA_DIR separata. Non tocca mai D:\Foto. Al termine verifica l'impronta
  della sorgente. Profili: balanced | light.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\loadtest\run-proof.ps1 -Count 5000
  powershell -ExecutionPolicy Bypass -File scripts\loadtest\run-proof.ps1 -Count 20000 -Profile light
#>
[CmdletBinding()]
param(
    [string]$Source = 'D:\Foto',
    [string]$Target = 'D:\photo-cat-loadtest',
    [int]$Count = 5000,
    [int]$Variants = 200,
    [ValidateSet('balanced', 'light')]
    [string]$Profile = 'balanced',
    [switch]$SkipMakeLibrary
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
$Source = Resolve-Full $Source
$Target = Resolve-Full $Target
Assert-Outside -Path $Target -Protected $Source

$repo = Resolve-Full (Join-Path $PSScriptRoot '..\..')
$fullLibrary = Join-Path $Target 'library-noraw'
$data = Join-Path $Target ("data-$Profile-$Count-noraw")
$env:PHOTO_CAT_DATA_DIR = $data
$env:CARGO_INCREMENTAL = '0'
$env:RUSTFLAGS = '-C debuginfo=0 -C codegen-units=1'
New-Item -ItemType Directory -Force -Path $data | Out-Null

Write-Host "Impronta di $Source (prima)..."
$before = Get-Fingerprint $Source
Write-Host ("  {0} file, {1}" -f $before.Count, $before.Hash)

if (-not $SkipMakeLibrary) {
    & (Join-Path $PSScriptRoot 'make-library-noraw.ps1') `
        -Source $Source `
        -Target $fullLibrary `
        -Count ([Math]::Max($Count, 100000)) `
        -Variants ([Math]::Max($Variants, 500))
}

# Sottoinsieme per prove scalate: collegamenti fisici ai primi N file della libreria completa.
$library = $fullLibrary
if ($Count -gt 0 -and (Test-Path -LiteralPath $fullLibrary)) {
    $existing = (Get-ChildItem -LiteralPath $fullLibrary -Recurse -File -ErrorAction SilentlyContinue |
        Measure-Object).Count
    if ($existing -gt $Count) {
        $library = Join-Path $Target ("library-noraw-{0}" -f $Count)
        Write-Host "Sottoinsieme $Count file in $library"
        if (-not (Test-Path -LiteralPath $library)) {
            New-Item -ItemType Directory -Force -Path $library | Out-Null
            $n = 0
            foreach ($f in (Get-ChildItem -LiteralPath $fullLibrary -Recurse -File)) {
                if ($n -ge $Count) { break }
                $rel = $f.FullName.Substring($fullLibrary.Length).TrimStart('\')
                $dest = Join-Path $library $rel
                if (-not (Test-Path -LiteralPath $dest)) {
                    New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
                    cmd /c "mklink /H `"$dest`" `"$($f.FullName)`" >nul 2>&1"
                    if ($LASTEXITCODE -ne 0) { throw "mklink /H fallito: $dest" }
                }
                $n++
            }
            Write-Host "  creati $n collegamenti"
        }
    }
}

$csv = Join-Path $data ('memory-{0:yyyyMMdd-HHmmss}.csv' -f (Get-Date))
$outLog = Join-Path $data 'import-proof.out.txt'
$monitor = Start-Process -PassThru -WindowStyle Hidden -FilePath powershell.exe -ArgumentList @(
    '-ExecutionPolicy', 'Bypass',
    '-File', (Join-Path $PSScriptRoot 'monitor.ps1'),
    '-DataDir', $data,
    '-Out', $csv,
    '-ProcessName', 'import_proof'
)
Write-Host "Monitor PID $($monitor.Id) → $csv"

try {
    Push-Location $repo
    $profileArg = if ($Profile -eq 'light') { 'light' } else { 'balanced' }
    # Cargo scrive su stderr: con $ErrorActionPreference=Stop un redirect 2>&1 può
    # trasformare le righe in errori terminanti. *> su file evita il problema.
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & cargo run --release -p pc-ingest --features heif --example import_proof -- `
            $library $data --profile $profileArg *> $outLog
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prevEap
    }
    if (Test-Path -LiteralPath $outLog) {
        Get-Content -LiteralPath $outLog | Select-Object -Last 30 | ForEach-Object { Write-Host $_ }
    }
    if ($code -ne 0) { throw "import_proof exit $code (log: $outLog)" }
} finally {
    Pop-Location
    if (-not $monitor.HasExited) {
        Stop-Process -Id $monitor.Id -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "Impronta di $Source (dopo)..."
$after = Get-Fingerprint $Source
if ($after.Count -ne $before.Count -or $after.Hash -ne $before.Hash) {
    throw ("$Source CAMBIATA: prima {0}/{1}, dopo {2}/{3}" -f $before.Count, $before.Hash, $after.Count, $after.Hash)
}
Write-Host ("  invariata: {0} file" -f $after.Count)
Write-Host "CSV: $csv"
if (Test-Path $csv) {
    Get-Content $csv | Select-Object -First 5
    Write-Host '...'
    Get-Content $csv | Select-Object -Last 3
}
