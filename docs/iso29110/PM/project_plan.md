# Project Plan — LLM API Server
> ISO/IEC 29110 Basic Profile — Project Management Process

## 1. Project Overview

| Field | Value |
|:--|:--|
| **Project Name** | LLM API Server |
| **Project Manager** | — |
| **Start Date** | 2026-03-02 |
| **Target Completion** | TBD |
| **Status** | 🟡 In Planning |

## 2. Objectives

สร้าง LLM API Server สำหรับ internal network โดย:
- ใช้ MLX / vLLM เป็น inference engine บน Mac Mini M4 Pro 64GB
- เปรียบเทียบ Rust-based engine (mistral.rs / Candle) กับ Python-based (vllm-mlx)
- มี Rust API Gateway จัดการ traffic, auth, routing
- มี OpenAI-compatible API สำหรับ clients ภายใน

## 3. Scope

### In Scope
- [ ] Environment setup (Python + Rust)
- [ ] Primary inference engine deployment
- [ ] Rust API gateway development
- [ ] Benchmark framework
- [ ] API documentation
- [ ] ISO 29110 compliance documents

### Out of Scope
- Cloud deployment
- Fine-tuning / training
- Frontend chat UI (ขั้นแรก)

## 4. Resources

| Resource | Specification |
|:--|:--|
| Hardware | Mac Mini M4 Pro, 64GB RAM, 273 GB/s bandwidth |
| OS | macOS |
| Languages | Rust, Python, Shell |
| Key Frameworks | vllm-mlx, mistral.rs, Axum/Actix, MLX |

## 5. Work Breakdown Structure (WBS)

| ID | Task | Dependency | Est. Duration | Status |
|:--|:--|:--|:--|:--|
| WBS-001 | Project governance & standards setup | — | 1 day | 🟡 In Progress |
| WBS-002 | Requirements specification | WBS-001 | 1 day | ⬜ Not Started |
| WBS-003 | Software design | WBS-002 | 2 days | ⬜ Not Started |
| WBS-004 | Environment setup (Python engines) | WBS-003 | 1 day | ⬜ Not Started |
| WBS-005 | Environment setup (Rust engines) | WBS-003 | 2 days | ⬜ Not Started |
| WBS-006 | Rust API gateway development | WBS-003 | 5 days | ⬜ Not Started |
| WBS-007 | Benchmark framework | WBS-004, WBS-005 | 2 days | ⬜ Not Started |
| WBS-008 | Integration testing | WBS-006, WBS-007 | 2 days | ⬜ Not Started |
| WBS-009 | Documentation & release | WBS-008 | 1 day | ⬜ Not Started |

## 6. Risks

| ID | Risk | Impact | Probability | Mitigation |
|:--|:--|:--|:--|:--|
| RISK-001 | vllm-mlx ยังเป็น experimental อาจมี bug | Medium | Medium | มี fallback ไป mlx_lm.server |
| RISK-002 | mistral.rs model support จำกัด | Medium | Medium | ใช้ GGUF format ซึ่ง support กว้าง |
| RISK-003 | RAM ไม่พอสำหรับ model ใหญ่ + KV cache | High | Low | ใช้ 4-bit quantization, monitor memory |
| RISK-004 | mlx-rs bindings ยังไม่ stable | Low | High | ใช้ mistral.rs แทน (มี Candle + Metal) |

## 7. Milestones

| Milestone | Target Date | Deliverables |
|:--|:--|:--|
| M1: Planning Complete | TBD | Project plan, Requirements, Design |
| M2: Engine Comparison | TBD | Benchmark results, Engine selection |
| M3: Gateway MVP | TBD | Working Rust API gateway with routing |
| M4: Production Ready | TBD | Full system, docs, tests |
