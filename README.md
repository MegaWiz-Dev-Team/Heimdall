# 🛡️ Heimdall — LLM Gateway

[![Version](https://img.shields.io/badge/version-0.4.0-blue.svg)](CHANGELOG.md)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-macOS_Apple_Silicon-lightgrey.svg)](#)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Part of Asgard](https://img.shields.io/badge/part%20of-Asgard%20AI%20Platform-purple.svg)](https://github.com/MegaWiz-Dev-Team/Asgard)

> Part of the [Asgard AI Platform](https://github.com/MegaWiz-Dev-Team/Asgard)

Heimdall is a high-performance LLM gateway built in Rust (Axum + Tokio) that provides a unified OpenAI-compatible API for multiple local LLM backends.

### 🏥 Role in Multi-Agent Ecosystem

> **LLM Gateway + Step-up Router (ยามเฝ้าประตู)** — Heimdall เป็นด่านหน้าที่ทุก Agent เรียกใช้ LLM ผ่าน โดยปกติ route ไปยังโมเดล Local (MedGemma / Qwen 3.5) แต่เมื่อเคสมีความอันตราย จะ **Step-up** ไปยัง Gemini 2.5 Pro เพื่อ Second Opinion
>
> **Guardrails:** G4 (Safety Filters, Temperature Clamp ≤ 0.3, Token Budget, Timeout)
>
> 📖 [Full Architecture →](https://github.com/MegaWiz-Dev-Team/Asgard/blob/main/docs/roadmap/MultiAgent_Architecture_Plan.md) | [Sprint Plan →](https://github.com/MegaWiz-Dev-Team/Asgard/blob/main/docs/roadmap/MultiAgent_Sprint_Plan.md)

## Features

- 🔄 **Multi-backend routing** — MLX, llama.cpp, Ollama, vLLM
- 🔐 **API Key authentication** — Bearer token validation with in-memory Rust speed
- 📊 **Prometheus metrics** — Request latency, token usage, auth tracking
- 🌊 **Zero-copy SSE streaming** — Real-time token streaming without memory bloat
- 📖 **Interactive API Docs** — Out-of-the-box OpenAPI 3.1 spec & Scalar Explorer UI
- 📈 **Benchmark suite** — Automated performance testing with historical tracking
- 🧮 **Native Rust Embedding Engine** — Optimized `bge-m3` using FastEmbed & ONNX Runtime without Python scripts
- 🛠️ **Deployment Utilities** — Built-in native MLX converters and Hugging Face publishers
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
| `8080` | **Heimdall Gateway** | Main API endpoint / Native Rust Embedding (Batch) |
| `8081` | mlx_lm | Text LLM backend |
| `8082` | mlx_vlm | Vision LLM backend *(reserved)* |
| `8083` | llama.cpp | GGUF backend *(reserved)* |
| `8084` | vLLM | NVIDIA backend *(reserved)* |
| `8089` | llama.cpp | Llama Fast Embeddings (Chat) |
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

### 2. Production Deployment (Daemons)
Heimdall components run as native OS Background Services (`launchd` on macOS). This prevents overlapping ports, terminal detachment issues, and allows for clean auto-restarts.

To install or reinstall Heimdall as a system daemon, run:
```bash
./scripts/install_daemons.sh
```

**Core Services Configured:**
1. **Rust Gateway** (`:8080`) — Primary API Gateway and reverse proxy. Managed by `launchd`.
2. **Embeddings Engine** (`:8001`, `:8089`) — Dual embedding setups. Managed by `launchd`.

> [!IMPORTANT]
> **Text Generation Engines (Port 8081)** are **INTENTIONALLY DECOUPLED** from `launchd`. 
> Depending on your hardware and requirements, you must manually run your engine of choice:
> - **Heavy Workloads (31B+, MoE)**: Use `Mimir/scripts/run_flash_moe.sh` to run the C++ Native Engine on `8081`. 
> - **Light/Research Workloads**: You can start Ollama or other local servers mapping backwards to `8081`.

### 3. Service & Model Management (Heimdall CLI)
Heimdall includes a unified CLI `./heimdall` at the root of the project to efficiently manage models and daemons. Legacy scripts like `start.sh` and `stop.sh` are **deprecated**.

```bash
cd ~/Developer/Heimdall

# Download a new model directly to the Heimdall Cache
./heimdall pull mlx-community/Qwen3.5-35B-A3B-4bit

# View all installed models across primary and external SSDs
./heimdall ls

# Permanently delete a model to free up space
./heimdall rm mlx-community/Qwen3-0.6B-4bit

# Start, Stop, or Restart the Heimdall background daemons
./heimdall start
./heimdall restart
./heimdall stop
```

*Optional:* Make it globally accessible by linking it to your bin folder:
```bash
sudo ln -sf ~/Developer/Heimdall/heimdall /usr/local/bin/heimdall
```

### 4. Test
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

Heimdall includes rigorous performance testing to validate inference throughput and latency.

```bash
# Run text generation benchmarks
./scripts/benchmark.sh
./scripts/benchmark_history.sh
open reports/

# Run hybrid embedding architecture benchmark (tests Rust ONNX vs MLX vs Llama.cpp)
python3 scripts/benchmark_embedding.py
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
Client → [:8080] Heimdall Gateway (Rust/Axum) [Includes Native ONNX Embedding]
              ├→ [:8081] mlx_lm         (Python/MLX)
              ├→ [:8082] mlx_vlm        (Python/MLX)  [reserved]
              ├→ [:8083] llama.cpp      (C++)         [reserved]
              ├→ [:8084] vLLM           (Python/CUDA) [reserved]
              ├→ [:8085] Flash-MoE      (SSD/MLX)     [experimental]
              ├→ [:8089] llama-server   (C++)         [Fast Embeddings]
              └→ [:11434] Ollama        (Go)
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
