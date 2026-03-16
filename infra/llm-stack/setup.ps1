#Requires -Version 5.1
<#
.SYNOPSIS
    Sets up the local LLM inference stack (Ollama ROCm + Open WebUI) for AMD RDNA 4 GPUs.

.DESCRIPTION
    Deploys Ollama with ROCm backend and Open WebUI via Docker Compose.
    Configured for AMD RX 9070 XT (RDNA 4 / gfx1201) with HSA override.
    Target directory: C:\dev\llm\

.NOTES
    Hardware: AMD RX 9070 XT 16GB VRAM, Ryzen 9 5900XT, 64GB DDR4
    Prerequisites: Docker Desktop with WSL2 backend
#>

[CmdletBinding()]
param(
    [string]$TargetDir = "C:\dev\llm",
    [string]$Model = "qwen2.5-coder:7b",
    [switch]$SkipModelPull,
    [switch]$SkipGpuCheck
)

$ErrorActionPreference = "Stop"

# ── Colors & Helpers ─────────────────────────────────────────────────

function Write-Step { param([string]$Msg) Write-Host "`n▸ $Msg" -ForegroundColor Cyan }
function Write-OK   { param([string]$Msg) Write-Host "  ✓ $Msg" -ForegroundColor Green }
function Write-Warn { param([string]$Msg) Write-Host "  ⚠ $Msg" -ForegroundColor Yellow }
function Write-Err  { param([string]$Msg) Write-Host "  ✗ $Msg" -ForegroundColor Red }

function Test-CommandExists {
    param([string]$Cmd)
    $null -ne (Get-Command $Cmd -ErrorAction SilentlyContinue)
}

# ── Prerequisites ────────────────────────────────────────────────────

Write-Step "Checking prerequisites"

if (-not (Test-CommandExists "docker")) {
    Write-Err "Docker is not installed or not in PATH."
    Write-Host "  Install Docker Desktop: https://docs.docker.com/desktop/install/windows-install/"
    exit 1
}
Write-OK "Docker CLI found"

# Check Docker Desktop is running
try {
    $dockerInfo = docker info 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Docker not responding" }
    Write-OK "Docker Desktop is running"
} catch {
    Write-Err "Docker Desktop is not running. Please start it first."
    exit 1
}

# Check WSL2 backend
if ($dockerInfo -match "Operating System:.*Windows") {
    Write-Warn "Docker appears to be using Windows containers. Switch to Linux containers for WSL2."
}

if (-not (Test-CommandExists "docker-compose") -and -not (Test-CommandExists "docker")) {
    Write-Err "docker compose not available."
    exit 1
}
Write-OK "docker compose available"

# ── Directory Setup ──────────────────────────────────────────────────

Write-Step "Setting up directory: $TargetDir"

if (-not (Test-Path $TargetDir)) {
    New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
    Write-OK "Created $TargetDir"
} else {
    Write-OK "Directory exists"
}

# Copy files to target directory
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$filesToCopy = @("docker-compose.yml", ".env", ".gitignore")

foreach ($file in $filesToCopy) {
    $src = Join-Path $ScriptDir $file
    $dst = Join-Path $TargetDir $file
    if (Test-Path $src) {
        Copy-Item -Path $src -Destination $dst -Force
        Write-OK "Copied $file"
    } else {
        Write-Warn "$file not found in script directory, skipping"
    }
}

# ── Pull Images ──────────────────────────────────────────────────────

Write-Step "Pulling Docker images (this may take a while on first run)"

$images = @(
    "ollama/ollama:rocm",
    "ghcr.io/open-webui/open-webui:main"
)

foreach ($img in $images) {
    Write-Host "  Pulling $img ..." -NoNewline
    docker pull $img 2>&1 | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Host " done" -ForegroundColor Green
    } else {
        Write-Err " failed to pull $img"
        exit 1
    }
}

# ── Start Stack ──────────────────────────────────────────────────────

Write-Step "Starting LLM stack"

