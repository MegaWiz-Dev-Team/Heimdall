# Test Report — Heimdall
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-TST-RPT-001 |
| **Version** | 2.0 |
| **Last Updated** | 2026-03-03 |
| **Status** | ✅ Sprint 3 results added |

---

## 2. Unit Tests — Gateway (Rust)

**Run date**: 2026-03-03 | **Command**: `cargo test`

| Test ID | Result | Notes |
|:--|:--|:--|
| UT-CFG-001 | ✅ Pass | Default config values |
| UT-CFG-002 | ✅ Pass | API keys parsing |
| UT-CFG-003 | ✅ Pass | Empty API keys |
| UT-CFG-004 | ✅ Pass | Backend URL construction |
| UT-AUTH-001 | ✅ Pass | No auth → pass all |
| UT-AUTH-002 | ✅ Pass | Valid API key → pass |
| UT-AUTH-003 | ✅ Pass | Invalid key → 401 |
| UT-AUTH-004 | ✅ Pass | Missing header → 401 |
| UT-AUTH-005 | ✅ Pass | Health bypasses auth |

**Summary**: 9/9 passed, 0 failed

---

## 3. Integration Tests — Sprint 3

**Run date**: 2026-03-03 | **Command**: `./tests/integration_test.sh`
**Backend**: mlx_lm.server (Qwen3-0.6B-4bit) | **Gateway**: Heimdall (port 3000)

| Test ID | Result | Notes |
|:--|:--|:--|
| ST-001 | ✅ Pass | GET /v1/models → 200 (model: Qwen3-0.6B-4bit) |
| ST-002 / IT-001 | ✅ Pass | POST /v1/chat/completions non-stream → 200 + AI response |
| ST-003 / IT-003 | ✅ Pass | POST /v1/chat/completions stream=true → 7 SSE chunks + [DONE] |
| ST-004 | ✅ Pass | GET /metrics → 200 + Prometheus (proxy_requests_total) |
| Health | ✅ Pass | GET /health → 200 |
| Root | ✅ Pass | GET / → 200 + "Heimdall" |

**Summary**: 6/6 passed, 0 failed

---

## 4. Known Issues

| ID | Description | Status | Resolution |
|:--|:--|:--|:--|
| ISS-001 | vllm-metal v0.16.0 crashes (`init_cpu_threads_env`) | 🟡 Open | Switched to mlx_lm.server (RISK-001 fallback) |
| ISS-002 | Model `Qwen3.5-35B-A3B-4bit` not supported by vllm-metal | 🟡 Open | Using mlx_lm.server which supports all MLX models |

---

## 5. Pending Tests

| Test ID | Description | Blocked On |
|:--|:--|:--|
| BT-001 | Gateway latency overhead | Need larger model for meaningful benchmark |
| BT-004 | Concurrent throughput | Need real workload testing |
