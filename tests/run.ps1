param([int]$DatabasePort = 55439, [int]$ApiPort = 8129, [int]$WebPort = 5173, [int]$ProviderPort = 8134, [switch]$SkipBuild)
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
$providerProcess = $null
$startedDatabase = $false
function Checked([scriptblock]$command) {
    & $command
    if ($LASTEXITCODE) { throw "Command failed with exit code $LASTEXITCODE" }
}
try {
    foreach ($port in @($DatabasePort,$ApiPort,$WebPort,$ProviderPort)) {
        $probe = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback,$port)
        try { $probe.Start() } catch { throw "Port $port is already in use; choose another QA port" } finally { $probe.Stop() }
    }
    Checked { cargo test --locked -p colang-front }
    Checked { cargo clippy --locked -p colang-front --target wasm32-unknown-unknown -- -D warnings }
    if (!$SkipBuild) {
        Checked { cargo build --locked --profile qa --jobs 2 -p colang --bin colang }
        Push-Location -LiteralPath (Join-Path $repo 'front')
        try {
            # 0.17 is also usable on development machines; locked builds were added later.
            if ((trunk build --help | Out-String).Contains('--locked')) { Checked { trunk build --release --locked } }
            else { Checked { trunk build --release } }
        } finally { Pop-Location }
    }
    Checked { & (Join-Path $pgBin 'initdb') -D $databaseDir -U colang_qa --auth=trust --encoding=UTF8 --locale=C }
    # Redirect to files when starting a daemon so inherited stdout does not hold the caller pipeline open.
    $pgStart = Start-Process -FilePath (Join-Path $pgBin 'pg_ctl.exe') -ArgumentList @('-D',$databaseDir,'-l',(Join-Path $artifacts "qa-postgres-$PID.log"),'-o',"`"-p $DatabasePort -h 127.0.0.1`"",'-w','start') -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $artifacts "qa-pg-start-$PID.log") -RedirectStandardError (Join-Path $artifacts "qa-pg-start-$PID.err")
    $pgStart.WaitForExit()
    if ($pgStart.ExitCode) { throw 'isolated PostgreSQL failed to start' }
    $startedDatabase = $true
    Checked { & (Join-Path $pgBin 'createdb') -h 127.0.0.1 -p $DatabasePort -U colang_qa colang_qa }
    $gatewayDatabase = "amigo_test_qa_$PID"
    Checked { & (Join-Path $pgBin 'createdb') -h 127.0.0.1 -p $DatabasePort -U colang_qa $gatewayDatabase }
    $environment = @{
        DATABASE_URL=$databaseUrl; JWT_SECRET='qa-8e49ea177bb247cb9a401a8696a7ed8e'; BIND_ADDR="127.0.0.1:$ApiPort"
        STATIC_DIR=(Join-Path $repo 'front/dist'); SPACE_PATH=(Join-Path $artifacts "qa-space-$PID")
        AI_PROVIDER_DEFAULT='zhipu'; ZHIPU_API_KEY='qa-platform-key'; ZHIPUAI_API_KEY='qa-platform-key'
        ZHIPUAI_BASE_URL="http://127.0.0.1:$ProviderPort"
        PUBLIC_BASE_URL="http://127.0.0.1:$WebPort"
        GITHUB_CLIENT_ID='qa-github-client'; GITHUB_CLIENT_SECRET='qa-github-secret'
        GOOGLE_CLIENT_ID='qa-google-client'; GOOGLE_CLIENT_SECRET='qa-google-secret'
    }
    $providerProcess = Start-Process -FilePath (Get-Command python).Source -ArgumentList @('tests/provider_stub.py','--port',"$ProviderPort") -WorkingDirectory $repo -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $artifacts "qa-provider-$PID.log") -RedirectStandardError (Join-Path $artifacts "qa-provider-$PID.err")
    $env:QA_PROVIDER_URL="http://127.0.0.1:$ProviderPort"
    for ($attempt=0; $attempt -lt 30; $attempt++) {
        if ($providerProcess.HasExited) { throw 'QA provider stub exited' }
        try { Invoke-RestMethod $env:QA_PROVIDER_URL | Out-Null; break } catch { Start-Sleep -Milliseconds 200 }
    }
    $apiProcess = Start-Process -FilePath (Join-Path $artifacts 'qa/colang.exe') -WorkingDirectory $repo -Environment $environment -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $artifacts "qa-api-$PID.log") -RedirectStandardError (Join-Path $artifacts "qa-api-$PID.err")
    $ready=$false
    for ($attempt=0; $attempt -lt 40; $attempt++) {
        if ($apiProcess.HasExited) { throw 'QA API exited during startup; inspect its log in target_directory' }
        try { Invoke-RestMethod "http://127.0.0.1:$ApiPort/api" | Out-Null; $ready=$true; break } catch { Start-Sleep -Milliseconds 500 }
    }
    if (!$ready) { throw 'QA API failed to become ready' }
    $env:TEST_DATABASE_URL=$databaseUrl
    $env:AMIGO_TEST_DATABASE_URL="postgresql://colang_qa@127.0.0.1:${DatabasePort}/$gatewayDatabase"
    $env:QA_DATABASE_URL=$databaseUrl
    $env:QA_API="http://127.0.0.1:$ApiPort"
    $env:QA_WEB="http://127.0.0.1:$WebPort"
    $env:QA_ARTIFACTS=$artifacts
    $env:QA_SPACE_PATH=$environment.SPACE_PATH
    Checked { node --test tests/platform.test.cjs }
    Checked { npm test --prefix miniprogram }
    Checked { python tests/api.py }
    Checked { python tests/review_integrity.py }
    Checked { python tests/providers.py }
    Checked { python tests/sentence_audio.py }
    Checked { python tests/voice.py }
    Checked { cargo test --locked --jobs 2 -p colang --lib }
    Checked { cargo test --locked --jobs 2 -p colang --lib pool_leases_are_released_between_queries_and_after_failures -- --ignored }
    Checked { cargo test --locked --jobs 2 -p colang --lib gateway_end_to_end -- --ignored }
    Checked { cargo test --locked --jobs 2 -p colang --lib study_flush_database_flow -- --ignored }
    Checked { cargo test --locked --jobs 2 -p colang --lib oauth_database_flow -- --ignored }
    Checked { cargo test --locked --jobs 2 -p colang --lib chat_stream_database_flow -- --ignored }
    Checked { python tests/oauth.py }
    $webProcess = Start-Process -FilePath (Get-Command python).Source -ArgumentList @('scripts/serve.py','--port',"$WebPort",'--api',"http://127.0.0.1:$ApiPort") -WorkingDirectory $repo -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $artifacts "qa-web-$PID.log") -RedirectStandardError (Join-Path $artifacts "qa-web-$PID.err")
    for ($attempt=0; $attempt -lt 20; $attempt++) {
        if ($webProcess.HasExited) { throw 'QA preview exited during startup' }
        try { Invoke-WebRequest $env:QA_WEB | Out-Null; break } catch { Start-Sleep -Milliseconds 250 }
    }
    Checked { powershell -NoProfile -File tests/voice-fixture.ps1 -OutputPath (Join-Path $artifacts 'qa-voice-speech.wav') }
    Checked { node tests/browser.cjs }
    Checked { node tests/ui.cjs }
    Checked { node tests/responsive.cjs }
} finally {
    if ($providerProcess -and !$providerProcess.HasExited) { Stop-Process -Id $providerProcess.Id }
    if ($webProcess -and !$webProcess.HasExited) { Stop-Process -Id $webProcess.Id }
    if ($apiProcess -and !$apiProcess.HasExited) { Stop-Process -Id $apiProcess.Id }
    if ($startedDatabase) { & (Join-Path $pgBin 'pg_ctl') -D $databaseDir -m fast -w stop }
    Remove-Item Env:TEST_DATABASE_URL,Env:AMIGO_TEST_DATABASE_URL,Env:QA_DATABASE_URL,Env:QA_API,Env:QA_WEB,Env:QA_ARTIFACTS,Env:QA_SPACE_PATH,Env:QA_PROVIDER_URL -ErrorAction SilentlyContinue
}
