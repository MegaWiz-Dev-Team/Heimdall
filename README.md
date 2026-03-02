# Mega LLM Server

> Internal LLM API Server on Mac Mini M4 Pro 64GB — vllm-mlx + Rust Gateway

## Architecture

```
Clients (LAN)  →  Rust Gateway (:3000)  →  vllm-mlx (:8000)
                  ├── API Key Auth            └── Qwen3.5-35B-A3B
                  ├── Rate Limiting                (MLX, 4-bit, MoE)
                  ├── Health Check
                  ├── Metrics (/metrics)
                  └── SSE Streaming Proxy
```

## Quick Start

```bash
# 1. Setup (one-time)
./scripts/setup.sh

# 2. Start server
./scripts/start.sh

# 3. Check health
./scripts/health_check.sh
```

## API Usage

API is **OpenAI-compatible** — use any OpenAI client library:

```bash
# List models
curl http://localhost:3000/v1/models

# Chat completion
curl http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "mlx-community/Qwen3.5-35B-A3B-Instruct-4bit",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# With streaming
curl http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "mlx-community/Qwen3.5-35B-A3B-Instruct-4bit",
    "messages": [{"role": "user", "content": "Count 1 to 5"}],
    "stream": true
  }'
```

### With API Key (if configured)

```bash
curl http://localhost:3000/v1/models \
  -H "Authorization: Bearer your-api-key"
```

### Python Client

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:3000/v1",
    api_key="your-api-key",  # or "none" if auth disabled
)

response = client.chat.completions.create(
    model="mlx-community/Qwen3.5-35B-A3B-Instruct-4bit",
    messages=[{"role": "user", "content": "Hello!"}],
)
print(response.choices[0].message.content)
```

## Configuration

Copy `.env.example` to `.env` and edit:

| Variable | Default | Description |
|:--|:--|:--|
| `BACKEND_PORT` | `8000` | vllm-mlx listen port |
| `GATEWAY_PORT` | `3000` | Gateway listen port |
| `LLM_MODEL` | `mlx-community/Qwen3.5-35B-A3B-Instruct-4bit` | Default model |
| `API_KEYS` | *(empty)* | Comma-separated API keys (empty = no auth) |
| `HOST` | `0.0.0.0` | Bind address |

## Project Structure

```
mega-llm-server/
├── gateway/                    # Rust API Gateway (Axum)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # Entry point
│       ├── auth.rs             # API key authentication
│       ├── config.rs           # Environment config
│       ├── health.rs           # Health check endpoints
│       ├── metrics_handler.rs  # Prometheus metrics
│       └── proxy.rs            # Reverse proxy + SSE streaming
├── scripts/
│   ├── setup.sh                # One-time setup
│   ├── start.sh                # Start all services
│   ├── stop.sh                 # Stop all services
│   └── health_check.sh         # Health check
├── docs/
│   ├── iso29110/               # ISO 29110 project documentation
│   └── llm_mlx.md              # MLX model reference
├── .agents/                    # Antigravity rules & workflows
├── .env.example                # Config template
└── README.md
```

## Hardware

| Spec | Value |
|:--|:--|
| Machine | Mac Mini M4 Pro |
| RAM | 64GB Unified Memory (~48GB usable for GPU) |
| Bandwidth | 273 GB/s |
| Default Model | Qwen3.5-35B-A3B-4bit (MoE, ~20GB, ~100+ tok/s) |

## License

MIT
