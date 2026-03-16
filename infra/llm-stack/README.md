# LLM Inference Stack

Local LLM inference stack for AMD RDNA 4 GPUs via Docker + ROCm.

**Hardware target**: RX 9070 XT 16GB · Ryzen 9 5900XT · 64GB DDR4 · Windows 11

## Quick Start

```powershell
# From this directory (or wherever you cloned it)
.\setup.ps1
```

The script will:
1. Validate Docker Desktop is running
2. Create `C:\dev\llm\` and copy config files
3. Pull Docker images
4. Start Ollama (ROCm) + Open WebUI
5. Pull `qwen2.5-coder:7b`
6. Verify GPU access

## Services

| Service | URL | Image |
|---------|-----|-------|
| Ollama API | http://localhost:11434 | `ollama/ollama:rocm` |
| Open WebUI | http://localhost:3000 | `ghcr.io/open-webui/open-webui:main` |

## Prerequisites

- **Docker Desktop** with WSL2 backend enabled
- **AMD GPU drivers** installed on Windows (Adrenalin 25.x+)
- **ROCm support in WSL2**: Docker Desktop handles GPU passthrough via `/dev/kfd` and `/dev/dri`

## RDNA 4 Compatibility

ROCm does not officially support RDNA 4 (gfx1201) yet. The stack uses:

```
HSA_OVERRIDE_GFX_VERSION=11.0.0
```

This forces ROCm to treat the RX 9070 XT as a Navi 31 (gfx1100 / RDNA 3) device. This is set in both `.env` and `docker-compose.yml` to ensure it propagates everywhere.

## GPU Verification

```powershell
# Check ROCm sees the GPU
docker exec ollama-rocm bash -c "rocm-smi"

# Check GPU utilization during inference
docker exec ollama-rocm bash -c "rocm-smi" &; docker exec ollama-rocm ollama run qwen2.5-coder:7b "Hello"

# Verbose mode — look for GPU layer offloading in output
docker exec ollama-rocm ollama run qwen2.5-coder:7b "Hello" --verbose

# If rocm-smi shows no GPU, check HSA override is active
docker exec ollama-rocm bash -c "echo \$HSA_OVERRIDE_GFX_VERSION"
```

**Signs of GPU acceleration**:
- `rocm-smi` shows GPU memory usage during inference
- Verbose output mentions GPU layers being offloaded
- Inference speed is significantly faster than expected for CPU (7B model should generate 30+ tokens/sec on RX 9070 XT)

**Signs of CPU fallback**:
- Very slow inference (< 5 tokens/sec for 7B)
- `rocm-smi` shows 0% GPU utilization during inference
- Ollama logs mention "no GPU detected"

## Manual Setup

If you prefer not to use the setup script:

```powershell
# 1. Create working directory
mkdir C:\dev\llm
Copy-Item docker-compose.yml, .env C:\dev\llm\
cd C:\dev\llm

# 2. Start the stack
docker compose up -d

# 3. Wait for ollama to be ready, then pull a model
docker exec ollama-rocm ollama pull qwen2.5-coder:7b

# 4. Test
docker exec ollama-rocm ollama run qwen2.5-coder:7b "Hello, world"
```

## Model Management

```powershell
# List installed models
docker exec ollama-rocm ollama list

# Pull a model
docker exec ollama-rocm ollama pull codellama:13b
docker exec ollama-rocm ollama pull deepseek-coder-v2:16b

# Remove a model
docker exec ollama-rocm ollama rm <model-name>

# Model storage location (Docker volume)
docker volume inspect llm-stack-ollama-data
```

### Recommended Models for 16GB VRAM

| Model | Size | Use Case |
|-------|------|----------|
| `qwen2.5-coder:7b` | ~4.5GB | Code generation, default |
| `codellama:13b` | ~7.4GB | Code analysis |
| `deepseek-coder-v2:16b` | ~9GB | Advanced code reasoning |
| `llama3.1:8b` | ~4.7GB | General purpose |
| `mistral:7b` | ~4.1GB | Fast general purpose |

## Stack Operations

```powershell
# Stop everything
cd C:\dev\llm && docker compose down

# Restart
cd C:\dev\llm && docker compose restart

# View logs
docker logs -f ollama-rocm
docker logs -f open-webui

# Update images
docker compose pull && docker compose up -d
```

## Ollama API Usage

The Ollama API is available at `http://localhost:11434/api`. Useful for programmatic access from the ouroboros agent.

```powershell
# Generate (streaming)
curl http://localhost:11434/api/generate -d '{"model":"qwen2.5-coder:7b","prompt":"Write a hello world in Rust"}'

# Chat
curl http://localhost:11434/api/chat -d '{"model":"qwen2.5-coder:7b","messages":[{"role":"user","content":"Explain borrow checking"}]}'

# List models
curl http://localhost:11434/api/tags

# Check running models
curl http://localhost:11434/api/ps
```

## Troubleshooting

### Ollama container fails to start
- Check Docker Desktop is using Linux containers (not Windows)
- Verify WSL2 backend is enabled in Docker Desktop settings
- Check logs: `docker logs ollama-rocm`

### GPU not detected / CPU fallback
1. Verify `HSA_OVERRIDE_GFX_VERSION=11.0.0` is set: `docker exec ollama-rocm printenv | findstr HSA`
2. Check devices are passed through: `docker exec ollama-rocm ls -la /dev/kfd /dev/dri`
3. Ensure AMD GPU drivers are up to date on the Windows host
4. Try restarting Docker Desktop (sometimes WSL2 GPU passthrough needs a restart)

### Open WebUI can't connect to Ollama
- Ensure both containers are on the same Docker network (compose handles this)
- Check Ollama is healthy: `docker inspect --format='{{.State.Health.Status}}' ollama-rocm`
- Verify the URL in WebUI settings is `http://ollama:11434` (container name, not localhost)

### Permission denied for /dev/kfd or /dev/dri
- The compose file adds `video` and `render` groups — this should suffice
- On some WSL2 setups, you may need to add your user to these groups in WSL

## Ouroboros Integration

The ouroboros agent will connect to Ollama via the REST API:

```
Base URL: http://localhost:11434/api
Default model: qwen2.5-coder:7b
```

Environment variables for the agent are stubbed in `.env` (commented out). Uncomment and configure when ready.
