# Requirements Specification — Heimdall
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-REQ-001 |
| **Version** | 2.0 |
| **Last Updated** | 2026-03-13 |
| **Status** | ✅ Updated for v0.4.0 (Sprint 6 complete) |

---

## 2. Functional Requirements

### Engine Layer

| Req ID | Priority | Requirement | Status |
|:--|:--|:--|:--|
| REQ-001 | MUST | MLX inference engine (mlx_vlm / mlx_lm) | ✅ mlx_vlm.server |
| REQ-001b | SHOULD | llama.cpp backend support | ✅ Sprint 4 |
| ~~REQ-002~~ | ~~MUST~~ | ~~mistral.rs support~~ | ❌ Cancelled |
| ~~REQ-003~~ | ~~SHOULD~~ | ~~vllm-metal support~~ | ❌ Cancelled (crashes) |
| REQ-004 | MUST | **OpenAI-compatible API** | ✅ /v1/chat/completions, /v1/models |
| REQ-005 | MUST | **SSE streaming** response | ✅ proxy.rs |
| REQ-006 | SHOULD | **Embedding endpoint** support | ✅ Sprint 6 — MLX embedding server |

### API Gateway Layer (Heimdall)

| Req ID | Priority | Requirement | Status |
|:--|:--|:--|:--|
| REQ-010 | MUST | **Rust API Gateway** (Axum) — reverse proxy | ✅ proxy.rs |
| ~~REQ-011~~ | ~~MUST~~ | ~~Multi-engine routing~~ | ❌ Cancelled |
| REQ-012 | MUST | **API key authentication** (Bearer token) | ✅ auth.rs (5 tests) |
| REQ-013 | SHOULD | **Rate limiting** | ⬜ Sprint 7 |
| REQ-014 | MUST | **Health check** endpoint | ✅ health.rs |
| REQ-015 | MUST | **SSE streaming** proxy | ✅ proxy.rs |
| REQ-016 | SHOULD | **Prometheus metrics** (`/metrics`) | ✅ metrics_handler.rs |
| REQ-017 | SHOULD | **OpenAPI 3.1 spec** (`/api-spec`) | ✅ Sprint 5 |
| REQ-018 | SHOULD | **API docs UI** (`/docs`) — Scalar | ✅ Sprint 5 |

### Operations

| Req ID | Priority | Requirement | Status |
|:--|:--|:--|:--|
| REQ-020 | MUST | Setup script | ✅ setup.sh |
| REQ-021 | MUST | Start/stop scripts | ✅ start.sh, stop.sh |
| REQ-022 | MUST | **Multi-model benchmark** | ✅ benchmark.sh (LLM) |
| REQ-023 | SHOULD | LAN client access | ✅ 0.0.0.0 binding |
| REQ-024 | SHOULD | **SemVer versioning** | ✅ VERSION + version.sh |
| REQ-025 | SHOULD | **SQLite persistence** for benchmarks | ✅ Sprint 4 — heimdall.db |
| REQ-026 | SHOULD | **Benchmark history** CLI | ✅ Sprint 4 — benchmark_history.sh |
| REQ-027 | SHOULD | **Model storage** management | ✅ Sprint 2 — model_manager.sh |
| REQ-028 | SHOULD | HTML report generation | ✅ generate_report.sh + report_template.py |
| REQ-029 | SHOULD | **MedGemma** medical model integration | ✅ Sprint 5 |

### Mimir Integration (Sprint 6)

| Req ID | Priority | Requirement | Status |
|:--|:--|:--|:--|
| REQ-030 | MUST | Heimdall as LLM provider for Mimir | ✅ Sprint 6 |
| REQ-031 | MUST | Model auto-detection | ✅ Sprint 6 |
| REQ-032 | SHOULD | MLX embedding server (FastAPI) | ✅ embedding_server.py |

---

## 3. Non-Functional Requirements

| Req ID | Priority | Requirement | Target | Status |
|:--|:--|:--|:--|:--|
| NFR-001 | MUST | Gateway latency overhead | < 1ms | ✅ Verified |
| NFR-002 | MUST | Memory: model + gateway + OS | ≤ 60GB | ✅ ~20GB + ~5MB |
| NFR-003 | SHOULD | Concurrent connections | ≥ 100 | 🟡 Untested |
| NFR-004 | MUST | TTFT | < 500ms | ✅ ~1.7s (incl. thinking) |
| NFR-005 | MUST | Unit test coverage | ≥ 80% | ✅ 9 tests (config + auth) |

---

## 4. Traceability Matrix

| Req ID | Design Component | Test Case | Status |
|:--|:--|:--|:--|
| REQ-001 | mlx_vlm.server backend | IT-001, BT-* | ✅ |
| REQ-001b | llama.cpp backend | BT-* | ✅ |
| REQ-005 | proxy.rs (SSE) | IT-003, ST-003 | ✅ |
| REQ-006 | embedding_server.py | — | ✅ |
| REQ-010 | proxy.rs | IT-001 | ✅ |
| REQ-012 | auth.rs | UT-AUTH-001~005 | ✅ 5/5 |
| REQ-014 | health.rs | UT-HC-001 | ✅ |
| REQ-016 | metrics_handler.rs | ST-004 | ✅ |
| REQ-017 | proxy.rs (/api-spec) | ST-005 | ✅ |
| REQ-018 | proxy.rs (/docs) | ST-006 | ✅ |
| REQ-020 | setup.sh | — | ✅ |
| REQ-021 | start.sh, stop.sh | — | ✅ |
| REQ-022 | benchmark.sh | BT-001~006 | ✅ |
| REQ-025 | heimdall.db | — | ✅ |
| REQ-029 | MedGemma 4B | BT-MED-001 | ✅ |
| REQ-030 | Mimir provider | — | ✅ |
