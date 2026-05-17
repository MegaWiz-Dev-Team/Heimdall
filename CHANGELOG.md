# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-05-17
### Added
- **Yggdrasil JWT authentication** (gateway 0.6.0). Heimdall now accepts RS256-signed JWTs issued by a Zitadel-backed Yggdrasil IdP alongside legacy static `API_KEYS`. JWKS and OIDC discovery responses are cached for 1 hour via `moka`. Audit events emit on `tracing` at INFO/WARN for Tyr ingestion.
- New env vars `YGGDRASIL_ISSUER` and `JWT_AUDIENCE` enable JWT mode; both auth modes can coexist on the same gateway.
- New module `gateway/src/auth_jwt.rs` with 8 unit tests covering: valid token, expired, wrong issuer, wrong audience, unknown kid, garbage input, JWKS caching, OIDC discovery.

### Changed
- `auth_middleware` is now dual-mode. Bearer tokens starting with `ey` (the b64 of a JWT header) are routed to JWT validation; everything else falls back to static-key compare.
- Metrics `auth_success_total` and `auth_failure_total` now carry a `mode` label (`jwt` / `static` / `missing`).

### Fixed
- `auth.rs` and `router.rs` test fixtures were missing `vlm_q4_port` / `vlm_q8_port` after the Sprint 51 VLM port additions; `cargo test` compilation no longer requires manual edits.

## [0.2.0] - 2026-04-02
### Added
- Created generic `upload_to_hf.py` script featuring `argparse` for standard public MLX model deployment to Hugging Face Hub.
- Included comprehensive generation benchmarks for `Qwen3.5-27B-Opus-Reasoning-MLX-4bit` natively evaluated on Apple M4 Pro architecture arrays.

### Fixed
- Fixed critical authentication bug in `benchmark.sh` where metric API calls failed when `API_KEYS` were enforced at gateway levels.
- Fixed `benchmark.sh` rendering issue where unformatted Array indices crashed bash JSON string concatenation.

### Changed
- Standardized `convert_qwen_27b.sh` structure to deduce dynamic execution paths, detaching dependencies on user-specific `$HOME/Developer/` roots.
- Removed hardcoded external SSD (`/Volumes/T7 Shield/...`) overrides from scripts, favoring dynamically resolved environmental standards (`HF_HUB_CACHE`).
- Overhauled README documentation specifically readying Apple Silicon operations for open-source public access.

## [0.1.0] - 2026-03-01
### Added
- Initial project layout for Heimdall AI Framework.
- MLX inference integration for lightweight LLM serving on Apple Silicon.
- Baseline benchmark pipelines.
