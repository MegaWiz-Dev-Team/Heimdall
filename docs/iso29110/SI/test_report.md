# Test Report — Heimdall
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-TSR-001 |
| **Version** | 1.0 |
| **Last Updated** | 2026-03-03 |
| **Status** | ✅ 9/9 unit tests passing |

---

## 2. Test Execution Summary

| Test Category | Total | Passed | Failed | Skipped |
|:--|:--|:--|:--|:--|
| Unit Tests | 9 | 9 | 0 | 0 |
| Integration Tests | 2 | 0 | 0 | 2 |
| API Smoke Tests | 4 | 0 | 0 | 4 |
| Benchmark Tests | 6 | 3 | 0 | 3 |
| **Total** | **21** | **12** | **0** | **9** |

---

## 3. Test Results Log

### Unit Tests — `cargo test` (2026-03-03)

```
running 9 tests
test config::tests::test_default_config .............. ok
test config::tests::test_api_keys_parsing ............ ok
test config::tests::test_empty_api_keys .............. ok
test config::tests::test_backend_url ................. ok
test auth::tests::test_no_auth_configured_passes_all . ok
test auth::tests::test_valid_api_key_passes .......... ok
test auth::tests::test_invalid_api_key_rejected ...... ok
test auth::tests::test_missing_auth_header_rejected .. ok
test auth::tests::test_health_endpoint_bypasses_auth . ok

test result: ok. 9 passed; 0 failed
```

| Date | Test ID | Result | Notes |
|:--|:--|:--|:--|
| 2026-03-03 | UT-CFG-001 | ✅ Pass | test_default_config |
| 2026-03-03 | UT-CFG-002 | ✅ Pass | test_api_keys_parsing |
| 2026-03-03 | UT-CFG-003 | ✅ Pass | test_empty_api_keys |
| 2026-03-03 | UT-CFG-004 | ✅ Pass | test_backend_url |
| 2026-03-03 | UT-AUTH-001 | ✅ Pass | test_no_auth_configured_passes_all |
| 2026-03-03 | UT-AUTH-002 | ✅ Pass | test_valid_api_key_passes |
| 2026-03-03 | UT-AUTH-003 | ✅ Pass | test_invalid_api_key_rejected |
| 2026-03-03 | UT-AUTH-004 | ✅ Pass | test_missing_auth_header_rejected |
| 2026-03-03 | UT-AUTH-005 | ✅ Pass | test_health_endpoint_bypasses_auth |

---

## 4. Known Issues

| Issue ID | Related Test | Description | Severity | Status |
|:--|:--|:--|:--|:--|
| ISS-001 | UT-CFG-002 | Original test had env var race condition (parallel tests) | Low | ✅ Fixed — tests now parse directly |

---

## 5. Pending Tests

| Test ID | Blocked On | Notes |
|:--|:--|:--|
| IT-001, IT-003 | Hardware | ต้อง start vllm-mlx server จริง |
| ST-001~ST-004 | Hardware | ต้อง start full stack |
| BT-001, BT-004 | Hardware | ต้องรัน benchmark บนเครื่องจริง |
