<#
    Build/run launcher.

        .\go.ps1            -> menu
        .\go.ps1 dev        -> server + client, offline, no deploy
        .\go.ps1 release    -> deploy\ containing floret-server.exe and dist\

    dev never touches the network and release never enables dev-local, so the
    two paths cannot contaminate each other.
#>
param([ValidateSet('dev', 'release')][string] $Mode)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

$site = Join-Path (Split-Path $PSScriptRoot -Parent) 'burvy-dev'

if (-not $Mode) {
    Write-Host ''
    Write-Host '  1. dev      server + client locally, offline'
    Write-Host '  2. release  build deploy\ for the live server'
    Write-Host ''
    $Mode = switch (Read-Host 'Choose') {
        '1' { 'dev' }
        '2' { 'release' }
        default { throw 'Pick 1 or 2.' }
    }
}

# Resolve cargo here instead of trusting the spawned window to inherit PATH.
# The server runs in a second window launched through the Store build of pwsh,
# and if that child comes up without .cargo\bin on PATH the only symptom is a
# bare "cargo is not recognized" in a window you did not type in. Passing the
# absolute path removes the guesswork.
$cargo = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $cargo) {
    throw "cargo is not on PATH in this window. Open a new terminal, or check that $env:USERPROFILE\.cargo\bin is on PATH."
}

function Invoke-Step($Description, [scriptblock] $Command) {
    Write-Host "`n==> $Description" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Description failed" }
}

if ($Mode -eq 'dev') {
    # Build both before launching anything: `cargo run` would start the client
    # while the server is still compiling, and it would fail to connect.
    Invoke-Step 'building server' { cargo build -p floret-server --features dev-local }
    Invoke-Step 'building client' { cargo build --features dev-local }

    Write-Host "`n==> starting server in a new window" -ForegroundColor Cyan
    Start-Process pwsh -ArgumentList @(
        '-NoExit', '-Command',
        "Set-Location '$PSScriptRoot'; & '$cargo' run -p floret-server --features dev-local"
    )

    # Client runs here so Ctrl+C returns you to a prompt; the server window keeps
    # its logs and outlives the client. Launch this script twice to get two
    # people in the room.
    Write-Host '==> starting client (Ctrl+C to stop, server window stays open)' -ForegroundColor Cyan
    cargo run --features dev-local
    exit
}

# RELEASE
# Ask cargo where target/ is rather than hardcoding it, so moving target-dir
# again does not break this.
$targetDir = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).target_directory
$exe = Join-Path $targetDir 'x86_64-pc-windows-msvc\release\floret-server.exe'

Invoke-Step 'building server (release)' { cargo build --release -p floret-server }
if (-not (Test-Path $exe)) { throw "expected the server at $exe" }

# Only floret-wasm: build.ps1 clears just what it rebuilds, so the other
# experiences keep the output they already have.
Invoke-Step 'building site + floret wasm' { & (Join-Path $site 'build.ps1') floret-wasm }

$out = Join-Path $PSScriptRoot 'deploy'
Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $out | Out-Null
Copy-Item $exe $out
Copy-Item (Join-Path $site 'dist') $out -Recurse

Write-Host "`ndeploy\ ready:" -ForegroundColor Green
Write-Host '  floret-server.exe   -> C:\Users\Burvy\Desktop\'
Write-Host '  dist\               -> C:\Users\Burvy\Desktop\burvy-dev\dist'
Write-Host '  then restart floret-server.exe (nginx needs no reload)'
Invoke-Item $out
