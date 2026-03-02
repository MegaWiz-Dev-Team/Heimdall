# Project Plan — Heimdall 🛡️
> ISO/IEC 29110 Basic Profile — Project Management Process

## 1. Project Overview

| Field | Value |
|:--|:--|
| **Project Name** | Heimdall — LLM API Server |
| **Version** | 0.1.0 |
| **Start Date** | 2026-03-02 |
| **Target Completion** | 2026-03-10 |
| **Status** | 🟢 Implementation Phase |

## 2. Objectives

สร้าง Internal LLM API Server (Heimdall) บน Mac Mini M4 Pro 64GB:
- ใช้ **vllm-mlx** เป็น inference engine (Metal GPU acceleration)
- มี **Rust API Gateway (Axum)** จัดการ auth, proxy, metrics
- มี **OpenAI-compatible API** สำหรับ clients ภายใน LAN
- มี **Benchmark suite** พร้อม visual HTML report รองรับ multi-model

## 3. Scope

### In Scope
- [x] Environment setup (Python + Rust)
- [x] vllm-mlx inference engine deployment
- [x] Rust API gateway (Axum) — auth, proxy, health, metrics
- [x] Operation scripts (setup, start, stop, health check)
- [x] Multi-model benchmark suite + HTML report
- [x] SemVer versioning system
- [x] ISO 29110 compliance documents

### Out of Scope
- Cloud deployment
- Fine-tuning / training
- Frontend chat UI
- Docker/K3s containers (analyzed → not beneficial for single Mac)

## 4. Resources

| Resource | Specification |
|:--|:--|
| Hardware | Mac Mini M4 Pro, 64GB RAM, 273 GB/s bandwidth |
| OS | macOS |
| Languages | Rust, Python, Shell |
| Key Frameworks | vllm-mlx, Axum (Tokio), MLX |

## 5. Work Breakdown Structure (WBS)

| ID | Task | Status | Commit |
|:--|:--|:--|:--|
| WBS-001 | Project governance & ISO 29110 setup | ✅ Done | `[WBS-001]` |
| WBS-002 | Architecture research & analysis | ✅ Done | — |
| WBS-003 | Architecture decision (Option A) | ✅ Done | MR-002 |
| WBS-004 | Container analysis (Docker/K3s) | ✅ Done | Decided: bare-metal |
| WBS-006 | Rust API Gateway (Axum) | ✅ Done | `[WBS-006]` |
| WBS-007 | Benchmark suite + SemVer versioning | ✅ Done | `[WBS-007]` |
| WBS-008 | Project rename to Heimdall | ✅ Done | `[WBS-008]` |
| WBS-009 | Multi-model benchmark support | ✅ Done | `[WBS-009]` |
| WBS-010 | ISO 29110 documentation update | 🟡 In Progress | — |
| WBS-011 | Integration testing | ⬜ Not Started | — |
| WBS-012 | Production deployment & benchmark | ⬜ Not Started | — |

## 6. Risks

| ID | Risk | Impact | Probability | Mitigation |
|:--|:--|:--|:--|:--|
| RISK-001 | vllm-mlx experimental → bugs | Medium | Medium | Fallback to mlx_lm.server |
| RISK-003 | RAM ไม่พอสำหรับ model ใหญ่ | High | Low | ใช้ MoE 4-bit (~20GB) → เหลือ ~28GB สำหรับ KV cache |
| RISK-005 | Docker/K3s ไม่ได้ Metal GPU | Medium | — | ✅ Mitigated: ตัดสินใจใช้ bare-metal |

## 7. Milestones

| Milestone | Target Date | Status | Deliverables |
|:--|:--|:--|:--|
| M1: Planning Complete | 2026-03-02 | ✅ Done | Project plan, Requirements, Design |
| M2: Gateway MVP | 2026-03-02 | ✅ Done | Rust gateway, scripts, 9 tests passing |
| M3: Benchmark Suite | 2026-03-03 | ✅ Done | Multi-model benchmark + HTML report |
| M4: Production Ready | TBD | ⬜ | Full integration test, deployment |
