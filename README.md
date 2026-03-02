# Heimdall 🛡️

> *ผู้พิทักษ์แห่ง LLM realm* — Internal LLM API Server on Mac Mini M4 Pro 64GB

**Heimdall** (เฮมดัลล์) เป็นเทพผู้พิทักษ์สะพาน Bifrost ในตำนานนอร์ส มองเห็นได้ไกลถึงขอบจักรวาล ได้ยินแม้หญ้างอก — เช่นเดียวกับ gateway นี้ที่เฝ้าดูทุก request ที่ผ่านเข้ามา

## Architecture

```
Clients (LAN)  →  Heimdall Gateway (:3000)  →  vllm-mlx (:8000)
                  ├── 🔑 API Key Auth              └── Qwen3.5-35B-A3B
                  ├── 🛡️ Rate Limiting                  (MLX, 4-bit, MoE)
                  ├── 💚 Health Check
                  ├── 📊 Metrics (/metrics)
                  └── 🌊 SSE Streaming Proxy
```

## Quick Start

```bash
# 1. Setup (one-time)
./scripts/setup.sh

# 2. Start server
./scripts/start.sh

# 3. Check health
./scripts/health_check.sh

# 4. Benchmark
./scripts/benchmark.sh 3
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

# Streaming
curl http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "mlx-community/Qwen3.5-35B-A3B-Instruct-4bit",
    "messages": [{"role": "user", "content": "Count 1 to 5"}],
    "stream": true
  }'
```

### With API Key

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

## Scripts

| Script | Command | Description |
|:--|:--|:--|
| Setup | `./scripts/setup.sh` | Install dependencies + build |
| Start | `./scripts/start.sh` | Launch vllm-mlx + Heimdall |
| Stop | `./scripts/stop.sh` | Graceful shutdown |
| Health | `./scripts/health_check.sh` | Check status |
| Benchmark | `./scripts/benchmark.sh 5` | Run benchmarks + HTML report |
| Version | `./scripts/version.sh show` | SemVer management |

## Configuration

Copy `.env.example` to `.env` and edit:

| Variable | Default | Description |
|:--|:--|:--|
| `BACKEND_PORT` | `8000` | vllm-mlx port |
| `GATEWAY_PORT` | `3000` | Heimdall Gateway port |
| `LLM_MODEL` | `Qwen3.5-35B-A3B-Instruct-4bit` | Default model |
| `API_KEYS` | *(empty)* | Comma-separated keys (empty = no auth) |
| `HOST` | `0.0.0.0` | Bind address |

## Project Structure

```
heimdall/
├── gateway/                    # Rust API Gateway (Axum)
│   └── src/
│       ├── main.rs             # Entry point
│       ├── auth.rs             # API key authentication
│       ├── config.rs           # Environment config
│       ├── health.rs           # Health check endpoints
│       ├── metrics_handler.rs  # Prometheus metrics
│       └── proxy.rs            # Reverse proxy + SSE streaming
├── scripts/                    # Operation & benchmark scripts
├── docs/iso29110/              # ISO 29110 project documentation
├── VERSION                     # SemVer (0.1.0)
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
