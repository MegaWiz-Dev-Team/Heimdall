# Gateway performance: admission control (Tier 1) + KV-reuse findings (Tier 2)

Adapted from techniques in the colibrì engine (bounded `--max-queue`,
`--kv-slots`/`cache_slot`). Heimdall is a *gateway*, so only the
serving-layer ideas apply here — kernel/quantization tricks live in the
MLX/llama.cpp backend and were explicitly out of scope (and prior work
measured no single-request decode win from them on Apple Silicon).

All numbers below were measured against the live `gemma-4-26b` (a4b MoE)
MLX backend, gateway built into an isolated `target-bench/` and run on
`:8090` so production `:8080` was never touched.

---

## Tier 1 — bounded FIFO admission control  ✅ implemented

**Problem.** `mlx_lm.server` continuous-batches fine up to ~16 concurrent,
but past its batch/KV ceiling it returns 5xx. The gateway had **no
backpressure** — it forwarded unbounded concurrency straight onto the
single-process backend, turning a capacity limit into client-visible
failures.

**Fix.** `gateway/src/admission.rs`: a Tokio `Semaphore` of `MAX_INFLIGHT`
permits (FIFO) plus a `MAX_QUEUE` bound on waiters. Total admitted-or-waiting
never exceeds `MAX_INFLIGHT + MAX_QUEUE`; excess is shed as an OpenAI-shaped
**429** (`rate_limit_exceeded`, with `Retry-After`), and a wait longer than
`QUEUE_TIMEOUT_S` returns **503** (`server_overloaded`). Admitted responses
carry `x-heimdall-queue-wait-ms`. Only the **local** backend path is gated;
external providers (OpenRouter/Gemini/OpenAI) manage their own capacity.

**Config (env):**

| var | default | meaning |
|---|---|---|
| `ADMISSION_ENABLED` | `1` | `0` restores old unbounded behavior |
| `MAX_INFLIGHT` | `16` | permits = concurrent requests sent to the backend |
| `MAX_QUEUE` | `32` | extra waiters allowed before 429 |
| `QUEUE_TIMEOUT_S` | `30` | wait longer → 503 |

Set `MAX_INFLIGHT` to the backend's measured **0-error concurrency ceiling**
(≈32 for gemma-4-26b on this box — see below); the demo used a conservative 16.

**Measured (max_tokens=8, greedy):**

| concurrency | baseline p99 | baseline errors | Tier-1 p99 | Tier-1 5xx | Tier-1 429 |
|---|---|---|---|---|---|
| 16 | 2.22s | 0 | 2.21s | 0 | 0 (zero overhead) |
| 32 | 5.28s | 0 | 4.39s | 0 | 0 |
| **64** | 6.61s | **19/64 = 30% 5xx** | **4.51s** | **0** | **32 clean shed** |

Headline: at overload, Tier-1 converts **30% hard 5xx → 0 errors + retryable
429 backpressure**, and admitted latency improves (backend no longer
thrashed). Trade-off: raw throughput-under-overload is capped by the operator
via `MAX_INFLIGHT`.

Reproduce: `scripts/bench/loadtest.py` (see `scripts/bench/*.json` for raw runs).

---

## Tier 2 — KV / prompt-prefix reuse  📋 measured, implementation deferred

**The win is real and large.** Re-using the KV of a shared system prompt
(2307 tokens) instead of re-prefilling it:

```
WARM (shared prefix):  0.36s/req   (cached_tokens=2289)
COLD (unique prefix):  3.97s/req   (cached_tokens=0)
→ 10.95×  ·  saved 3.6s/req
```

**But the MLX backend already delivers it** — no gateway change needed — for
a small number of active prefixes. **The gap:** `mlx_lm.server`'s prompt cache
holds only ~1–2 distinct prefixes. Probed with 8 distinct tenant prefixes,
pass-2 hit rate was **0/8** — the win collapses under the multi-tenant
workload Heimdall actually serves (Eir/CDS: many tenants, each a distinct long
system prompt).

**Therefore gateway sticky-routing to one backend is a no-op** (one small
cache). Making the 11× win survive multi-tenant needs **N backend slots +
gateway tenant/session-affinity routing** (colibrì `--kv-slots` + `cache_slot`
pattern): each hot tenant keeps its own warm KV slot, routed stickily by the
gateway.

**Deferred because it is an infra/RAM decision**, not a gateway patch: each
gemma slot is ~15 GB, so a 64 GB box holds ~2–3 slots. Options when picked up:

1. **Multi-instance MLX** — run N `mlx_lm.server` on N ports; gateway routes
   `hash(tenant|session) → port`; add per-route health + admission.
2. **llama.cpp `--slots N`** — one process, N independent KV slots; likely a
   better fit than mlx here (mlx's cache is too small). Gateway maps
   tenant/session → `cache_slot` (OpenAI-compatible extension field).
3. **Interim:** just size/verify the backend prompt cache; route stable-prefix
   traffic to keep one conversation warm.

Reproduce the win + capacity probe: `scripts/bench/prefix_probe.py`.
