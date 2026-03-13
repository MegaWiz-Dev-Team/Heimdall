# Software Design — Heimdall
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-DES-001 |
| **Version** | 2.0 |
| **Last Updated** | 2026-03-13 |
| **Status** | ✅ Updated for v0.4.0 |

---

## 2. System Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Internal Network (LAN)                 │
│  Clients: Mimir, Bifrost, AI agents, dev machines        │
└──────────────────────┬───────────────────────────────────┘
                       │ HTTP
                       ▼
┌──────────────────────────────────────────────────────────┐
│  Heimdall Gateway (Rust/Axum)              :3000         │
│  ┌──────────┬──────────┬──────────┬──────────┐          │
│  │ Auth     │ Proxy    │ Health   │ Metrics  │          │
│  │ (Bearer) │(/v1→/)   │ Check    │(Prometheus│          │
│  └──────────┴────┬─────┴──────────┴──────────┘          │
│                  │                                       │
│  Endpoints:                                              │
│    /v1/chat/completions  → proxy to backend              │
│    /v1/models            → proxy to backend              │
│    /health, /ready       → health.rs                     │
│    /metrics              → Prometheus                    │
│    /api-spec             → OpenAPI 3.1 JSON              │
│    /docs                 → Scalar UI                     │
│    /                     → root info                     │
│                  │                                       │
│                  ▼                                       │
│  ┌──────────────────────────┐  ┌────────────────┐       │
│  │ MLX Backend       :8000  │  │ Embedding :8001│       │
│  │ • mlx_vlm.server        │  │ mlx-embedding  │       │
│  │ • mlx_lm.server         │  │ FastAPI        │       │
│  │ • llama.cpp              │  └────────────────┘       │
│  │ Active: Qwen3.5-27B-4bit│                            │
│  └──────────────────────────┘                            │
│                                                          │
│  ┌──────────────────────────┐                            │
│  │ SQLite: heimdall.db      │ ← benchmark persistence   │
│  └──────────────────────────┘                            │
│                                                          │
│  Mac Mini M4 Pro — 64GB Unified Memory                   │
└──────────────────────────────────────────────────────────┘
```

---

## 3. Component Design

### 3.1 Gateway (`gateway/src/`)

| Module | File | Purpose | Req IDs |
|:--|:--|:--|:--|
| **Main** | `main.rs` | Axum server, middleware stack, routing | — |
| **Config** | `config.rs` | Load .env settings (4 tests) | — |
| **Auth** | `auth.rs` | Bearer token validation, bypass /health (5 tests) | REQ-012 |
| **Proxy** | `proxy.rs` | Reverse proxy, SSE streaming, /api-spec, /docs | REQ-010, REQ-015, REQ-017, REQ-018 |
| **Health** | `health.rs` | `/health` + `/ready` — probes backend | REQ-014 |
| **Metrics** | `metrics_handler.rs` | Prometheus `/metrics` | REQ-016 |

### 3.2 Backend Engines

| Engine | Port | API | Models |
|:--|:--|:--|:--|
| mlx_vlm.server (primary) | :8000 | OpenAI-compatible | Qwen3.5-27B-4bit, MedGemma-4B |
| mlx_lm.server | :8000 | OpenAI-compatible | Text-only models |
| llama.cpp (llama-server) | :8000 | OpenAI-compatible | GGUF models |

### 3.3 Embedding Server

| Component | Details |
|:--|:--|
| Script | `scripts/embedding_server.py` |
| Framework | FastAPI + mlx-embedding-models |
| Port | :8001 |
| API | `/v1/embeddings` (OpenAI-compatible) |
| Model | mlx-community/bge-small-en-v1.5 |

### 3.4 Scripts (`scripts/`)

| Script | Purpose |
|:--|:--|
| `setup.sh` | Install Rust, Python, vllm-mlx, build gateway |
| `start.sh` | Start backend + gateway, manage PIDs, readiness check |
| `stop.sh` | Graceful shutdown via PID files |
| `health_check.sh` | Probe gateway + backend health |
| `benchmark.sh` | Multi-model benchmark (--models, --all, --runs) |
| `generate_report.sh` | Generate HTML dashboard from benchmark JSON |
| `report_template.py` | Python HTML report renderer |
| `version.sh` | SemVer management (show/bump/set/tag/release) |
| `model_manager.sh` | Archive/restore models (internal ↔ external SSD) |
| `embedding_server.py` | FastAPI embedding server |

### 3.5 Data Layer

| Store | Location | Purpose |
|:--|:--|:--|
| **SQLite** | `data/heimdall.db` | Benchmark persistence |
| **JSON** | `reports/*.json` | Per-run benchmark results |
| **HTML** | `reports/*.html` | Visual benchmark dashboards |

Schema:

| Table | Columns |
|:--|:--|
| `runs` | id, timestamp, version, git_commit, hardware_json |
| `results` | id, run_id, model, test_type, tps_avg, ttfb_avg, tokens_avg, memory_mb |

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
    ├─ /health, /ready → health.rs  → probe backend → JSON
    ├─ /metrics        → metrics.rs → Prometheus text
    ├─ /api-spec       → proxy.rs   → OpenAPI 3.1 JSON
    ├─ /docs           → proxy.rs   → Scalar UI
    ├─ /v1/*           → proxy.rs   → forward to backend
    │                     ├─ JSON response → return
    │                     └─ SSE stream → pipe through
    └─ /               → proxy.rs   → {"name":"Heimdall",...}
```

---

## 5. Requirement Traceability

| Design Component | Requirement IDs | Status |
|:--|:--|:--|
| Gateway — auth.rs | REQ-012 | ✅ 5 tests |
| Gateway — proxy.rs | REQ-010, REQ-015, REQ-017, REQ-018 | ✅ |
| Gateway — health.rs | REQ-014 | ✅ |
| Gateway — metrics_handler.rs | REQ-016 | ✅ |
| Engine — mlx_vlm.server | REQ-001, REQ-004, REQ-005 | ✅ |
| Engine — llama.cpp | REQ-001b | ✅ |
| Embedding — FastAPI | REQ-006, REQ-032 | ✅ |
| Scripts — operations | REQ-020, REQ-021, REQ-023 | ✅ |
| Scripts — benchmark | REQ-022, REQ-028 | ✅ |
| Scripts — versioning | REQ-024 | ✅ |
| Data — SQLite | REQ-025 | ✅ |
| Data — history | REQ-026 | ✅ |
| MedGemma | REQ-029 | ✅ |
| Mimir integration | REQ-030, REQ-031 | ✅ |
