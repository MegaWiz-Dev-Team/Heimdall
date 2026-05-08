# Contributing to Heimdall

Heimdall is part of the [Asgard AI Platform](https://github.com/MegaWiz-Dev-Team/Asgard). For the high-level workflow, CLA, and code of conduct, see [Asgard's CONTRIBUTING.md](https://github.com/MegaWiz-Dev-Team/Asgard/blob/main/CONTRIBUTING.md).

## This repo specifically

### Layout

- `gateway/` — Rust HTTP gateway (Axum) that fronts MLX, Ollama, llama.cpp, and remote LLM providers
- `scripts/` — operational tooling: benchmarks, log shipping, mlx_lm patches, launchd helpers
- `deploy/launchd/` — launchd plist templates for running on macOS as a host service
- `docs/iso29110/` — ISO 29110 docs and sprint reports

### Platform note

Heimdall runs **natively on the host** (not in Docker / not in Kubernetes) because Apple Silicon GPU/MLX cannot be passed through container runtimes. Linux/CUDA paths are best-effort.

### Development setup

```bash
# Rust gateway
cd gateway && cargo build --release

# Python venv for MLX server (only needed if running mlx_lm.server)
python3 -m venv .venv && source .venv/bin/activate && pip install mlx-lm

# Run tests
cd gateway && cargo test
```

### Running locally

```bash
# Set required env vars before launching
export HEIMDALL_API_KEY=$(openssl rand -hex 32 | sed 's/^/hml-/')
export API_KEYS="$HEIMDALL_API_KEY"

# Start gateway (binds :8080)
./gateway/target/release/heimdall-gateway

# Sanity check
curl -H "Authorization: Bearer $HEIMDALL_API_KEY" http://localhost:8080/v1/models
```

### Style

- `cargo fmt` + `cargo clippy --all-targets -- -D warnings`
- Conventional Commits (`feat:`, `fix:`, `docs:`, etc.)

### Reporting issues

- 🐛 Bugs: open an issue with the bug report template
- 💡 Features: open an issue with the feature request template
- 🔒 Security: see [SECURITY.md](SECURITY.md) (do **not** open public issues)

### License & CLA

By contributing, you agree to license your contribution under [AGPL-3.0](LICENSE) and the [Asgard CLA](https://github.com/MegaWiz-Dev-Team/Asgard/blob/main/CLA.md). Your first PR serves as your electronic signature.
