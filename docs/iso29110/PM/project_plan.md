# Project Plan — Heimdall 🛡️
> ISO/IEC 29110 Basic Profile — Project Management Process

## 1. Project Overview

| Field | Value |
|:--|:--|
| **Project Name** | Heimdall — LLM API Gateway |
| **Version** | 0.4.0 |
| **Start Date** | 2026-03-02 |
| **Status** | 🟢 Production |
| **Sprint Duration** | ~3 days |
| **Part of** | Asgard AI Platform ecosystem |

## 2. Objectives

สร้าง LLM API Gateway (Heimdall) บน Apple Silicon + NVIDIA:
- ใช้ **MLX** เป็น native inference engine (Apple Silicon)
- ใช้ **llama.cpp** เป็น alternative backend
- ใช้ **Ollama** เป็น multi-model backend
- มี **Rust API Gateway (Axum)** จัดการ auth, proxy, metrics
- มี **OpenAI-compatible API** สำหรับ clients
- มี **Benchmark suite** พร้อม visual HTML report
- **Part of Asgard ecosystem** — serves as LLM provider for Mimir, Bifrost

## 3. Scope

### In Scope
- [x] Environment setup (Python + Rust)
- [x] MLX inference engine deployment
- [x] llama.cpp backend support
- [x] Ollama backend support
- [x] Rust API gateway (Axum) — auth, proxy, health, metrics
- [x] Operation scripts (setup, start, stop, health check)
- [x] Multi-model benchmark suite + HTML report
- [x] SemVer versioning system
- [x] ISO 29110 compliance documents
- [x] Model storage management (internal/external SSD)
- [x] SQLite benchmark persistence
- [x] API documentation (OpenAPI 3.1 + Scalar UI)
- [x] MedGemma medical model integration
- [ ] vLLM backend (NVIDIA GPU)
- [ ] Prometheus metrics export
- [ ] Rate limiting per tenant

### Out of Scope
- Cloud deployment
- Fine-tuning / training
- Frontend chat UI
- Docker/K3s containers (analyzed → not beneficial for single Mac)

## 4. Resources

| Resource | Specification |
|:--|:--|
| Hardware | Mac Mini M4 Pro, 64GB RAM, 273 GB/s bandwidth |
| Storage | Internal SSD 460GB + External T7 Shield 1.8TB |
| OS | macOS |
| Languages | Rust, Python, Shell |
| Key Frameworks | MLX, llama.cpp, Axum (Tokio) |

---

## 5. Sprint Plan

### ✅ Sprint 0 — Foundation (2026-03-02) `DONE`

> เป้าหมาย: วางรากฐานโปรเจ็ค + ตัดสินใจ architecture

| ID | Task | Status |
|:--|:--|:--|
| WBS-001 | Project governance & ISO 29110 setup | ✅ |
| WBS-002 | Architecture research & analysis | ✅ |
| WBS-003 | Architecture decision (Option A) | ✅ |
| WBS-004 | Container analysis (Docker/K3s → bare-metal) | ✅ |

**Deliverables**: ISO 29110 docs, Architecture decision (MR-002)

---

### ✅ Sprint 1 — Gateway MVP (2026-03-02 ~ 03) `DONE`

> เป้าหมาย: Rust Gateway พร้อมใช้ + Operation scripts

| ID | Task | Status |
|:--|:--|:--|
| WBS-006 | Rust API Gateway (Axum) — auth, proxy, health, metrics | ✅ |
| WBS-007 | Benchmark suite + SemVer versioning | ✅ |
| WBS-008 | Project rename to Heimdall 🛡️ | ✅ |

**Deliverables**: Gateway 9/9 tests, scripts, VERSION, version.sh

---

### ✅ Sprint 2 — Multi-Model & Tooling (2026-03-03) `DONE`

> เป้าหมาย: Benchmark หลาย model + จัดการ model storage

