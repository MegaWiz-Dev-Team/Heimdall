# Test Cases — Heimdall
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-TST-001 |
| **Version** | 2.0 |
| **Last Updated** | 2026-03-13 |
| **Status** | ✅ Updated for v0.4.0 |

---

## 2. Unit Tests

### Gateway — Config Module (`config.rs`)

| Test ID | Requirement | Description | Status |
|:--|:--|:--|:--|
| UT-CFG-001 | — | Default config values (host, ports) | ✅ Pass |
| UT-CFG-002 | — | API keys parsing (comma-separated) | ✅ Pass |
| UT-CFG-003 | — | Empty API keys (auth disabled) | ✅ Pass |
| UT-CFG-004 | — | Backend URL construction | ✅ Pass |

### Gateway — Auth Module (`auth.rs`)

| Test ID | Requirement | Description | Status |
|:--|:--|:--|:--|
| UT-AUTH-001 | REQ-012 | No auth configured → pass all | ✅ Pass |
| UT-AUTH-002 | REQ-012 | Valid API key → pass | ✅ Pass |
| UT-AUTH-003 | REQ-012 | Invalid API key → 401 | ✅ Pass |
| UT-AUTH-004 | REQ-012 | Missing Authorization header → 401 | ✅ Pass |
| UT-AUTH-005 | REQ-012 | Health endpoint bypasses auth | ✅ Pass |

---

## 3. Integration Tests

| Test ID | Requirement | Description | Status |
|:--|:--|:--|:--|
| IT-001 | REQ-001, REQ-010 | Client → Heimdall → backend (chat completion) | ✅ Pass |
| IT-003 | REQ-015 | SSE streaming through gateway | ✅ Pass |

---

## 4. API Smoke Tests

| Test ID | Requirement | Command | Status |
|:--|:--|:--|:--|
| ST-001 | REQ-004 | `GET /v1/models` → 200 + model list | ✅ Pass |
| ST-002 | REQ-004 | `POST /v1/chat/completions` → 200 + AI response | ✅ Pass |
| ST-003 | REQ-005 | `POST /v1/chat/completions` stream=true → SSE | ✅ Pass |
| ST-004 | REQ-016 | `GET /metrics` → Prometheus format | ✅ Pass |
| ST-005 | REQ-017 | `GET /api-spec` → OpenAPI 3.1 JSON | ✅ Pass |
| ST-006 | REQ-018 | `GET /docs` → Scalar UI HTML | ✅ Pass |

---

## 5. Benchmark Tests

| Test ID | Requirement | Description | Status |
|:--|:--|:--|:--|
| BT-001 | NFR-001 | Gateway latency overhead | ✅ < 1ms |
| BT-002 | NFR-004 | Time to First Token (TTFT) | ✅ Measured |
| BT-003 | — | Tokens per Second (TPS) | ✅ Measured |
| BT-004 | NFR-003 | Concurrent throughput | ⬜ Pending |
| BT-005 | NFR-002 | Memory usage | ✅ Measured |
| BT-006 | — | Multi-model comparison report | ✅ HTML generated |
| BT-MED-001 | REQ-029 | MedGemma medical benchmark | ✅ Done |
