# Requirements Specification — Heimdall
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-REQ-001 |
| **Version** | 1.0 |
| **Last Updated** | 2026-03-03 |
| **Status** | ✅ Approved (Option A: Single Engine) |

---

## 2. Functional Requirements

### Engine Layer

| Req ID | Priority | Requirement | Status |
|:--|:--|:--|:--|
| REQ-001 | MUST | ระบบต้องรัน LLM inference ผ่าน **vllm-mlx** ได้ | 🟡 Implemented (untested on hw) |
| ~~REQ-002~~ | ~~MUST~~ | ~~mistral.rs support~~ | ❌ Cancelled (Option A) |
| ~~REQ-003~~ | ~~SHOULD~~ | ~~mlx_lm.server support~~ | ❌ Cancelled (Option A) |
| REQ-004 | MUST | Engine ต้อง expose **OpenAI-compatible API** | ✅ vllm-mlx provides this |
| REQ-005 | MUST | ต้องรองรับ **streaming** response (SSE) | ✅ Implemented in proxy.rs |

### API Gateway Layer (Heimdall)

| Req ID | Priority | Requirement | Status |
|:--|:--|:--|:--|
| REQ-010 | MUST | **Rust API Gateway** ทำ reverse proxy ไปยัง backend | ✅ proxy.rs |
| ~~REQ-011~~ | ~~MUST~~ | ~~Multi-engine routing~~ | ❌ Cancelled (single engine) |
| REQ-012 | MUST | **API key authentication** (Bearer token) | ✅ auth.rs (5 tests) |
| REQ-013 | SHOULD | **Rate limiting** | ⬜ Not yet |
| REQ-014 | MUST | **Health check** endpoint | ✅ health.rs |
| REQ-015 | MUST | **SSE streaming** proxy | ✅ proxy.rs |
| REQ-016 | SHOULD | **Metrics** endpoint (`/metrics`) | ✅ metrics_handler.rs |

### Operations

| Req ID | Priority | Requirement | Status |
|:--|:--|:--|:--|
| REQ-020 | MUST | Script **setup** environment | ✅ setup.sh |
| REQ-021 | MUST | Script **start/stop** server | ✅ start.sh, stop.sh |
| REQ-022 | MUST | **Benchmark** script with multi-model support | ✅ benchmark.sh + HTML report |
| REQ-023 | SHOULD | LAN clients ต้องเข้าถึง API ได้ | 🟡 Config ready (0.0.0.0 binding) |
| REQ-024 | SHOULD | **SemVer versioning** system | ✅ VERSION + version.sh |
| REQ-025 | SHOULD | เก็บผล benchmark ลง **SQLite** ทุกครั้งที่รัน | ⬜ Planned |
| REQ-026 | SHOULD | **Benchmark history** CLI เปรียบเทียบผลข้าม version | ⬜ Planned |

---

## 3. Non-Functional Requirements

| Req ID | Priority | Requirement | Target | Status |
|:--|:--|:--|:--|:--|
| NFR-001 | MUST | Gateway latency overhead | < 1ms | 🟡 Untested |
| NFR-002 | MUST | Memory: model + gateway + OS | ≤ 60GB | ✅ MoE ~20GB + gateway ~5MB |
| NFR-003 | SHOULD | Concurrent connections | ≥ 100 | 🟡 Untested |
| NFR-004 | MUST | TTFT | < 500ms | 🟡 Untested |
| NFR-005 | MUST | Unit test coverage | ≥ 80% | 🟡 9 tests (config + auth) |

---

## 4. Traceability Matrix

| Req ID | Design Component | Test Case | Status |
|:--|:--|:--|:--|
| REQ-001 | vllm-mlx backend | IT-001 | 🟡 |
| REQ-005 | proxy.rs (SSE) | IT-003, ST-003 | ✅ Implemented |
| REQ-010 | proxy.rs | IT-001 | ✅ Implemented |
| REQ-012 | auth.rs | UT-AUTH-001~005 | ✅ 5/5 Pass |
| REQ-014 | health.rs | UT-HC-001 | ✅ Implemented |
| REQ-016 | metrics_handler.rs | ST-004 | ✅ Implemented |
| REQ-020 | setup.sh | — | ✅ Done |
| REQ-021 | start.sh, stop.sh | — | ✅ Done |
| REQ-022 | benchmark.sh | BT-001~005 | ✅ Done |
| REQ-024 | VERSION, version.sh | — | ✅ Done |
| REQ-025 | data/heimdall.db | — | ⬜ Planned |
| REQ-026 | benchmark_history.sh | — | ⬜ Planned |