| ID | Task | Status |
|:--|:--|:--|
| WBS-009 | Multi-model benchmark support (--models, --all) | ✅ |
| WBS-010 | ISO 29110 documentation update | ✅ |
| WBS-014 | Model storage management (T7 Shield SSD) | ✅ |

**Deliverables**: Multi-model HTML report, model_manager.sh, model_catalog.md

---

### ✅ Sprint 3 — Go Live (2026-03-03) `DONE`

> เป้าหมาย: Start server จริง + benchmark บน hardware จริง

| ID | Task | Status |
|:--|:--|:--|
| WBS-012 | Download model + start server | ✅ |
| WBS-011a | Integration test: chat completion | ✅ |
| WBS-011b | Integration test: SSE streaming | ✅ |
| WBS-011c | API smoke tests (models, chat, metrics) | ✅ |
| WBS-011d | LAN connectivity test | ✅ |

**Results**: 6/6 integration tests pass, 9/9 unit tests pass
**Model**: Qwen3.5-27B-4bit — 16.4 TPS, 16.2 GB peak RAM

---

### ✅ Sprint 4 — Multi-Engine & Persistence (2026-03-03 ~ 04) `DONE`

> เป้าหมาย: llama.cpp backend + SQLite persistence + benchmark comparison

| ID | Task | Status |
|:--|:--|:--|
| WBS-013a | llama.cpp backend integration | ✅ |
| WBS-013b | SQLite benchmark persistence (runs + results tables) | ✅ |
| WBS-013c | Benchmark history CLI (query, compare versions) | ✅ |
| WBS-013d | Multi-engine HTML report (MLX vs llama.cpp) | ✅ |

**Deliverables**: Dual-engine support, SQLite persistence, comparison reports

---

### ✅ Sprint 5 — API Docs & MedGemma (2026-03-03 ~ 04) `DONE`

> เป้าหมาย: OpenAPI docs + medical model benchmarks

| ID | Task | Status |
|:--|:--|:--|
| WBS-015a | OpenAPI 3.1 spec (`/api-spec`) | ✅ |
| WBS-015b | Scalar UI docs (`/docs`) | ✅ |
| WBS-015c | MedGemma 4B model integration | ✅ |
| WBS-015d | MedGemma benchmark (medical Q&A) | ✅ |

**Deliverables**: API docs at /docs, MedGemma benchmarks, v0.4.0 release notes

---

### ✅ Sprint 6 — Mimir Integration (2026-03-04) `DONE`

> เป้าหมาย: Heimdall เป็น LLM provider สำหรับ Mimir

| ID | Task | Status |
|:--|:--|:--|
| WBS-016 | Mimir Heimdall provider implementation | ✅ |
| WBS-017 | Model auto-detection | ✅ |
| WBS-018 | Embedding endpoint support | ✅ |
| WBS-019 | MLX embedding server (FastAPI) | ✅ |

**Deliverables**: Mimir Sprint 15 integration, 5 models available via Heimdall

---

### ⬜ Sprint 7 — vLLM & Production Hardening (Target: 2026-04)

> เป้าหมาย: NVIDIA GPU support + production features

| ID | Task | Priority | Est. |
|:--|:--|:--|:--|
| WBS-020 | vLLM backend integration (NVIDIA) | 🔴 HIGH | 4 hr |
| WBS-021 | Prometheus metrics export | 🟡 MED | 3 hr |
| WBS-022 | Rate limiting per tenant | 🟡 MED | 3 hr |
| WBS-023 | JWT validation (Yggdrasil) | 🟡 MED | 2 hr |
| WBS-024 | Health check improvements | 🟢 LOW | 1 hr |

---

### ✅ Sprint 8 — Open-Source Hardening (2026-04-02) `DONE`

> เป้าหมาย: Prepare repository for public release (Security + Tooling)

