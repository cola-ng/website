param([int]$DatabasePort = 55439, [int]$ApiPort = 8129, [int]$WebPort = 5173)
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $repo
$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE) { throw 'cargo metadata failed' }
$artifacts = $metadata.target_directory
$databaseDir = Join-Path $artifacts "qa-postgres-$PID"
$pgBin = Split-Path (Get-Command psql).Source
$databaseUrl = "postgresql://colang_qa@127.0.0.1:${DatabasePort}/colang_qa"
$apiProcess = $null
$webProcess = $null
$startedDatabase = $false
function Checked([scriptblock]$command) {
    & $command
    if ($LASTEXITCODE) { throw "Command failed with exit code $LASTEXITCODE" }
}
try {
    foreach ($port in @($DatabasePort,$ApiPort,$WebPort)) {
        $probe = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback,$port)
        try { $probe.Start() } catch { throw "Port $port is already in use; choose another QA port" } finally { $probe.Stop() }
    }
    Checked { cargo test --locked -p colang-front }
    Checked { cargo clippy --locked -p colang-front --target wasm32-unknown-unknown -- -D warnings }
    Checked { cargo build --locked -p colang --bin colang }
    Push-Location -LiteralPath (Join-Path $repo 'front')
    try {
        # 0.17 is also usable on development machines; locked builds were added later.
        if ((trunk build --help | Out-String).Contains('--locked')) { Checked { trunk build --release --locked } }
        else { Checked { trunk build --release } }
    } finally { Pop-Location }
    Checked { & (Join-Path $pgBin 'initdb') -D $databaseDir -U colang_qa --auth=trust --encoding=UTF8 --locale=C }
    Checked { & (Join-Path $pgBin 'pg_ctl') -D $databaseDir -l (Join-Path $artifacts "qa-postgres-$PID.log") -o "-p $DatabasePort -h 127.0.0.1" -w start }
    $startedDatabase = $true
    Checked { & (Join-Path $pgBin 'createdb') -h 127.0.0.1 -p $DatabasePort -U colang_qa colang_qa }
    $environment = @{
        DATABASE_URL=$databaseUrl; JWT_SECRET='isolated-local-qa-secret'; BIND_ADDR="127.0.0.1:$ApiPort"
        STATIC_DIR=(Join-Path $repo 'front/dist'); SPACE_PATH=(Join-Path $artifacts "qa-space-$PID")
        ZHIPU_API_KEY=''; ZHIPUAI_API_KEY=''; DOUBAO_API_KEY=''; DOUBAO_APP_ID=''; DOUBAO_ACCESS_TOKEN=''
    }
    $apiProcess = Start-Process -FilePath (Join-Path $artifacts 'debug/colang.exe') -WorkingDirectory $repo -Environment $environment -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $artifacts "qa-api-$PID.log") -RedirectStandardError (Join-Path $artifacts "qa-api-$PID.err")
    $ready=$false
    for ($attempt=0; $attempt -lt 40; $attempt++) {
        if ($apiProcess.HasExited) { throw 'QA API exited during startup; inspect its log in target_directory' }
        try { Invoke-RestMethod "http://127.0.0.1:$ApiPort/api" | Out-Null; $ready=$true; break } catch { Start-Sleep -Milliseconds 500 }
    }
    if (!$ready) { throw 'QA API failed to become ready' }
    $env:QA_DATABASE_URL=$databaseUrl
    $env:QA_API="http://127.0.0.1:$ApiPort"
    $env:QA_WEB="http://127.0.0.1:$WebPort"
    Checked { python tests/api.py }
    $webProcess = Start-Process -FilePath (Get-Command python).Source -ArgumentList @('scripts/serve.py','--port',"$WebPort",'--api',"http://127.0.0.1:$ApiPort") -WorkingDirectory $repo -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $artifacts "qa-web-$PID.log") -RedirectStandardError (Join-Path $artifacts "qa-web-$PID.err")
    for ($attempt=0; $attempt -lt 20; $attempt++) {
        if ($webProcess.HasExited) { throw 'QA preview exited during startup' }
        try { Invoke-WebRequest $env:QA_WEB | Out-Null; break } catch { Start-Sleep -Milliseconds 250 }
    }
    Checked { node tests/browser.cjs }
    Checked { node tests/ui.cjs }
} finally {
    if ($webProcess -and !$webProcess.HasExited) { Stop-Process -Id $webProcess.Id }
    if ($apiProcess -and !$apiProcess.HasExited) { Stop-Process -Id $apiProcess.Id }
    if ($startedDatabase) { & (Join-Path $pgBin 'pg_ctl') -D $databaseDir -m fast -w stop }
    Remove-Item Env:QA_DATABASE_URL,Env:QA_API,Env:QA_WEB -ErrorAction SilentlyContinue
}
