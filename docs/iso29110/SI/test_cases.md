# Test Cases
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-TST-001 |
| **Version** | 0.1 (Draft) |
| **Last Updated** | 2026-03-02 |
| **Status** | 🟡 Draft |

---

## 2. Unit Tests

### Gateway — Auth Module

| Test ID | Requirement | Description | Input | Expected Output |
|:--|:--|:--|:--|:--|
| UT-AUTH-001 | REQ-012 | Valid API key ผ่าน auth | Request + valid Bearer token | 200 + proxy pass-through |
| UT-AUTH-002 | REQ-012 | Invalid API key ถูก reject | Request + wrong token | 401 Unauthorized |
| UT-AUTH-003 | REQ-012 | Missing API key ถูก reject | Request ไม่มี Authorization header | 401 Unauthorized |

### Gateway — Router Module

| Test ID | Requirement | Description | Input | Expected Output |
|:--|:--|:--|:--|:--|
| UT-RTR-001 | REQ-011 | Route ไปยัง vllm-mlx เมื่อ header ระบุ | `X-Engine: vllm-mlx` | Forward to `:8000` |
| UT-RTR-002 | REQ-011 | Route ไปยัง mistral.rs เมื่อ header ระบุ | `X-Engine: mistral-rs` | Forward to `:8001` |
| UT-RTR-003 | REQ-011 | Route ไปยัง default engine เมื่อไม่ระบุ | No `X-Engine` header | Forward to default backend |
| UT-RTR-004 | REQ-011 | Unknown engine → error | `X-Engine: unknown` | 400 Bad Request |

### Gateway — Rate Limiter

| Test ID | Requirement | Description | Input | Expected Output |
|:--|:--|:--|:--|:--|
| UT-RL-001 | REQ-013 | Under limit → pass | 5 requests (limit=10) | All 200 |
| UT-RL-002 | REQ-013 | Over limit → reject | 15 requests (limit=10) | First 10: 200, rest: 429 |

### Gateway — Health Check

| Test ID | Requirement | Description | Input | Expected Output |
|:--|:--|:--|:--|:--|
| UT-HC-001 | REQ-014 | Healthy backend → included | Backend responds 200 | Backend in pool |
| UT-HC-002 | REQ-014 | Unhealthy backend → removed | Backend unreachable | Backend removed from pool |
| UT-HC-003 | REQ-014 | Recovered backend → re-added | Backend comes back online | Backend re-added to pool |

---

## 3. Integration Tests

| Test ID | Requirement | Description | Steps | Expected |
|:--|:--|:--|:--|:--|
| IT-001 | REQ-001,REQ-010 | End-to-end: client → gateway → vllm-mlx | Send chat completion via gateway | Valid response from vllm-mlx |
| IT-002 | REQ-002,REQ-010 | End-to-end: client → gateway → mistral.rs | Send chat completion via gateway | Valid response from mistral.rs |
| IT-003 | REQ-015 | SSE streaming through gateway | `"stream": true` via gateway | Chunked SSE response |
| IT-004 | REQ-014 | Failover when engine down | Stop vllm-mlx → send request | Route to fallback engine |

---

## 4. API Smoke Tests

| Test ID | Requirement | Description | Command | Expected |
|:--|:--|:--|:--|:--|
| ST-001 | REQ-004 | List models | `GET /v1/models` | 200 + model list |
| ST-002 | REQ-004 | Chat completion | `POST /v1/chat/completions` | 200 + response |
| ST-003 | REQ-005 | Streaming | `POST /v1/chat/completions` stream=true | 200 + SSE chunks |
| ST-004 | REQ-016 | Metrics | `GET /metrics` | 200 + Prometheus format |

---

## 5. Benchmark Tests

| Test ID | Requirement | Description | Metrics |
|:--|:--|:--|:--|
| BT-001 | NFR-001 | Gateway latency overhead | Gateway latency - direct backend latency |
| BT-002 | NFR-004 | Time to First Token | TTFT for each engine |
| BT-003 | — | Tokens per Second | TPS for each engine |
| BT-004 | NFR-003 | Concurrent throughput | Requests/min at 1, 5, 10, 50, 100 concurrent |
| BT-005 | NFR-002 | Memory usage | Peak RAM during inference |