| ID | Task | Status |
|:--|:--|:--|
| WBS-025a | Fix `benchmark.sh` parsing and authorization flaws | ✅ |
| WBS-025b | Remove personal Mac SSD paths from conversions | ✅ |
| WBS-025c | Create robust Hugging Face `upload_to_hf.py` script | ✅ |
| WBS-025d | Benchmark `Qwen3.5-27B-Opus-Reasoning` & update Docs | ✅ |
| WBS-025e | Establish `CHANGELOG.md` and bump `v0.2.0` | ✅ |

**Deliverables**: Clean scripts, comprehensive documentation updates, Public release ready.

---

### ⬜ Sprint 9 — Native MLX Rust Backend (Target: Q3 2026)

> เป้าหมาย: ทดแทน Python Engine ด้วย `mlx-rs` เพื่อลด Memory footprint และสร้าง Single Binary Gateway

| ID | Task | Priority | Est. |
|:--|:--|:--|:--|
| WBS-026 | Research and prototype `mlx-rs` binding | 🟡 MED | 4 hr |
| WBS-027 | Integrate Rust engine natively into Heimdall proxy | 🔴 HIGH | 8 hr |
| WBS-028 | Single binary deployment pipeline | 🟡 MED | 3 hr |

---

## 6. Sprint Summary

| Sprint | Duration | Focus | Status |
|:--|:--|:--|:--|
| **Sprint 0** | 1 day | Foundation + Architecture | ✅ Done |
| **Sprint 1** | 1 day | Gateway MVP | ✅ Done |
| **Sprint 2** | 1 day | Multi-Model + Tooling | ✅ Done |
| **Sprint 3** | 1 day | Go Live + Integration Tests | ✅ Done |
| **Sprint 4** | 1 day | Multi-Engine + Persistence | ✅ Done |
| **Sprint 5** | 1 day | API Docs + MedGemma | ✅ Done |
| **Sprint 6** | 1 day | Mimir Integration | ✅ Done |
| **Sprint 7** | TBD | vLLM + Production | ⬜ Planned |
| **Sprint 8** | 1 day | Open-Source Hardening | ✅ Done |
| **Sprint 9** | TBD | Native MLX Rust (`mlx-rs`) | ⬜ Planned |

---

## 7. Risks

| ID | Risk | Impact | Probability | Mitigation |
|:--|:--|:--|:--|:--|
| RISK-001 | vllm-mlx experimental → bugs | Medium | **Triggered** | ✅ Mitigated: mlx_vlm.server fallback |
| RISK-003 | RAM ไม่พอสำหรับ model ใหญ่ | High | Low | MoE 4-bit (~20GB) → เหลือ ~28GB สำหรับ KV cache |
| RISK-005 | Docker ไม่ได้ Metal GPU | Medium | — | ✅ Mitigated: bare-metal decision |
| RISK-006 | External SSD ถอดระหว่าง inference | High | Low | model_manager.sh ย้าย active model ไป internal |
| RISK-007 | vLLM NVIDIA driver issues | Medium | Medium | Test on DGX Spark first; fallback to Ollama |

## 8. Milestones

| Milestone | Sprint | Target Date | Status |
|:--|:--|:--|:--|
| M1: Planning Complete | S0 | 2026-03-02 | ✅ Done |
| M2: Gateway MVP | S1 | 2026-03-02 | ✅ Done |
| M3: Benchmark Suite | S2 | 2026-03-03 | ✅ Done |
| M4: Server Running | S3 | 2026-03-03 | ✅ Done |
| M5: Multi-Engine | S4 | 2026-03-04 | ✅ Done |
| M6: API Docs + MedGemma | S5 | 2026-03-04 | ✅ Done |
| M7: Mimir Integration | S6 | 2026-03-04 | ✅ Done |
| M8: vLLM + Production | S7 | 2026-04 | ⬜ Planned |
| M9: Open-Source Status | S8 | 2026-04-02 | ✅ Done |
| M10: Native MLX Rust | S9 | Q3 2026 | ⬜ Planned |
