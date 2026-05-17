# Release Notes — Heimdall

## v0.3.0 — Yggdrasil JWT Auth (2026-05-17)

> Heimdall ฟัง Yggdrasil ได้แล้ว — Heimdall now speaks JWT.

### ✨ New Features
- **Yggdrasil JWT authentication** — Heimdall validates RS256-signed JWTs from a Zitadel-backed Yggdrasil IdP, with JWKS + OIDC discovery cached for 1 hour. Set `YGGDRASIL_ISSUER` and `JWT_AUDIENCE` to enable.
- **Dual-mode auth** — Static `API_KEYS` (legacy) and JWT can coexist on the same gateway; routing is by token shape (`ey…` → JWT, else static). No flag-day cutover required.
- **Mode-labeled metrics** — `auth_success_total{mode="jwt|static"}` and `auth_failure_total{mode="jwt|static|missing"}` for finer Prometheus dashboards.
- **Audit-grade tracing** — Successful JWT validations log `sub`, `tenant`, and `scope` claims at INFO; Tyr filebeat can ingest these without extra wiring.

### 🧪 Tests
- 8 new unit tests covering valid token, expired (with leeway), wrong issuer, wrong audience, unknown `kid`, garbage input, JWKS cache hit, and OIDC discovery flow.
- All 64 existing tests still pass.

### 📦 Crate changes
- gateway `0.5.0 → 0.6.0`
- `+ jsonwebtoken = "9"`, `+ moka = "0.12"` (with `future`), `+ wiremock = "0.6"` (dev-dep)

### 🔐 Security notes
- Default jsonwebtoken leeway (60s clock-skew tolerance) is retained.
- Static `API_KEYS` mode is **unchanged**; existing deployments are zero-impact unless `YGGDRASIL_ISSUER` is set.

### 📖 Docs
- `README.md` §2 rewritten with the dual-mode configuration matrix.
- Companion guide in Yggdrasil repo: `docs/heimdall-key-gen.md`.

---

## v0.4.0 — Production (2026-03-04)

> Asgard เป็นของทุกคนแล้ว — Asgard belongs to everyone.

### ✨ New Features
- **API Documentation** — `/api-spec` (OpenAPI 3.1 JSON) + `/docs` (Scalar UI)
- **MedGemma Benchmark** — 4B medical model performance tested
- **Qwen3.5 Benchmark** — 9B vs 27B on Apple Silicon (Mac Mini M4 Pro)

### 📊 Stats
- Production-ready gateway
- Multi-backend verified (Ollama, MLX, Gemini, OpenAI)

---

## v0.3.0 — MLX Native (2026-03-03)

### ✨ New Features
- **MLX Native Provider** — direct MLX embedding on Apple Silicon
- **Model Catalog** — centralized model registry with metadata
- **Embedding Server** — FastAPI + mlx-embedding-models

---

## v0.2.0 — Multi-Provider (2026-03-02)

### ✨ New Features
- **Gemini Provider** — Google AI integration
- **OpenAI Provider** — ChatGPT / GPT-4 compatible
- **Provider Router** — auto-select backend by model name

---

## v0.1.0 — Foundation (2026-02-28)

### ✨ New Features
- FastAPI + Uvicorn gateway server
- Ollama proxy with streaming response support
- Health checks and model listing
- Docker Compose integration

---

*Asgard เป็นของทุกคนแล้ว — Asgard belongs to everyone.*
