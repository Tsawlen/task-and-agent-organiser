#Requires -Version 5.1
<#
.SYNOPSIS
    Installs Organiser on Windows.
.DESCRIPTION
    Checks prerequisites (Rust, Node.js), installs npm dependencies,
    builds the Tauri app, and copies the .exe to %LOCALAPPDATA%\Programs.
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$AppName  = "organiser"
$RepoDir  = Split-Path -Parent $MyInvocation.MyCommand.Path

function Write-Step  { param($msg) Write-Host "`n==> $msg" -ForegroundColor Blue }
function Write-Ok    { param($msg) Write-Host $msg -ForegroundColor Green }
function Write-Warn  { param($msg) Write-Host $msg -ForegroundColor Yellow }
function Fail        { param($msg) Write-Host "ERROR: $msg" -ForegroundColor Red; exit 1 }

# ── Rust ──────────────────────────────────────────────────────────────────────
Write-Step "Checking Rust"
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Warn "rustup not found — downloading installer..."
    $installer = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $installer
    & $installer -y --no-modify-path
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"
} else {
    Write-Ok "rustup found"
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Fail "cargo not found after rustup install. Open a new terminal and re-run."
}
Write-Ok "cargo $(cargo --version)"

# ── Node.js ───────────────────────────────────────────────────────────────────
Write-Step "Checking Node.js"
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Fail "Node.js not found. Install it from https://nodejs.org (v20+) and re-run."
}
$nodeMajor = [int](node -e "process.stdout.write(String(process.versions.node.split('.')[0]))")
if ($nodeMajor -lt 20) {
    Fail "Node.js $nodeMajor found but v20+ is required."
}
Write-Ok "node $(node --version)"

# ── WebView2 ──────────────────────────────────────────────────────────────────
Write-Step "Checking WebView2"
$wv2Key = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
if (-not (Test-Path $wv2Key)) {
    Write-Warn "WebView2 runtime not detected — downloading installer..."
    $wv2Installer = "$env:TEMP\MicrosoftEdgeWebview2Setup.exe"
    Invoke-WebRequest -Uri "https://go.microsoft.com/fwlink/p/?LinkId=2124703" -OutFile $wv2Installer
    & $wv2Installer /silent /install
    Write-Ok "WebView2 installed"
} else {
    Write-Ok "WebView2 present"
}

# ── npm install ───────────────────────────────────────────────────────────────
Write-Step "Installing npm dependencies"
Set-Location $RepoDir
npm install

# ── Build ─────────────────────────────────────────────────────────────────────
Write-Step "Building $AppName (this takes a few minutes on first run)"
npm run tauri build

# ── Install ───────────────────────────────────────────────────────────────────
Write-Step "Installing to %LOCALAPPDATA%\Programs"
$bundleDir = Join-Path $RepoDir "src-tauri\target\release"
$exePath   = Get-ChildItem -Path $bundleDir -Filter "*.exe" |
             Where-Object { $_.Name -notmatch "build|deps" } |
             Sort-Object LastWriteTime -Descending |
             Select-Object -First 1 -ExpandProperty FullName

if (-not $exePath) {
    Fail "Could not find built .exe in $bundleDir"
}

$dest = "$env:LOCALAPPDATA\Programs\$AppName"
if (Test-Path $dest) {
    Write-Warn "Removing existing $dest"
    Remove-Item $dest -Recurse -Force
}
New-Item -ItemType Directory -Path $dest -Force | Out-Null
Copy-Item $exePath "$dest\$AppName.exe"

Write-Host "`n✓ Done! Installed to $dest\$AppName.exe" -ForegroundColor Green
Write-Host "  Add $dest to your PATH or create a shortcut to launch it." -ForegroundColor Cyan
