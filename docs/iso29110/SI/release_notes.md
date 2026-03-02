# Release Notes — Heimdall
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## Releases

---

## v0.1.0 — 2026-03-03

### Summary
Initial release of Heimdall — Internal LLM API Gateway for Mac Mini M4 Pro.

### Architecture
- **Option A**: Single engine (vllm-mlx) + Rust API Gateway (Axum)
- **Bare-metal**: No containers (Docker/K3s cannot access Metal GPU on macOS)
- **Model**: Qwen3.5-35B-A3B-Instruct-4bit (MoE, ~20GB, ~100+ tok/s expected)

### New Features
- **[REQ-010]** Rust API Gateway (Axum) — reverse proxy to vllm-mlx
- **[REQ-012]** API key authentication (Bearer token, configurable)
- **[REQ-014]** Health check endpoint (`/health`, `/ready`)
- **[REQ-015]** SSE streaming proxy for chat completions
- **[REQ-016]** Prometheus metrics endpoint (`/metrics`)
- **[REQ-020]** One-command setup script (`setup.sh`)
- **[REQ-021]** Start/stop scripts with PID management
- **[REQ-022]** Multi-model benchmark suite with HTML dashboard report
- **[REQ-024]** SemVer versioning system with git tag integration

### Test Results
- Unit: **9/9** passed (config: 4, auth: 5)
- Integration: 0 (pending hardware test)
- Benchmark scripts: ready (`--models`, `--all`, `--runs`)

### Components

| Component | Version | Language |
|:--|:--|:--|
| heimdall-gateway | 0.1.0 | Rust (Axum + Tokio) |
| vllm-mlx | latest | Python |
| benchmark suite | 0.1.0 | Bash + Python |
| version manager | 0.1.0 | Bash |

### Git Commits

| Commit | Description |
|:--|:--|
| `[WBS-001]` | ISO 29110 + TDD framework |
| `[WBS-006]` | Rust gateway + operation scripts |
| `[WBS-007]` | Benchmark suite + SemVer |
| `[WBS-008]` | Rename to Heimdall 🛡️ |
| `[WBS-009]` | Multi-model benchmark |
| `[WBS-010]` | ISO 29110 documentation update |

### Known Issues
- Rate limiting (REQ-013) not yet implemented
- Integration tests pending hardware deployment
