# 🌑 Skuggi → Tyr (Wazuh SIEM) wiring

Ready-to-apply artifacts that turn Heimdall's Skuggi DLP events into
**structured, severity-tagged security events + alerts** in Tyr (Wazuh).

> **Status (2026-07-14): NOT YET ACTIVE.** The whole `wazuh` namespace is scaled
> to `0/0` (manager, indexer, dashboard) — almost certainly to save RAM on this
> box. Nothing flows to Tyr until it is running. These artifacts are authored and
> the shipper-side is already tested; **activation = apply the config below once
> Tyr is up** (ideally on a dedicated SIEM host, not the LLM-fleet box).

## What Skuggi emits

Heimdall logs one line per cloud-bound decision (`gateway/src/proxy.rs`):

```
🌑 skuggi[local-only]   EGRESS BLOCKED tenant=Some("asgard_medical") provider=gemini model=… — forbidden
🌑 skuggi[block-on-pii] BLOCKED        tenant=… provider=… model=… detections=…
🌑 skuggi[detect-only]  tenant=… provider=… model=… detections=… (NOT REDACTED)
🌑 skuggi[mask-and-send] tenant=… provider=… model=… detections=hn=1,patient_name=1
```

## Severity design — the DLP risk of each call

| event | condition | severity | Wazuh level | why it matters |
|---|---|---|---|---|
| **PII forwarded UNREDACTED** | `detect-only`/`off` + detections | **high** | 10 | a real leak — PII left to the cloud unmasked |
| policy DB/tier failure | Skuggi error / fallback | **high** | 9 | fail-open risk: tenant policy unavailable |
| PII rejected (strict) | `block-on-pii` + blocked | notice | 7 | PII caught, call rejected |
| no-cloud egress attempt | `local-only` + blocked | notice | 6 | a no-cloud tenant tried to reach the cloud (caller mis-config / probing) — **correlate on spikes** |
| PII redacted + sent | `mask-and-send` + detections | low | 3 | normal operation (telemetry / baseline) |
| clean/startup | connected, clean call | info | 2 | telemetry |

**Correlation alerts (define on top of the above):**
- `local-only` egress attempts **> 10 / 5 min** for one tenant → **level 12** (probable mis-config or probing).
- any **high (≥10)** event → page immediately (unredacted PHI to a third party).

## Two integration paths

### Path A — shipper → indexer (works with THIS setup) ✅ tested
`scripts/log_shipper.py` already tails `logs/gateway-stdout.log` and bulk-indexes
to the Wazuh Indexer (OpenSearch, `:30920`). It now **classifies Skuggi lines**
into `event_category=skuggi_dlp` + `skuggi_*` fields + `security_severity` +
`wazuh_level` (see `classify_skuggi`). Detection = **OpenSearch Alerting monitors**
(`opensearch-monitors.json`).
- Pro: no Wazuh-agent log mount needed; the shipper already reads the host log.
- Apply: bring the indexer up → the shipper delivers enriched events → import the monitors.

### Path B — agent → manager (the "proper" Wazuh path)
The `wazuh-agent` container forwards a monitored logfile to the manager, whose
**decoders + rules** classify it. Use `wazuh-skuggi-decoders.xml` +
`wazuh-skuggi-rules.xml` + `wazuh-agent-localfile.xml`.
- Pro: native SIEM rule engine, active-response, MITRE mapping.
- Con: the agent runs in a container → **mount the Heimdall `logs/` dir into it**
  (agent's `ossec.conf` `<localfile>` must see `gateway-stdout.log`).
- ⚠️ The decoders are **drafted, not validated** — run `wazuh-logtest` on a real
  line before enabling (Tyr was down, couldn't test).

## Prerequisites / recommended

1. **Disable ANSI in Heimdall logs** — `main.rs` tracing init: `fmt::layer().with_ansi(false)`
   so the log file is clean text (the classifier strips ANSI defensively, but
   clean logs are better for both paths). Ships with the next Heimdall build.
2. **Future upgrade (robust):** emit a dedicated JSON audit line to
   `logs/skuggi-audit.log` from Heimdall → Wazuh `log_format: json` needs **no
   decoder** and can't drift from the human-log format. Recommended once this
   stabilises.

## Known gaps

- `off` mode skips detection entirely, so a per-call "raw PII sent" event isn't
  logged — catch it via **config audit** (a tenant on `off`/`detect-only` is the
  alert, from `pii_redactions.pii_mode_used`), not the per-call log.
- Missing `X-Tenant-Id` → `tenant=None`; the DLP policy can't bind. Alert on
  `skuggi_tenant=None` for external calls too.
