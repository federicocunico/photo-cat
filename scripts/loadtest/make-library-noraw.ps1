<#
.SYNOPSIS
  Libreria di carico senza RAW (raf/dng/nef/…): solo formati a decodifica rapida.

.DESCRIPTION
  Crea collegamenti fisici e poche varianti JPEG sotto <Target> (es.
  D:\photo-cat-loadtest\library-noraw). Non scrive mai dentro <Source>.
#>
[CmdletBinding()]
param(
    [string]$Source = 'D:\Foto',
    [string]$Target = 'D:\photo-cat-loadtest\library-noraw',
    [int]$Count = 100000,
    [int]$Variants = 500
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

$Source = Resolve-Full $Source
$Target = Resolve-Full $Target
Assert-Outside -Path $Target -Protected $Source
if (-not (Test-Path -LiteralPath $Source -PathType Container)) { throw "Sorgente non trovata: $Source" }
if ([IO.Path]::GetPathRoot($Source) -ne [IO.Path]::GetPathRoot($Target)) {
    throw "I collegamenti fisici richiedono lo stesso volume: $Source e $Target"
}

$OkExt = @(
    '.jpg', '.jpeg', '.png', '.webp', '.heic', '.heif',
    '.gif', '.tif', '.tiff', '.bmp'
)

Write-Host "Impronta di $Source (sola lettura)..."
$before = Get-Fingerprint $Source
Write-Host ("  {0} file, impronta {1}" -f $before.Count, $before.Hash)

$images = Get-ChildItem -LiteralPath $Source -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $OkExt -contains $_.Extension.ToLowerInvariant() } |
    Sort-Object FullName
if ($images.Count -eq 0) { throw "Nessuna immagine non-RAW in $Source" }
Write-Host ("  {0} immagini non-RAW" -f $images.Count)

New-Item -ItemType Directory -Force -Path $Target | Out-Null
$linksWanted = [Math]::Max(0, $Count - $Variants)
$made = 0
$copy = 0
while ($made -lt $linksWanted) {
    $copyDir = Join-Path $Target ('copy-{0:D2}' -f $copy)
    foreach ($img in $images) {
        if ($made -ge $linksWanted) { break }
        $rel = $img.FullName.Substring($Source.Length).TrimStart('\')
        $dest = Join-Path $copyDir $rel
        if (-not (Test-Path -LiteralPath $dest)) {
            New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
            # Redirect via cmd so stderr non diventa errore terminante sotto Stop.
            cmd /c "mklink /H `"$dest`" `"$($img.FullName)`" >nul 2>&1"
            if ($LASTEXITCODE -ne 0) { throw "mklink /H fallito: $dest" }
        }
        $made++
        if ($made % 10000 -eq 0) { Write-Host "  collegamenti: $made / $linksWanted" }
    }
    $copy++
}

Add-Type -AssemblyName System.Drawing
$vdir = Join-Path $Target 'variants'
New-Item -ItemType Directory -Force -Path $vdir | Out-Null
$v = 0
$jpgOnly = $images | Where-Object { $_.Extension -match '(?i)\.jpe?g$' }
foreach ($img in $jpgOnly) {
    if ($v -ge $Variants) { break }
    $dest = Join-Path $vdir ("v-{0:D4}.jpg" -f $v)
    if (Test-Path -LiteralPath $dest) { $v++; continue }
    $bmp = $null
    $crop = $null
    $g = $null
    try {
        $bmp = [System.Drawing.Bitmap]::FromFile($img.FullName)
        $w = [Math]::Max(1, [int]($bmp.Width * 0.9))
        $h = [Math]::Max(1, [int]($bmp.Height * 0.9))
        $x = [int](($bmp.Width - $w) / 2)
        $y = [int](($bmp.Height - $h) / 2)
        $crop = New-Object System.Drawing.Bitmap $w, $h
        $g = [System.Drawing.Graphics]::FromImage($crop)
        $g.DrawImage(
            $bmp,
            (New-Object System.Drawing.Rectangle 0, 0, $w, $h),
            (New-Object System.Drawing.Rectangle $x, $y, $w, $h),
            [System.Drawing.GraphicsUnit]::Pixel
        )
        $crop.Save($dest, [System.Drawing.Imaging.ImageFormat]::Jpeg)
        $v++
    } catch {
        # salta file non ritagliabili
    } finally {
        if ($g) { $g.Dispose() }
        if ($crop) { $crop.Dispose() }
        if ($bmp) { $bmp.Dispose() }
    }
}

$after = Get-Fingerprint $Source
if ($after.Count -ne $before.Count -or $after.Hash -ne $before.Hash) {
    throw ("$Source CAMBIATA durante la creazione della libreria")
}
$total = (Get-ChildItem -LiteralPath $Target -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count
Write-Host ("library-noraw: {0} file (link={1}, varianti={2}); sorgente invariata" -f $total, $made, $v)
