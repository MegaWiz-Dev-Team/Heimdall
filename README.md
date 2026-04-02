# 🛡️ Heimdall — LLM Gateway

> Part of the [Asgard AI Platform](https://github.com/megacare-dev/Asgard)

Heimdall is a high-performance LLM gateway built in Rust (Axum + Tokio) that provides a unified OpenAI-compatible API for multiple local LLM backends.

## Features

- 🔄 **Multi-backend routing** — MLX, llama.cpp, Ollama, vLLM
- 🔐 **API Key authentication** — Bearer token + API key validation
- 📊 **Prometheus metrics** — Request latency, token usage, model stats
- 🌊 **SSE streaming** — Real-time token streaming
- 📈 **Benchmark suite** — Automated performance testing with historical tracking
- 🧮 **MLX Embedding server** — BAAI/bge-m3 for vector embeddings
- 💾 **SQLite persistence** — Benchmark history and model catalog

## Asgard Port Assignments

> Full port map: [Asgard Port Allocation](https://github.com/megacare-dev/Asgard/blob/main/docs/technical/port-allocation-startup.md)

| Port | Service | Description |
|------|---------|-------------|
| `8080` | **Heimdall Gateway** | Main API endpoint |
| `8081` | mlx_lm | Text LLM backend |
| `8082` | mlx_vlm | Vision LLM backend *(reserved)* |
| `8083` | llama.cpp | GGUF backend *(reserved)* |
| `8084` | vLLM | NVIDIA backend *(reserved)* |
| `8001` | Embedding Server | MLX bge-m3 |
| `11434` | Ollama | Managed models |

## Quick Start

### Prerequisites
- macOS with Apple Silicon (M1/M2/M3/M4)
- Rust (1.75+)
- Python 3.11+ (for MLX backends)

### 1. Setup
```bash
cp .env.example .env
./scripts/setup.sh
```

### 2. Start
```bash
./scripts/start.sh
```

This starts (in order):
1. **MLX Backend** (`:8081`) — loads the LLM model
2. **Embedding Server** (`:8001`) — BAAI/bge-m3
3. **Rust Gateway** (`:8080`) — proxies to backends

### 3. Test
```bash
# Health check
curl http://localhost:8080/health

# List models
curl http://localhost:8080/v1/models

# Chat completion
curl http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY_HERE" \
  -H "Content-Type: application/json" \
  -d '{"model":"auto","messages":[{"role":"user","content":"Hello!"}]}'
```

### 4. Stop
```bash
./scripts/stop.sh
```

## Configuration

Edit `.env` to customize:

```env
GATEWAY_PORT=8080          # Gateway listen port
BACKEND_PORT=8081          # MLX backend port
BACKEND_ENGINE=mlx         # mlx | llama.cpp | ollama
LLM_MODEL=mlx-community/Qwen3.5-9B-MLX-4bit
EMBEDDING_MODEL=BAAI/bge-m3
EMBEDDING_PORT=8001
```

## Benchmarks

```bash
./scripts/benchmark.sh              # Run benchmarks
./scripts/benchmark_history.sh      # View historical results
open reports/                       # HTML reports
```

## Open-Source Deployment Tools

Heimdall includes utilities for converting and uploading MLX models for seamless integration with the open-source community:

```bash
# Convert a Hugging Face model natively to MLX 4-bit safely respecting cache logic:
./scripts/convert_qwen_27b.sh

# Upload your MLX compiled model securely to your Hugging Face Organization:
python scripts/upload_to_hf.py --model-dir ./models/MyModel --repo-id MegaWizCo/MyModel
```

## Architecture

```
Client → [:8080] Heimdall Gateway (Rust/Axum)
              ├→ [:8081] mlx_lm    (Python/MLX)
              ├→ [:8082] mlx_vlm   (Python/MLX)  [reserved]
              ├→ [:8083] llama.cpp (C++)          [reserved]
              ├→ [:8084] vLLM     (Python/CUDA)   [reserved]
              └→ [:11434] Ollama  (Go)
```

---

*Part of [🏰 Asgard AI Platform](https://github.com/megacare-dev/Asgard) — A self-hosted AI platform for Apple Silicon & NVIDIA GPU*
