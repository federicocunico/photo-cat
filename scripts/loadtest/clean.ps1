<#
.SYNOPSIS
  Cancella la cartella del test di carico (solo <Target>, mai la sorgente delle foto).
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\loadtest\clean.ps1
#>
[CmdletBinding()]
param(
    [string]$Target = 'D:\photo-cat-loadtest',
    [string]$Source = 'D:\Foto',
    # Conserva la cartella dati (catalogo e log della prova).
    [switch]$KeepData
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

$Target = Resolve-Full $Target
Assert-Outside -Path $Target -Protected $Source
if ((Split-Path $Target -Leaf) -ne 'photo-cat-loadtest') {
    throw "Rifiutato: la pulizia agisce solo su una cartella chiamata 'photo-cat-loadtest' ($Target)"
}
if (-not (Test-Path -LiteralPath $Target)) { Write-Host "Niente da cancellare: $Target"; return }

$before = Get-Fingerprint (Resolve-Full $Source)
$what = if ($KeepData) { @(Join-Path $Target 'library') } else { @($Target) }
foreach ($p in $what) {
    if (Test-Path -LiteralPath $p) {
        Write-Host "Cancello $p"
        # Cancellare un collegamento fisico rimuove solo il nome nella cartella di prova:
        # il file originale nella sorgente resta.
        Remove-Item -LiteralPath $p -Recurse -Force
    }
}
$after = Get-Fingerprint (Resolve-Full $Source)
if ($after.Count -ne $before.Count -or $after.Hash -ne $before.Hash) {
    throw "La sorgente $Source risulta cambiata dopo la pulizia"
}
Write-Host ("Sorgente invariata: {0} file" -f $after.Count)
