param([switch]$NoBrowser, [switch]$SkipBuild, [switch]$Restart, [switch]$Lan, [int]$Port = 8119)
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
Push-Location $project
try {
    $metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE) { throw 'Cargo metadata failed' }
    $target = [IO.Path]::GetFullPath($metadata.target_directory)
    $executable = Join-Path $target 'debug/colang.exe'
    $runtime = Join-Path $target 'amigo-server'
    New-Item -ItemType Directory -Force -Path $runtime | Out-Null
    $old = Get-CimInstance Win32_Process -Filter "name='colang.exe'" | Where-Object { $_.ExecutablePath -eq $executable }
    if ($old -and !$Restart) { throw 'This website backend is already running; use -Restart to replace it.' }
    if ($old) { $old | ForEach-Object { Stop-Process -Id $_.ProcessId } }
    if (!$SkipBuild) {
        if (!$env:PQ_LIB_DIR -and (Test-Path 'C:/Program Files/PostgreSQL/18/lib/libpq.lib')) {
            $env:PQ_LIB_DIR = 'C:/Program Files/PostgreSQL/18/lib'
            $env:PATH = 'C:/Program Files/PostgreSQL/18/bin;' + $env:PATH
        }
        cargo build -p colang --bin colang
        if ($LASTEXITCODE) { throw 'Website backend build failed' }
        Push-Location (Join-Path $project 'front')
        try { trunk build --release; if ($LASTEXITCODE) { throw 'WASM frontend build failed' } }
        finally { Pop-Location }
    }
    if (!(Test-Path $executable) -or !(Test-Path (Join-Path $project 'front/dist/index.html'))) { throw 'Build the website backend and frontend first.' }
    $env:STATIC_DIR = Join-Path $project 'front/dist'
    $env:BIND_ADDR = if ($Lan) { "0.0.0.0:$Port" } else { "127.0.0.1:$Port" }
    if ($Lan) {
        $probe = New-Object Net.Sockets.UdpClient
        try { $probe.Connect('192.0.2.1', 9); $address = $probe.Client.LocalEndPoint.Address.ToString(); $env:AMIGO_GATEWAY_URL = "ws://${address}:$Port/api/amigo/session" }
        finally { $probe.Dispose() }
    }
    $env:RUST_LOG = 'warn,colang::services::amigo::gateway=info'
    $process = Start-Process -FilePath $executable -WorkingDirectory $project -WindowStyle Hidden -PassThru -RedirectStandardOutput "$runtime/server.stdout.log" -RedirectStandardError "$runtime/server.stderr.log"
    for ($attempt=0; $attempt -lt 60; $attempt++) {
        if ($process.HasExited) { throw "Website backend exited; inspect $runtime/server.stderr.log" }
        try { if ((Invoke-WebRequest "http://127.0.0.1:$Port/api" -UseBasicParsing -TimeoutSec 2).StatusCode -eq 200) { break } } catch { Start-Sleep -Milliseconds 500 }
    }
    if ($attempt -eq 60) { throw "Website startup timed out; inspect $runtime/server.stderr.log" }
    $url = "http://127.0.0.1:$Port/amigo"
    Write-Host "Colang website and Amigo gateway: $url (PID $($process.Id))"
    if (!$NoBrowser) { Start-Process $url }
} finally { Pop-Location }
