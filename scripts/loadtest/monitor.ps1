<#
.SYNOPSIS
  Registra in un CSV la memoria di photo-cat e dei suoi processi WebView2, più i long task
  dell'interfaccia, durante il test di carico.

.DESCRIPTION
  Ogni <Interval> secondi scrive una riga con:
  - RSS (working set) e memoria privata di photo-cat.exe;
  - RSS e memoria privata sommati dei processi msedgewebview2.exe discendenti di photo-cat.exe;
  - RAM libera del sistema;
  - long task dell'interfaccia (≥ 50 ms) registrati nel log dall'ultima riga, con la durata massima.
    I long task si registrano solo in modalità diagnostica: livello di log "debug" o "trace"
    (Diagnostica › Livello di log).
  Termina da solo quando photo-cat.exe si chiude, oppure con Ctrl+C.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\loadtest\monitor.ps1 -DataDir D:\photo-cat-loadtest\data
#>
[CmdletBinding()]
param(
    [string]$DataDir = 'D:\photo-cat-loadtest\data',
    [string]$Out = '',
    [int]$Interval = 5,
    [string]$ProcessName = 'photo-cat'
)

$ErrorActionPreference = 'Stop'
if (-not $Out) { $Out = Join-Path $DataDir ('memory-{0:yyyyMMdd-HHmmss}.csv' -f (Get-Date)) }
New-Item -ItemType Directory -Force -Path (Split-Path $Out) | Out-Null
'time,app_rss_mb,app_private_mb,webview_rss_mb,webview_private_mb,webview_processes,free_ram_mb,long_tasks,long_task_max_ms' |
    Set-Content -Encoding utf8 $Out
Write-Host "CSV: $Out"

$logs = Join-Path $DataDir 'logs'
$logPos = @{}

# Long task registrati dall'UI (riga di log "long task dell'interfaccia") dopo l'ultima lettura.
function Read-LongTasks {
    $count = 0; $max = 0
    if (-not (Test-Path $logs)) { return @(0, 0) }
    $file = Get-ChildItem -LiteralPath $logs -Filter 'photo-cat*.log' | Sort-Object LastWriteTime | Select-Object -Last 1
    if (-not $file) { return @(0, 0) }
    $pos = if ($logPos.ContainsKey($file.FullName)) { $logPos[$file.FullName] } else { 0 }
    $fs = [IO.File]::Open($file.FullName, 'Open', 'Read', 'ReadWrite')
    try {
        if ($pos -gt $fs.Length) { $pos = 0 }
        [void]$fs.Seek($pos, 'Begin')
        $reader = New-Object IO.StreamReader($fs)
        while (($line = $reader.ReadLine()) -ne $null) {
            if ($line -notlike '*long task dell*') { continue }
            try {
                $f = ($line | ConvertFrom-Json).fields
                $count += [int]$f.count
                if ([double]$f.max_ms -gt $max) { $max = [double]$f.max_ms }
            } catch { }
        }
        $logPos[$file.FullName] = $fs.Position
    } finally { $fs.Dispose() }
    @($count, $max)
}

$seenApp = $false
while ($true) {
    $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name, WorkingSetSize, PrivatePageCount
    $app = @($all | Where-Object { $_.Name -eq "$ProcessName.exe" })
    if ($app.Count -eq 0) {
        if ($seenApp) { Write-Host 'photo-cat chiuso: fine registrazione'; break }
        Start-Sleep -Seconds $Interval
        continue
    }
    $seenApp = $true
    # Discendenti di photo-cat.exe (il processo browser di WebView2 e i suoi figli).
    $children = @{}
    foreach ($p in $all) {
        if (-not $children.ContainsKey($p.ParentProcessId)) { $children[$p.ParentProcessId] = @() }
        $children[$p.ParentProcessId] += $p
    }
    $queue = New-Object Collections.Queue
    $app | ForEach-Object { $queue.Enqueue($_.ProcessId) }
    $web = @()
    while ($queue.Count -gt 0) {
        $id = $queue.Dequeue()
        foreach ($c in @($children[$id])) {
            if (-not $c) { continue }
            if ($c.Name -eq 'msedgewebview2.exe') { $web += $c }
            $queue.Enqueue($c.ProcessId)
        }
    }
    $mb = 1MB
    $appRss = ($app | Measure-Object WorkingSetSize -Sum).Sum / $mb
    $appPriv = ($app | Measure-Object PrivatePageCount -Sum).Sum / $mb
    $webRss = ($web | Measure-Object WorkingSetSize -Sum).Sum / $mb
    $webPriv = ($web | Measure-Object PrivatePageCount -Sum).Sum / $mb
    $free = (Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory / 1KB
    $lt = Read-LongTasks
    $row = [string]::Format([Globalization.CultureInfo]::InvariantCulture,
        '{0:s},{1:F0},{2:F0},{3:F0},{4:F0},{5},{6:F0},{7},{8:F0}',
        (Get-Date), $appRss, $appPriv, $webRss, $webPriv, $web.Count, $free, $lt[0], $lt[1])
    Add-Content -Encoding utf8 $Out $row
    Write-Host $row
    Start-Sleep -Seconds $Interval
}