Push-Location $TargetDir
try {
    docker compose down 2>&1 | Out-Null
    docker compose up -d
    if ($LASTEXITCODE -ne 0) {
        Write-Err "Failed to start stack"
        exit 1
    }
    Write-OK "Stack started"
} finally {
    Pop-Location
}

# ── Wait for Ollama Health ───────────────────────────────────────────

Write-Step "Waiting for Ollama to become healthy"

$maxAttempts = 30
$attempt = 0
$healthy = $false

while ($attempt -lt $maxAttempts) {
    $attempt++
    Start-Sleep -Seconds 5

    $health = docker inspect --format='{{.State.Health.Status}}' ollama-rocm 2>&1
    if ($health -eq "healthy") {
        $healthy = $true
        break
    }

    Write-Host "  Attempt $attempt/$maxAttempts - status: $health" -ForegroundColor Gray
}

if (-not $healthy) {
    Write-Warn "Ollama did not become healthy within timeout. Continuing anyway..."
    Write-Host "  Check logs: docker logs ollama-rocm"
} else {
    Write-OK "Ollama is healthy"
}

# ── GPU Verification ─────────────────────────────────────────────────

if (-not $SkipGpuCheck) {
    Write-Step "Verifying GPU access"

    $rocmOutput = docker exec ollama-rocm bash -c "HSA_OVERRIDE_GFX_VERSION=11.0.0 rocm-smi 2>&1" 2>&1
    if ($LASTEXITCODE -eq 0 -and $rocmOutput -match "GPU") {
        Write-OK "ROCm detects GPU"
        Write-Host $rocmOutput -ForegroundColor Gray
    } else {
        Write-Warn "rocm-smi did not detect GPU. May still work with HSA override."
        Write-Host "  Output: $rocmOutput" -ForegroundColor Gray
        Write-Host "  This is common with RDNA 4 - Ollama may still use the GPU." -ForegroundColor Gray
    }
}

# ── Pull Default Model ───────────────────────────────────────────────

if (-not $SkipModelPull) {
    Write-Step "Pulling model: $Model (this will take several minutes)"

    docker exec ollama-rocm ollama pull $Model
    if ($LASTEXITCODE -eq 0) {
        Write-OK "Model $Model pulled successfully"
    } else {
        Write-Err "Failed to pull model $Model"
        Write-Host "  Try manually: docker exec ollama-rocm ollama pull $Model"
    }

    # Quick smoke test
    Write-Step "Running smoke test"
    $response = docker exec ollama-rocm ollama run $Model "Respond with only: GPU inference working" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-OK "Model responded: $response"
    } else {
        Write-Warn "Smoke test inconclusive. Check: docker logs ollama-rocm"
    }
}

# ── Summary ──────────────────────────────────────────────────────────

Write-Host "`n" -NoNewline
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  LLM Stack Ready" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Ollama API:    http://localhost:$((Get-Content (Join-Path $TargetDir '.env') | Select-String 'OLLAMA_PORT=(\d+)').Matches.Groups[1].Value)" -ForegroundColor White
Write-Host "  Open WebUI:    http://localhost:$((Get-Content (Join-Path $TargetDir '.env') | Select-String 'WEBUI_PORT=(\d+)').Matches.Groups[1].Value)" -ForegroundColor White
Write-Host "  GPU Override:  HSA_OVERRIDE_GFX_VERSION=11.0.0 (RDNA 4)" -ForegroundColor Gray
Write-Host ""
Write-Host "  Verify GPU:    docker exec ollama-rocm bash -c 'rocm-smi'" -ForegroundColor Gray
Write-Host "  View logs:     docker logs -f ollama-rocm" -ForegroundColor Gray
Write-Host "  Stop stack:    cd $TargetDir && docker compose down" -ForegroundColor Gray
Write-Host "  Pull models:   docker exec ollama-rocm ollama pull <model>" -ForegroundColor Gray
Write-Host ""
Write-Host "  Ouroboros API endpoint: http://localhost:11434/api" -ForegroundColor Yellow
Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
