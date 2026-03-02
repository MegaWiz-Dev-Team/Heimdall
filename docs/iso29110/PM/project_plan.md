# Project Plan — Heimdall 🛡️
> ISO/IEC 29110 Basic Profile — Project Management Process

## 1. Project Overview

| Field | Value |
|:--|:--|
| **Project Name** | Heimdall — LLM API Server |
| **Version** | 0.1.0 |
| **Start Date** | 2026-03-02 |
| **Target Completion** | 2026-03-16 |
| **Status** | 🟢 Implementation Phase |
| **Sprint Duration** | ~3 days |

## 2. Objectives

สร้าง Internal LLM API Server (Heimdall) บน Mac Mini M4 Pro 64GB:
- ใช้ **vllm-mlx** เป็น inference engine (Metal GPU acceleration)
- มี **Rust API Gateway (Axum)** จัดการ auth, proxy, metrics
- มี **OpenAI-compatible API** สำหรับ clients ภายใน LAN
- มี **Benchmark suite** พร้อม visual HTML report รองรับ multi-model/multi-type

## 3. Scope

### In Scope
- [x] Environment setup (Python + Rust)
- [x] vllm-mlx inference engine deployment
- [x] Rust API gateway (Axum) — auth, proxy, health, metrics
- [x] Operation scripts (setup, start, stop, health check)
- [x] Multi-model benchmark suite + HTML report
- [x] SemVer versioning system
- [x] ISO 29110 compliance documents
- [x] Model storage management (internal/external SSD)
- [ ] SQLite benchmark persistence
- [ ] Multi-type benchmark (Embedding/Reranker)
- [ ] Integration testing on hardware

### Out of Scope
- Cloud deployment
- Fine-tuning / training
- Frontend chat UI
- Docker/K3s containers (analyzed → not beneficial for single Mac)

## 4. Resources

| Resource | Specification |
|:--|:--|
| Hardware | Mac Mini M4 Pro, 64GB RAM, 273 GB/s bandwidth |
| Storage | Internal SSD 460GB (385GB free) + External T7 Shield 1.8TB |
| OS | macOS |
| Languages | Rust, Python, Shell |
| Key Frameworks | vllm-mlx, Axum (Tokio), MLX |

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

### ⬜ Sprint 3 — Go Live (เป้าหมาย: 2026-03-04 ~ 06)

> เป้าหมาย: Start server จริง + benchmark บน hardware จริง

| ID | Task | Priority | Est. |
|:--|:--|:--|:--|
| WBS-012 | Download model + start vllm-mlx + Heimdall gateway | 🔴 HIGH | 1 hr |
| WBS-011a | Integration test: gateway → vllm-mlx (chat completion) | 🔴 HIGH | 2 hr |
| WBS-011b | Integration test: SSE streaming through gateway | 🔴 HIGH | 1 hr |
| WBS-011c | API smoke tests: /v1/models, /v1/chat/completions | 🟡 MED | 1 hr |
| WBS-011d | LAN connectivity test (from another machine) | 🟡 MED | 30 min |
| WBS-012b | Run real benchmark (--all models) | 🟡 MED | 2 hr |

**Definition of Done**: Server running, API responds, benchmark report generated from real hardware

---

### ⬜ Sprint 4 — Data & Persistence (เป้าหมาย: 2026-03-07 ~ 09)

> เป้าหมาย: เก็บผล benchmark ลง SQLite + เปรียบเทียบ version

| ID | Task | Priority | Est. |
|:--|:--|:--|:--|
| WBS-013a | Create SQLite schema (runs + results tables) | 🟡 MED | 2 hr |
| WBS-013b | Update benchmark.sh → insert results to SQLite | 🟡 MED | 3 hr |
| WBS-013c | Create benchmark_history.sh (query, compare versions) | 🟡 MED | 3 hr |
| WBS-013d | Update HTML report → read from SQLite | 🟢 LOW | 2 hr |

