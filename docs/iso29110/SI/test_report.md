# Test Report — Heimdall
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-TST-RPT-001 |
| **Version** | 3.0 |
| **Last Updated** | 2026-03-03 |
| **Status** | ✅ Sprint 3 complete — benchmark results added |

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
**Backend**: mlx_vlm.server (Qwen3.5-27B-4bit) | **Gateway**: Heimdall (port 3000)

| Test ID | Result | Notes |
|:--|:--|:--|
| ST-001 | ✅ Pass | GET /v1/models → 200 (model: Qwen3.5-27B-4bit) |
| ST-002 / IT-001 | ✅ Pass | POST /v1/chat/completions non-stream → 200 + AI response |
| ST-003 / IT-003 | ✅ Pass | POST /v1/chat/completions stream=true → 7 SSE chunks |
| ST-004 | ✅ Pass | GET /metrics → 200 + Prometheus (proxy_requests_total) |
| Health | ✅ Pass | GET /health → 200 |
| Root | ✅ Pass | GET / → 200 + "Heimdall" |

**Summary**: 6/6 passed, 0 failed

---

## 4. Benchmark Results — Qwen3.5-27B-4bit

**Run date**: 2026-03-03 02:43 | **Command**: `./scripts/benchmark.sh --runs 3`
**Hardware**: Mac Mini M4 Pro, 64GB Unified Memory
**Backend**: mlx_vlm.server | **Model**: `mlx-community/Qwen3.5-27B-4bit`
**Peak RAM**: 16.2 GB (~25% of 64 GB)

### Short Prompt (max 20 tokens)

| Run | TTFT (s) | TPS |
|:--|:--|:--|
| 1 | 1.797 | 11.1 |
| 2 | 1.696 | 11.8 |
| 3 | 1.711 | 11.7 |
| **Avg** | **1.73** | **11.5** |

### Medium Generation (max 200 tokens)

| Run | TTFT (s) | Tokens | TPS |
|:--|:--|:--|:--|
| 1 | 13.426 | 200 | 14.9 |
| 2 | 13.362 | 200 | 15.0 |
| 3 | 13.649 | 200 | 14.7 |
| **Avg** | **13.48** | **200** | **14.9** |

### Long Generation (max 500 tokens)

| Run | TTFT (s) | Tokens | TPS |
|:--|:--|:--|:--|
| 1 | 33.638 | 500 | 14.9 |
| 2 | 33.192 | 500 | 15.1 |
| 3 | 32.806 | 500 | 15.2 |
| **Avg** | **33.21** | **500** | **15.1** |

### Performance Summary

| Metric | Short | Medium | Long |
|:--|:--|:--|:--|
| **Avg TPS** | 11.5 | 14.9 | 15.1 |
| **Avg TTFT** | 1.73s | 13.48s | 33.21s |
| **Max Tokens** | 20 | 200 | 500 |

> **Note**: TTFT includes thinking time — Qwen3.5 generates `<think>` tokens before answer. Actual sustained generation TPS is ~15 tok/s.

---

## 5. Known Issues

| ID | Description | Status | Resolution |
|:--|:--|:--|:--|
| ISS-001 | vllm-metal v0.16.0 crashes (`init_cpu_threads_env`) | 🟡 Open | Switched to mlx_vlm.server (RISK-001) |
| ISS-002 | Qwen3.5 = multimodal model, cannot use mlx_lm | ✅ Resolved | Use mlx_vlm.server instead |
| ISS-003 | `head -n -1` fails on macOS | ✅ Resolved | Changed to `sed '$d'` in benchmark.sh |
| ISS-004 | HTML report generator crashes (`IndexError`) | 🟡 Open | Raw benchmark data captured in this doc |

---

## 6. Pending Tests

| Test ID | Description | Blocked On |
|:--|:--|:--|
| BT-001 | Gateway latency overhead measurement | Needs control vs proxy comparison |
| BT-004 | Concurrent throughput | Needs load testing tool |
| WBS-011d | LAN connectivity test | Needs another device on network |
