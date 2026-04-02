# 🛡️ Heimdall — LLM Gateway
![Version](https://img.shields.io/badge/version-0.2.0-blue.svg) ![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg) ![Platform](https://img.shields.io/badge/platform-macOS_Apple_Silicon-lightgrey.svg) ![License](https://img.shields.io/badge/license-AGPL_3.0-blue.svg)

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

## Why Heimdall?

While there are many AI proxies and local runners available, Heimdall fills a specific niche for enterprise-grade Apple Silicon deployments:

- **vs. LiteLLM**: Heimdall is written in Rust for bare-metal performance with zero Python-GIL bottlenecks at the gateway layer, optimizing local network traffic.
- **vs. Ollama / LM Studio**: Ollama is a fantastic model runner (which Heimdall can interface with), but Heimdall acts as an overarching **API Gateway** capable of load-balancing, enforcing API keys, and unifying models running across MLX, Llama.cpp, and vLLM simultaneously.
- **vs. LocalAI**: Heimdall focuses strictly on extracting maximum unified-memory bandwidth from Mac M-Series chips through deep MLX native integrations, providing the ultimate environment for reasoning models like `Qwen3.5-Opus`.

## Key Use Cases

1. **Local Drop-in Replacement for OpenAI**: Point your Cursor IDE, LangChain, or agentic applications to `http://localhost:8080/v1` and use world-class models on your Mac for free, completely offline.
2. **Private RAG Ecosystem**: Built-in endpoints for both LLM generation (`:8081`) and Embeddings (`:8001`) make Heimdall the perfect foundation for privacy-focused document retrieval systems.
3. **The Ultimate Mac AI Router**: Consolidate disparate backend inference servers (MLX, Ollama, vLLM) behind a single unified, secure, API-Key-protected endpoint.
4. **Hardware Benchmarking**: Analyze the true capability of your M-Series chip utilizing the integrated `benchmark.sh` generation reports and SQLite persistence tracking.

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

## API Documentation

Heimdall provides interactive API documentation out-of-the-box (OpenAPI 3.1):

- **Scalar UI**: [http://localhost:8080/docs](http://localhost:8080/docs) (Interactive Explorer)
- **OpenAPI JSON**: [http://localhost:8080/api-spec](http://localhost:8080/api-spec)

## Configuration

Customizing Heimdall is handled entirely through the `.env` file at the root of the project.

### 1. Routing Setup (Gateway & Destination)
Control where Heimdall listens and where it sends traffic:
```env
# --- Gateway Configuration (The Front Door) ---
HOST=0.0.0.0               # Allow external LAN access
GATEWAY_PORT=8080          # Heimdall's exposed port

# --- Backend Configuration (The Destination) ---
BACKEND_HOST=127.0.0.1     # Destination engine IP
BACKEND_PORT=8081          # Destination engine port (e.g., MLX or Ollama)
```

### 2. Authentication (API Keys)
Heimdall includes zero-cost, in-memory API key validation. 
```env
# Comma-separated list of allowed Bearer tokens
API_KEYS=sk-mimir-vip-1234,sk-dev-admin-9999
```
* **Enabled**: By providing keys, Heimdall rejects unauthorized requests instantly.
* **Disabled**: By commenting out or leaving `API_KEYS` empty, Heimdall runs as a public API.


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

## Contributing

We welcome contributions! If you'd like to improve Heimdall, please:
1. Ensure your Rust environment (`1.75+`) is configured locally.
2. Submit a Pull Request targeting the `main` branch.
3. Check that your code compiles cleanly via `cargo check` and `cargo test`.

## License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**. See the `LICENSE` file for full details.

---

*Part of [🏰 Asgard AI Platform](https://github.com/megacare-dev/Asgard) — A self-hosted AI platform for Apple Silicon & NVIDIA GPU*
