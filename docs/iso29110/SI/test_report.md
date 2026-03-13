# Test Report — Heimdall
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-TST-RPT-001 |
| **Version** | 4.0 |
| **Last Updated** | 2026-03-13 |
| **Status** | ✅ v0.4.0 — Sprint 6 complete |

---

## 2. Unit Tests — Gateway (Rust)

**Run date**: 2026-03-08 | **Command**: `cargo test`

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

## 3. Integration Tests

**Run date**: 2026-03-08 | **Backend**: mlx_vlm.server (Qwen3.5-27B-4bit)

| Test ID | Result | Notes |
|:--|:--|:--|
| ST-001 | ✅ Pass | GET /v1/models → 200 (Qwen3.5-27B-4bit) |
| ST-002 / IT-001 | ✅ Pass | POST /v1/chat/completions non-stream → 200 |
| ST-003 / IT-003 | ✅ Pass | POST /v1/chat/completions stream=true → SSE chunks |
| ST-004 | ✅ Pass | GET /metrics → Prometheus format |
| ST-005 | ✅ Pass | GET /api-spec → OpenAPI 3.1 JSON |
| ST-006 | ✅ Pass | GET /docs → Scalar UI |
| Health | ✅ Pass | GET /health → 200 |
| Root | ✅ Pass | GET / → 200 + "Heimdall" |

**Summary**: 8/8 passed, 0 failed

---

## 4. Benchmark Results

### 4.1 Qwen3.5-27B-4bit

**Hardware**: Mac Mini M4 Pro, 64GB | **Engine**: mlx_vlm.server | **Peak RAM**: 16.2 GB

| Scenario | TTFT (avg) | TPS (avg) | Tokens |
|:--|:--|:--|:--|
| Short (20 tok) | 1.73s | 11.5 | 20 |
| Medium (200 tok) | 13.48s | 14.9 | 200 |
| Long (500 tok) | 33.21s | 15.1 | 500 |

### 4.2 Qwen3.5-9B-4bit

**Engine**: mlx_lm.server | **Peak RAM**: ~6 GB

| Scenario | TTFT (avg) | TPS (avg) | Tokens |
|:--|:--|:--|:--|
| Short (20 tok) | 0.8s | 28.5 | 20 |
| Medium (200 tok) | 6.5s | 30.2 | 200 |
| Long (500 tok) | 16.0s | 31.0 | 500 |

### 4.3 MedGemma 4B

**Engine**: mlx_lm.server | **Peak RAM**: ~4 GB

| Scenario | Medical Q&A | TPS (avg) |
|:--|:--|:--|
| Short medical query | ✅ Accurate | 35+ |
| Long medical analysis | ✅ Accurate | 32+ |

---

## 5. Known Issues

| ID | Description | Status | Resolution |
|:--|:--|:--|:--|
| ISS-001 | vllm-metal crashes (`init_cpu_threads_env`) | ✅ Resolved | Switched to mlx_vlm.server |
| ISS-002 | Qwen3.5 = multimodal, can't use mlx_lm | ✅ Resolved | Use mlx_vlm.server |
| ISS-003 | `head -n -1` fails on macOS | ✅ Resolved | Changed to `sed '$d'` |
| ISS-004 | HTML report IndexError | ✅ Resolved | Fixed in report_template.py |

---

## 6. Test Summary

| Category | Count | Passed | Failed |
|:--|:--|:--|:--|
| Unit Tests | 9 | 9 | 0 |
| Integration Tests | 8 | 8 | 0 |
| Benchmark Scenarios | 7+ | 7+ | 0 |
| **Total** | **24+** | **24+** | **0** |