**Definition of Done**: Benchmark auto-saves to DB, history queryable, version comparison works

---

### ⬜ Sprint 5 — Multi-Type Benchmark (เป้าหมาย: 2026-03-10 ~ 12)

> เป้าหมาย: Benchmark embedding + reranker models

| ID | Task | Priority | Est. |
|:--|:--|:--|:--|
| WBS-015a | Embedding benchmark script (encode/s, batch, long-text) | 🟡 MED | 4 hr |
| WBS-015b | Reranker benchmark script (pairs/s, accuracy) | 🟡 MED | 4 hr |
| WBS-015c | Update report_template.py → multi-type sections | 🟡 MED | 4 hr |
| WBS-015d | Run benchmarks: BGE-M3, Jina, MiniLM + rerankers | 🟢 LOW | 2 hr |

**Definition of Done**: HTML report with LLM + Embedding + Reranker sections, all benchmarked

---

### ⬜ Sprint 6 — Polish & Release v0.2.0 (เป้าหมาย: 2026-03-13 ~ 16)

> เป้าหมาย: Release v0.2.0 พร้อม production

| ID | Task | Priority | Est. |
|:--|:--|:--|:--|
| WBS-016 | Update README with final architecture + usage guide | 🟡 MED | 2 hr |
| WBS-017 | Update ISO 29110 docs — final test report + release notes | 🟡 MED | 2 hr |
| WBS-018 | Rate limiting (REQ-013) — optional | 🟢 LOW | 4 hr |
| WBS-019 | Version bump → v0.2.0, git tag, release | 🟡 MED | 30 min |

**Definition of Done**: v0.2.0 tagged, all docs updated, production deployment complete

---

## 6. Sprint Summary

| Sprint | Duration | Focus | Status |
|:--|:--|:--|:--|
| **Sprint 0** | 1 day | Foundation + Architecture | ✅ Done |
| **Sprint 1** | 1 day | Gateway MVP | ✅ Done |
| **Sprint 2** | 1 day | Multi-Model + Tooling | ✅ Done |
| **Sprint 3** | 3 days | Go Live + Real Benchmark | ⬜ Next |
| **Sprint 4** | 3 days | SQLite Persistence | ⬜ Planned |
| **Sprint 5** | 3 days | Multi-Type Benchmark | ⬜ Planned |
| **Sprint 6** | 3 days | Polish + Release v0.2.0 | ⬜ Planned |

---

## 7. Risks

| ID | Risk | Impact | Probability | Mitigation |
|:--|:--|:--|:--|:--|
| RISK-001 | vllm-mlx experimental → bugs | Medium | Medium | Fallback to mlx_lm.server |
| RISK-003 | RAM ไม่พอสำหรับ model ใหญ่ | High | Low | ใช้ MoE 4-bit (~20GB) → เหลือ ~28GB สำหรับ KV cache |
| RISK-005 | Docker/K3s ไม่ได้ Metal GPU | Medium | — | ✅ Mitigated: ตัดสินใจใช้ bare-metal |
| RISK-006 | External SSD ถอดระหว่าง inference | High | Low | model_manager.sh ย้าย active model ไป internal ก่อน serve |

## 8. Milestones

| Milestone | Sprint | Target Date | Status | Deliverables |
|:--|:--|:--|:--|:--|
| M1: Planning Complete | S0 | 2026-03-02 | ✅ Done | Project plan, Requirements, Design |
| M2: Gateway MVP | S1 | 2026-03-02 | ✅ Done | Rust gateway, scripts, 9 tests |
| M3: Benchmark Suite | S2 | 2026-03-03 | ✅ Done | Multi-model benchmark + HTML report |
| M4: Production Ready | S3 | 2026-03-06 | ⬜ | Running server, real benchmarks |
| M5: Data Persistence | S4 | 2026-03-09 | ⬜ | SQLite + history CLI |
| M6: Release v0.2.0 | S6 | 2026-03-16 | ⬜ | Full release, docs, tagged |
