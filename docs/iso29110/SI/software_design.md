# Software Design
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-DES-001 |
| **Version** | 0.1 (Draft) |
| **Last Updated** | 2026-03-02 |
| **Status** | 🟡 Draft — Pending requirements approval |

---

## 2. System Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Internal Network                       │
│  Clients (dev machines, CI/CD, agents)                   │
└──────────────────────┬───────────────────────────────────┘
                       │ HTTP
                       ▼
┌──────────────────────────────────────────────────────────┐
│  Rust API Gateway (Axum)              :3000              │
│  ┌──────────┬──────────┬──────────┬──────────┐          │
│  │ Auth     │ Router   │ Rate     │ Metrics  │          │
│  │ Module   │ Module   │ Limiter  │ Module   │          │
│  └──────────┴────┬─────┴──────────┴──────────┘          │
│                  │                                       │
│  ┌───────────────┼──────────────────────────┐           │
│  │ Health Check  │  Backend Pool            │           │
│  └───────────────┼──────────────────────────┘           │
│                  │                                       │
│     ┌────────────┼────────────┐                         │
│     ▼            ▼            ▼                         │
│  ┌────────┐ ┌─────────┐ ┌──────────┐                   │
│  │vllm-mlx│ │mistral. │ │mlx_lm    │                   │
│  │ :8000  │ │rs :8001 │ │.server   │                   │
│  │(Python)│ │ (Rust)  │ │:8080 (Py)│                   │
│  └────────┘ └─────────┘ └──────────┘                   │
│                                                          │
│  Mac Mini M4 Pro — 64GB RAM                             │
└──────────────────────────────────────────────────────────┘
```

---

## 3. Component Design

> จะเติมรายละเอียดเมื่อ requirements ได้รับอนุมัติ

### 3.1 API Gateway (Rust)
- **Framework**: TBD (Axum vs Actix-web)
- **Crate structure**: TBD
- **Modules**: auth, router, rate_limiter, health, metrics, proxy

### 3.2 Engine Adapters
- **vllm-mlx adapter**: HTTP client → `localhost:8000`
- **mistral.rs adapter**: HTTP client → `localhost:8001`
- **mlx_lm adapter**: HTTP client → `localhost:8080`

### 3.3 Configuration
- Environment variables (`.env`)
- YAML config (`config/`)

---

## 4. Data Flow

> จะเติม sequence diagrams เมื่อ design finalized

---

## 5. Requirement Traceability

| Design Component | Requirement IDs |
|:--|:--|
| Gateway — Auth Module | REQ-012 |
| Gateway — Router | REQ-010, REQ-011 |
| Gateway — Rate Limiter | REQ-013 |
| Gateway — Health Check | REQ-014 |
| Gateway — Streaming Proxy | REQ-015 |
| Gateway — Metrics | REQ-016 |
| Engine — vllm-mlx | REQ-001, REQ-004, REQ-005 |
| Engine — mistral.rs | REQ-002, REQ-004, REQ-005 |
| Engine — mlx_lm | REQ-003, REQ-004 |
| Scripts | REQ-020, REQ-021, REQ-022, REQ-023 |
