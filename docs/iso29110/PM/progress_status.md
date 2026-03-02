# Progress Status — Heimdall 🛡️
> ISO/IEC 29110 Basic Profile — Project Management Process

## Activity Log

| Date | Sprint | Activity | Status | Notes |
|:--|:--|:--|:--|:--|
| 2026-03-02 | S0 | WBS-001: Project governance & ISO 29110 | ✅ Done | 9 docs created |
| 2026-03-02 | S0 | WBS-002/003: Architecture decision (Option A) | ✅ Done | Single engine, bare-metal |
| 2026-03-02 | S0 | WBS-004: Container analysis | ✅ Done | Docker/K3s rejected |
| 2026-03-02 | S1 | WBS-006: Rust API Gateway (Axum) | ✅ Done | 9/9 unit tests |
| 2026-03-03 | S1 | WBS-007: Benchmark + SemVer versioning | ✅ Done | benchmark.sh, version.sh |
| 2026-03-03 | S1 | WBS-008: Project rename to Heimdall | ✅ Done | All files updated |
| 2026-03-03 | S2 | WBS-009: Multi-model benchmark support | ✅ Done | --models, --all flags |
| 2026-03-03 | S2 | WBS-010: ISO 29110 docs update | ✅ Done | All 9 docs updated |
| 2026-03-03 | S2 | WBS-014: Model storage management | ✅ Done | model_manager.sh, T7 Shield |
| 2026-03-03 | S3 | WBS-012: Start server | ✅ Done | mlx_vlm.server + Heimdall gateway |
| 2026-03-03 | S3 | WBS-011: Integration testing | ✅ Done | 6/6 IT + 9/9 UT pass |
| 2026-03-03 | S3 | RISK-001 triggered: vllm-metal broken | 🟡 Mitigated | Fallback to mlx_vlm.server |
| 2026-03-03 | S3 | Bug fix: start.sh binary name | ✅ Done | `llm-gateway` → `heimdall-gateway` |
| 2026-03-03 | S3 | Bug fix: gateway proxy /v1 prefix | ✅ Done | Strip /v1 for mlx_vlm compat |
| 2026-03-03 | S3 | Bug fix: health check endpoint | ✅ Done | `/v1/models` → `/models` |

## Sprint Velocity

| Sprint | Planned Tasks | Completed | Carry-over | Duration |
|:--|:--|:--|:--|:--|
| S0 | 4 | 4 | 0 | 1 day |
| S1 | 3 | 3 | 0 | 1 day |
| S2 | 3 | 3 | 0 | 1 day |
| S3 | 6 | 4 | 2 (LAN test, benchmark) | 1 day |

## Blockers

| ID | Description | Status | Impact |
|:--|:--|:--|:--|
| BLK-001 | vllm-metal v0.16.0 `init_cpu_threads_env` crash | ✅ Resolved | Switched to mlx_vlm.server |
| BLK-002 | Qwen3.5 = multimodal model (not text-only) | ✅ Resolved | Use mlx_vlm.server (not mlx_lm) |
| BLK-003 | Model name `Qwen3.5-35B-A3B-Instruct-4bit` invalid | ✅ Resolved | Correct: Qwen3.5-27B-4bit |
