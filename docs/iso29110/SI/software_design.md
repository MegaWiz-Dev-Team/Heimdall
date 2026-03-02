# Software Design — Heimdall
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-DES-001 |
| **Version** | 1.0 |
| **Last Updated** | 2026-03-03 |
| **Status** | ✅ Implemented |

---

## 2. System Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Internal Network (LAN)                 │
│  Clients (dev machines, CI/CD, AI agents)                │
└──────────────────────┬───────────────────────────────────┘
                       │ HTTP
                       ▼
┌──────────────────────────────────────────────────────────┐
│  Heimdall Gateway (Rust/Axum)              :3000         │
│  ┌──────────┬──────────┬──────────┬──────────┐          │
│  │ Auth     │ Proxy    │ Health   │ Metrics  │          │
│  │ (Bearer) │ (SSE)    │ Check    │(Prometheus│          │
│  └──────────┴────┬─────┴──────────┴──────────┘          │
│                  │                                       │
│                  ▼                                       │
│  ┌──────────────────────────┐                           │
│  │ vllm-mlx      :8000     │                           │
│  │ Qwen3.5-35B-A3B-4bit    │                           │
│  │ (Metal GPU, ~20GB)      │                           │
│  └──────────────────────────┘                           │
│                                                          │
│  Mac Mini M4 Pro — 64GB Unified Memory                  │
└──────────────────────────────────────────────────────────┘
```

---

## 3. Component Design

### 3.1 Heimdall Gateway (`gateway/src/`)

| Module | File | Purpose | Req IDs |
|:--|:--|:--|:--|
| **Main** | `main.rs` | Axum server setup, middleware stack, routing | — |
| **Config** | `config.rs` | Load settings from env (.env) | — |
| **Auth** | `auth.rs` | Bearer token validation, bypass for /health, /metrics | REQ-012 |
| **Proxy** | `proxy.rs` | Reverse proxy to backend, SSE streaming, root endpoint | REQ-010, REQ-015 |
| **Health** | `health.rs` | `/health` + `/ready` — probes backend status | REQ-014 |
| **Metrics** | `metrics_handler.rs` | Prometheus `/metrics` — request count, latency | REQ-016 |

### 3.2 Backend Engine

| Component | Details |
|:--|:--|
| Engine | vllm-mlx (Python) |
| Port | `:8000` |
| API | OpenAI-compatible (`/v1/chat/completions`, `/v1/models`) |
| Model | `mlx-community/Qwen3.5-35B-A3B-Instruct-4bit` |
| Memory | ~20GB (MoE, 4-bit) + ~28GB KV cache |

### 3.3 Scripts (`scripts/`)

| Script | Purpose |
|:--|:--|
| `setup.sh` | Install Rust, Python, vllm-mlx, build gateway |
| `start.sh` | Start vllm-mlx + Heimdall, manage PIDs, readiness check |
| `stop.sh` | Graceful shutdown via PID files |
| `health_check.sh` | Probe gateway + backend health |
| `benchmark.sh` | Multi-model benchmark (--models, --all, --runs) |
| `generate_report.sh` | Generate HTML dashboard from benchmark JSON |
| `report_template.py` | Python HTML report renderer |
| `version.sh` | SemVer management (show/bump/set/tag/release) |
| `benchmark_history.sh` | Query benchmark history, compare versions (planned) |

### 3.4 Data Layer (Planned)

- **`data/heimdall.db`** — SQLite database for benchmark persistence
- Schema:

| Table | Columns | Purpose |
|:--|:--|:--|
| `runs` | id, timestamp, version, git_commit, hardware_json | Benchmark run metadata |
| `results` | id, run_id, model, test_type, tps_avg, ttfb_avg, tokens_avg, memory_mb, runs_json | Per-model per-test results |

### 3.5 Configuration

- **`.env`** file (loaded by both scripts and gateway)
- Key variables: `GATEWAY_PORT`, `BACKEND_PORT`, `API_KEYS`, `LLM_MODEL`

---

## 4. Data Flow

```
Client Request
    │
    ▼
[Auth Middleware] ──── Invalid key → 401
    │ Valid
    ▼
[Route Matching]
    │
    ├─ /health    → health.rs   → probe backend → JSON
    ├─ /metrics   → metrics.rs  → Prometheus text
    ├─ /v1/*      → proxy.rs    → forward to backend
    │                              ├─ JSON response → return
    │                              └─ SSE stream → pipe through
    └─ /          → proxy.rs    → {"name":"Heimdall",...}
```

---

## 5. Requirement Traceability

| Design Component | Requirement IDs | Status |
|:--|:--|:--|
| Gateway — auth.rs | REQ-012 | ✅ 5 tests |
| Gateway — proxy.rs | REQ-010, REQ-015 | ✅ Implemented |
| Gateway — health.rs | REQ-014 | ✅ Implemented |
| Gateway — metrics_handler.rs | REQ-016 | ✅ Implemented |
| Engine — vllm-mlx | REQ-001, REQ-004, REQ-005 | 🟡 Configured |
| Scripts — operations | REQ-020, REQ-021, REQ-023 | ✅ Done |
| Scripts — benchmark | REQ-022 | ✅ Multi-model |
| Scripts — versioning | REQ-024 | ✅ SemVer |
| Data — SQLite | REQ-025 | ⬜ Planned |
| Scripts — history | REQ-026 | ⬜ Planned |
