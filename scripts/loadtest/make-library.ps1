<#
.SYNOPSIS
  Crea una libreria di prova per il test di carico di photo-cat, con collegamenti fisici
  alle foto di una cartella sorgente aperta solo in lettura.

.DESCRIPTION
  - Crea <Target>\library\copy-NN\... con collegamenti fisici (stesso volume) ai file immagine
    di <Source>, ripetuti finché non si raggiungono <Count> file.
  - Crea <Variants> varianti generate (ritaglio e ricodifica JPEG) in <Target>\library\variants\,
    così non tutto è un duplicato esatto.
  - Prima e dopo confronta numero di file e date di modifica di <Source>: se cambiano, fallisce.
  - Non scrive né cancella MAI nulla dentro <Source>.

  Attenzione: un collegamento fisico è lo stesso file della sorgente. Nessuno (né questo script
  né photo-cat) deve aprire in scrittura i file in <Target>\library: photo-cat li legge soltanto.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\loadtest\make-library.ps1
  powershell -ExecutionPolicy Bypass -File scripts\loadtest\make-library.ps1 -Count 5000 -Variants 200
#>
[CmdletBinding()]
param(
    [string]$Source = 'D:\Foto',
    [string]$Target = 'D:\photo-cat-loadtest',
    [int]$Count = 100000,
    [int]$Variants = 2000,
    # Solo file distinti (nessuna ripetizione copy-NN): secondo scenario del design.
    [switch]$Distinct
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

Write-Host "Impronta di $Source (solo lettura)..."
$before = Get-Fingerprint $Source
Write-Host ("  {0} file, impronta {1}" -f $before.Count, $before.Hash)

$images = Get-ChildItem -LiteralPath $Source -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $ImageExtensions -contains $_.Extension.TrimStart('.').ToLowerInvariant() } |
    Sort-Object FullName
if ($images.Count -eq 0) { throw "Nessuna immagine in $Source" }
Write-Host ("  {0} immagini supportate" -f $images.Count)

$library = Join-Path $Target 'library'
New-Item -ItemType Directory -Force -Path $library | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Target 'data') | Out-Null

$linksWanted = if ($Distinct) { [Math]::Min($Count, $images.Count) } else { [Math]::Max(0, $Count - $Variants) }
$made = 0
$copy = 0
while ($made -lt $linksWanted) {
    $copyDir = Join-Path $library ('copy-{0:D2}' -f $copy)
    foreach ($img in $images) {
        if ($made -ge $linksWanted) { break }
        $rel = $img.FullName.Substring($Source.Length).TrimStart('\')
        $dest = Join-Path $copyDir $rel
        if (-not (Test-Path -LiteralPath $dest)) {
            New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
            # mklink /H: più veloce di New-Item -ItemType HardLink su decine di migliaia di file.
            $out = cmd /c mklink /H "$dest" "$($img.FullName)" 2>&1
            if ($LASTEXITCODE -ne 0) { throw "mklink /H fallito per $dest : $out" }
        }
        $made++
        if ($made % 5000 -eq 0) { Write-Host "  collegamenti: $made / $linksWanted" }
    }
    $copy++
    if ($Distinct) { break }
}

# Varianti: ritaglio del 90% centrale e ricodifica JPEG (solo lettura della sorgente).
if (-not $Distinct -and $Variants -gt 0) {
    Add-Type -AssemblyName System.Drawing
    $variantsDir = Join-Path $library 'variants'
    New-Item -ItemType Directory -Force -Path $variantsDir | Out-Null
    $jpegs = @($images | Where-Object { @('.jpg', '.jpeg') -contains $_.Extension.ToLowerInvariant() })
    $codec = [Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq 'image/jpeg' }
    $params = New-Object Drawing.Imaging.EncoderParameters 1
    $params.Param[0] = New-Object Drawing.Imaging.EncoderParameter ([Drawing.Imaging.Encoder]::Quality, [long]80)
    $done = 0
    for ($i = 0; $done -lt $Variants -and $jpegs.Count -gt 0 -and $i -lt $Variants * 2; $i++) {
        $img = $jpegs[$i % $jpegs.Count]
        $dest = Join-Path $variantsDir ('v{0:D6}.jpg' -f $done)
        if (Test-Path -LiteralPath $dest) { $done++; continue }
        $stream = $null; $bmp = $null; $crop = $null
        try {
            $stream = [IO.File]::Open($img.FullName, 'Open', 'Read', 'Read')
            $bmp = [Drawing.Image]::FromStream($stream, $false, $false)
            $w = [int]($bmp.Width * 0.9); $h = [int]($bmp.Height * 0.9)
            $rect = New-Object Drawing.Rectangle ([int](($bmp.Width - $w) / 2)), ([int](($bmp.Height - $h) / 2)), $w, $h
            $crop = New-Object Drawing.Bitmap $w, $h
            $g = [Drawing.Graphics]::FromImage($crop)
            $g.DrawImage($bmp, (New-Object Drawing.Rectangle 0, 0, $w, $h), $rect, [Drawing.GraphicsUnit]::Pixel)
            $g.Dispose()
            $crop.Save($dest, $codec, $params)
            $done++
            if ($done % 500 -eq 0) { Write-Host "  varianti: $done / $Variants" }
        } catch {
            Write-Warning "variante saltata ($($img.Name)): $($_.Exception.Message)"
        } finally {
            if ($crop) { $crop.Dispose() }
            if ($bmp) { $bmp.Dispose() }
            if ($stream) { $stream.Dispose() }
        }
    }
}

$total = (Get-ChildItem -LiteralPath $library -Recurse -File | Measure-Object).Count
Write-Host "File in $library : $total"

Write-Host "Verifica che $Source sia invariata..."
$after = Get-Fingerprint $Source
if ($after.Count -ne $before.Count -or $after.Hash -ne $before.Hash) {
    throw ("$Source È CAMBIATA: prima {0} file ({1}), dopo {2} file ({3})" -f $before.Count, $before.Hash, $after.Count, $after.Hash)
}
Write-Host ("  invariata: {0} file, impronta {1}" -f $after.Count, $after.Hash)
Write-Host ""
Write-Host "Avvio dell'app sulla libreria di prova (cartella dati separata):"
Write-Host "  `$env:PHOTO_CAT_DATA_DIR = '$(Join-Path $Target 'data')'"
Write-Host "  poi aggiungere la sorgente $library"
