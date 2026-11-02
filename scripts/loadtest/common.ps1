# Funzioni condivise dagli script di carico (dot-source: . .\common.ps1).

$ImageExtensions = @(
    'jpg', 'jpeg', 'png', 'tif', 'tiff', 'webp', 'bmp', 'gif', 'heic', 'heif', 'avif',
    'dng', 'cr2', 'cr3', 'nef', 'arw', 'orf', 'rw2', 'raf', 'pef', 'srw'
)

function Resolve-Full([string]$Path) {
    [IO.Path]::GetFullPath($Path).TrimEnd('\')
}

# Fallisce se $Path coincide con $Protected o è al suo interno (o lo contiene).
function Assert-Outside([string]$Path, [string]$Protected) {
    $p = (Resolve-Full $Path).ToLowerInvariant() + '\'
    $q = (Resolve-Full $Protected).ToLowerInvariant() + '\'
    if ($p.StartsWith($q) -or $q.StartsWith($p)) {
        throw "Rifiutato: '$Path' si sovrappone alla cartella protetta '$Protected'"
    }
}

# Impronta in sola lettura: numero di file e hash di percorso, dimensione e data di modifica.
function Get-Fingerprint([string]$Root) {
    $sha = [Security.Cryptography.SHA256]::Create()
    $n = 0
    $files = Get-ChildItem -LiteralPath $Root -Recurse -File -Force -ErrorAction SilentlyContinue | Sort-Object FullName
    $sb = New-Object Text.StringBuilder
    foreach ($f in $files) {
        [void]$sb.Append($f.FullName.Substring($Root.Length)).Append('|').Append($f.Length).Append('|').Append($f.LastWriteTimeUtc.Ticks).Append("`n")
        $n++
    }
    $bytes = [Text.Encoding]::UTF8.GetBytes($sb.ToString())
    $hash = ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') }) -join ''
    [pscustomobject]@{ Count = $n; Hash = $hash.Substring(0, 16) }
}
