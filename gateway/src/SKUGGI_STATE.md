# 🌑 Skuggi PII Guardrail — Implementation State

**As of:** 2026-05-09 (Sprint 50b W1 D1-D3 shipped)
**ADR:** [ADR-007](../../../Asgard/docs/architecture/ADR-007-Skuggi-PII-Guardrail.md)
**Status:** Tier 1 + per-tenant policy + audit insert wired. Tier 2 + Image PII pending.

## Sprint 50b checklist

### Week 1 — Text PII core

- [x] **D1.1** DB migration `20260509000000_skuggi_pii_redactions.sql` — adds 8 cols to `pii_redactions` + idx_blocked_created + tenant_configs.pii_mode default = `mask-and-send`. Applied to prod MariaDB.
- [x] **D1.2** `src/tenant_config.rs::TenantConfigCache` — sqlx + DashMap, 60s TTL, fail-open to `mask-and-send` if MariaDB unreachable
- [x] **D1.3** `src/tenant_config.rs::PiiMode` enum (off/detect-only/mask-and-send/block-on-pii) + 4 unit tests
- [x] **D1.4** AppState wires optional `tenant_cfg: Option<Arc<TenantConfigCache>>`. Heimdall starts cleanly with or without `MARIADB_URL`.
- [x] **D2.1** `proxy.rs` reads `X-Tenant-Id` + `X-Request-Id` from headers
- [x] **D2.2** Mode resolution order: tenant_configs → SKUGGI_MODE env → mask-and-send default
- [x] **D2.3** 4-mode dispatch:
  - `off` — no redaction, no audit
  - `detect-only` — run, log, send original
  - `mask-and-send` — run, log, send redacted (default)
  - `block-on-pii` — run; return HTTP 422 if any PII; emit `skuggi_blocked_total` metric
- [x] **D3.1** `tenant_config.rs::insert_audit` — fire-and-forget `tokio::spawn` so request hot path is not blocked on DB write
- [x] **D3.2** `AuditEvent` struct captures tenant_id, request_id, provider, model, mode, detections JSON, pii_total_count, blocked, payload_bytes, redacted_bytes, duration
- [x] **D3.3** Audit row also fires on blocked calls (compliance proof)
- [ ] **D4** ADR-007 update + this state doc — done
- [ ] **D4-5** E2E test recipe + insurance POC runbook draft

### Week 2 — Tier 2 Thai NER

- [ ] PyThaiNLP FastAPI sidecar (port 8086, K8s Deployment)
- [ ] Heimdall calls Tier 2 conditionally when Tier 1 detection count is low
- [ ] Tier 2 categories: thai_person_name, thai_address, hospital_name
- [ ] Conditional firing heuristic (~10-20% of calls per ADR estimate)

### Week 3 — Image PII

- [ ] OpenCV YuNet face detector + blur
- [ ] PaddleOCR text + bbox → Thai national ID box detection + blackout
- [ ] Integration with syn-api cloud-OCR path
- [ ] Image hash audit (sha256 pre + post for chain-of-custody)

### Week 4 — Audit + POC

- [ ] Mimir dashboard panel for pii_redactions queries
- [ ] Tenant admin UI for `pii_mode` + `pii_custom_patterns`
- [ ] Insurance POC runbook
- [ ] Performance hardening (Tier 1 <1ms p99, Tier 2 ≤300ms p99)

## How to enable

### Production (with MariaDB tenant config)

1. Ensure `MARIADB_URL` env var set on Heimdall pod, e.g.:
   ```
   MARIADB_URL=mysql://mimir:mimir_password@mariadb.asgard.svc:3306/mimir
   ```
2. Set tenant policy via Mimir admin UI or directly:
   ```sql
   UPDATE tenant_configs SET pii_mode = 'block-on-pii' WHERE tenant_id = 'insurance-acme';
   ```
3. Insurance integration sends `X-Tenant-Id: insurance-acme` header on every cloud-bound LLM call
4. Heimdall logs every redaction to `pii_redactions` (`surface=text`, `detection_tier=tier1`)

### Dev / fallback (no DB)

```bash
export SKUGGI_MODE=mask-and-send
launchctl unload ~/Library/LaunchAgents/com.asgard.heimdall-gateway.plist
launchctl load   ~/Library/LaunchAgents/com.asgard.heimdall-gateway.plist
```

## E2E smoke test recipe (post-deployment)

```bash
# 1. Ensure tenant policy is set
TENANT_ID=test-tenant
kubectl exec -n asgard-infra mariadb-... -- mariadb -u mimir -p... mimir -e \
  "INSERT INTO tenant_configs (tenant_id, default_provider, default_model, max_daily_tokens, pii_mode) \
   VALUES ('$TENANT_ID','heimdall','dummy',100000,'mask-and-send') \
   ON DUPLICATE KEY UPDATE pii_mode='mask-and-send';"

# 2. Send a request with PII
curl -X POST http://heimdall:3000/v1/chat/completions \
  -H "X-Tenant-Id: $TENANT_ID" \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini/gemini-2.5-flash",
    "messages": [{"role":"user","content":"Patient ID 1-2345-67890-12-3, phone 081-234-5678"}]
  }'

# 3. Verify redaction in audit
kubectl exec -n asgard-infra mariadb-... -- mariadb -u mimir -p... mimir -e \
  "SELECT id, tenant_id, mode, detections, pii_total_count, blocked, duration_us \
   FROM pii_redactions \
   WHERE tenant_id='$TENANT_ID' AND surface='text' \
   ORDER BY created_at DESC LIMIT 1;"

# Expected: detections shows thai_national_id=1, thai_phone=1; pii_total_count=2
```

## Compliance audit query (per-tenant, last 30 days)

```sql
SELECT
  DATE(created_at) AS day,
  COUNT(*) AS total_calls,
  SUM(CASE WHEN pii_total_count > 0 THEN 1 ELSE 0 END) AS calls_with_pii,
  SUM(blocked) AS blocked_calls,
  AVG(duration_us) / 1000 AS avg_duration_ms
FROM pii_redactions
WHERE tenant_id = ?
  AND surface = 'text'
  AND created_at > NOW() - INTERVAL 30 DAY
GROUP BY DATE(created_at)
ORDER BY day DESC;
```
