//! Per-tenant configuration cache for 🌑 Skuggi (Sprint 50b Day-1).
//!
//! Heimdall reads `tenant_configs.pii_mode` from Mimir's MariaDB on
//! demand. Lookups are cached in-process with a short TTL so the proxy
//! hot path stays sub-millisecond on cache hits.
//!
//! ## Why direct DB instead of Mimir API
//!
//! - Single-field read; full Mimir tenant-config endpoint is admin-only
//!   and returns way more than we need
//! - Heimdall is in the same K8s namespace as MariaDB (asgard / asgard-infra)
//! - sqlx pool keeps the round-trip <2ms on cache miss
//! - Failure mode (DB unreachable) falls back to env-var default rather
//!   than blocking cloud calls — see `get_pii_mode`
//!
//! ## Cache eviction
//!
//! - **TTL:** 60s (admin policy changes propagate within a minute)
//! - **No invalidation on update** — admin UI updates land in DB; readers
//!   pick them up at next TTL expiry. Trade simplicity for staleness.

use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sqlx::{mysql::MySqlPool, MySqlPool as _Alias};

/// Skuggi enforcement mode (ADR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PiiMode {
    /// No redaction. Tenant explicitly opted out (legacy or testing).
    Off,
    /// Run detection, log to audit table, but pass payload through unchanged.
    DetectOnly,
    /// Detect + redact + send redacted payload to provider. **Default.**
    MaskAndSend,
    /// Detect — if any PII found, fail the call. Strict-PHI hospitals.
    BlockOnPii,
}

impl PiiMode {
    /// Parse the kebab-case form stored in `tenant_configs.pii_mode`.
    /// Unknown values fall back to the safe default `mask-and-send`.
    pub fn from_db(s: &str) -> Self {
        match s {
            "off" => PiiMode::Off,
            "detect-only" | "detect_only" | "detectonly" => PiiMode::DetectOnly,
            "mask-and-send" | "mask_and_send" | "maskandsend" => PiiMode::MaskAndSend,
            "block-on-pii" | "block_on_pii" | "blockonpii" => PiiMode::BlockOnPii,
            _ => {
                tracing::warn!("Unknown pii_mode '{}', defaulting to mask-and-send", s);
                PiiMode::MaskAndSend
            }
        }
    }

    /// Stable string the audit table records.
    pub fn as_db_str(self) -> &'static str {
        match self {
            PiiMode::Off => "off",
            PiiMode::DetectOnly => "detect-only",
            PiiMode::MaskAndSend => "mask-and-send",
            PiiMode::BlockOnPii => "block-on-pii",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Cached {
    mode: PiiMode,
    fetched: Instant,
}

/// Per-tenant config cache.
///
/// Wraps a `DashMap` (lock-free concurrent map) so the read path doesn't
/// hold a mutex across `await` points. The associated `pool` is the
/// shared MariaDB pool from `AppState`.
pub struct TenantConfigCache {
    pool: MySqlPool,
    cache: DashMap<String, Cached>,
    ttl: Duration,
    /// Default applied when no row exists for this tenant_id.
    default_mode: PiiMode,
}

impl TenantConfigCache {
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            pool,
            cache: DashMap::new(),
            ttl: Duration::from_secs(60),
            default_mode: PiiMode::MaskAndSend,
        }
    }

    /// Look up the tenant's `pii_mode`. On cache hit (within TTL) returns
    /// immediately. On miss, hits MariaDB; if that fails, returns the
    /// safe default rather than blocking the request.
    pub async fn get_pii_mode(&self, tenant_id: &str) -> PiiMode {
        if let Some(c) = self.cache.get(tenant_id) {
            if c.fetched.elapsed() < self.ttl {
                return c.mode;
            }
        }

        match self.fetch_from_db(tenant_id).await {
            Ok(mode) => {
                self.cache.insert(
                    tenant_id.to_string(),
                    Cached { mode, fetched: Instant::now() },
                );
                mode
            }
            Err(e) => {
                tracing::warn!(
                    "tenant_config DB lookup failed for tenant_id={} ({}); using default {:?}",
                    tenant_id, e, self.default_mode
                );
                self.default_mode
            }
        }
    }

    async fn fetch_from_db(&self, tenant_id: &str) -> Result<PiiMode, sqlx::Error> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT pii_mode FROM tenant_configs WHERE tenant_id = ? LIMIT 1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some((s,)) => PiiMode::from_db(&s),
            None => self.default_mode,
        })
    }

    /// Test/admin only — clear cache so the next `get_pii_mode` call
    /// re-reads from DB. Used when the admin UI updates a tenant policy
    /// and wants the change to apply faster than the 60s TTL.
    #[allow(dead_code)]
    pub fn invalidate(&self, tenant_id: &str) {
        self.cache.remove(tenant_id);
    }

    /// Expose the underlying pool for audit inserts. Sprint 50b Day-3:
    /// `pii_redactions` writes share the same connection pool to avoid
    /// double-pooling when Heimdall has both readers (this cache) and
    /// writers (audit).
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }
}

// ─── pii_redactions audit insert ──────────────────────────────────────────
//
// Sprint 50b Day-3. Every Heimdall cloud-bound LLM call records one
// row, regardless of whether anything was redacted, so the compliance
// query "did Skuggi run for tenant X between dates Y/Z?" returns
// completeness data — not just hits.
//
// Inserts are fire-and-forget (`tokio::spawn`) so the request hot path
// is not blocked on DB write. If MariaDB is unreachable we log and drop
// the audit row rather than failing the call — `surface=text` audit is
// belt-and-suspenders to the upstream Mimir-core-ai cloud-call log;
// losing a row is recoverable.

use crate::skuggi::Detection;

/// One Heimdall Skuggi call worth recording.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub tenant_id: String,
    pub request_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub mode: PiiMode,
    pub detections: Vec<Detection>,
    pub pii_total_count: usize,
    pub blocked: bool,
    pub payload_bytes: usize,
    pub redacted_bytes: usize,
    pub duration: Duration,
    /// Which detection layers actually produced a finding.
    /// `"tier1"`, `"tier1+tier2"`, or `"tier2"`.
    pub detection_tier: &'static str,
}

impl AuditEvent {
    fn detections_json(&self) -> String {
        serde_json::to_string(&self.detections).unwrap_or_else(|_| "[]".to_string())
    }
}

/// Insert one audit row. Errors are logged and swallowed — the caller's
/// code path must not depend on this succeeding.
pub async fn insert_audit(pool: &MySqlPool, evt: AuditEvent) {
    // Schema reuses the Sprint 50 OCR audit table; surface='text' keeps
    // Heimdall rows separable from Syn OCR rows. id is VARCHAR(36) per
    // Sprint 50 schema — generate a fresh UUID per row.
    let id = uuid_v4_string();
    let detections_json = evt.detections_json();

    let res = sqlx::query(
        r#"INSERT INTO pii_redactions
            (id, tenant_id, request_id, provider, model, pii_mode_used, surface,
             detection_tier, decision, detections, pii_total_count, blocked,
             payload_bytes, redacted_bytes, duration_us, latency_ms,
             text_pii_count, image_face_count, image_text_box_count,
             pii_types_found)
           VALUES (?, ?, ?, ?, ?, ?, 'text',
                   ?, ?, ?, ?, ?,
                   ?, ?, ?, ?,
                   ?, 0, 0,
                   ?)"#,
    )
    .bind(&id)
    .bind(&evt.tenant_id)
    .bind(&evt.request_id)
    .bind(&evt.provider)
    .bind(&evt.model)
    .bind(evt.mode.as_db_str())
    .bind(evt.detection_tier) // tier1 | tier1+tier2 | tier2
    .bind(if evt.blocked { "blocked" } else { "passed" })
    .bind(&detections_json)
    .bind(evt.pii_total_count as u32)
    .bind(if evt.blocked { 1u8 } else { 0u8 })
    .bind(evt.payload_bytes as u32)
    .bind(evt.redacted_bytes as u32)
    .bind(evt.duration.as_micros() as u32)
    .bind((evt.duration.as_millis() as u32).max(1)) // legacy latency_ms column, never null per Sprint 50 schema
    .bind(evt.pii_total_count as u32) // text_pii_count = same as total for tier1
    .bind(&detections_json) // pii_types_found legacy column
    .execute(pool)
    .await;

    if let Err(e) = res {
        tracing::warn!(
            "🌑 skuggi audit insert failed (tenant={}, provider={}): {}",
            evt.tenant_id, evt.provider, e
        );
    }
}

fn uuid_v4_string() -> String {
    // Heimdall avoids the `uuid` crate to keep deps tight. Inline a v4
    // generator backed by `rand` (already transitively pulled in by
    // sqlx). Falls back to a timestamp-based id if rand fails.
    let mut bytes = [0u8; 16];
    if getrandom::getrandom(&mut bytes).is_err() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        bytes[..16].copy_from_slice(&now.to_le_bytes()[..16.min(16)]);
    }
    bytes[6] = (bytes[6] & 0x0F) | 0x40; // v4
    bytes[8] = (bytes[8] & 0x3F) | 0x80; // variant 1
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_modes() {
        assert_eq!(PiiMode::from_db("off"), PiiMode::Off);
        assert_eq!(PiiMode::from_db("detect-only"), PiiMode::DetectOnly);
        assert_eq!(PiiMode::from_db("mask-and-send"), PiiMode::MaskAndSend);
        assert_eq!(PiiMode::from_db("block-on-pii"), PiiMode::BlockOnPii);
    }

    #[test]
    fn parses_underscore_alias_variants() {
        // Some legacy rows may use underscores; accept both.
        assert_eq!(PiiMode::from_db("mask_and_send"), PiiMode::MaskAndSend);
        assert_eq!(PiiMode::from_db("block_on_pii"), PiiMode::BlockOnPii);
    }

    #[test]
    fn unknown_value_defaults_to_mask_and_send() {
        // Critical: never silently disable redaction on a typo.
        assert_eq!(PiiMode::from_db("garbage"), PiiMode::MaskAndSend);
        assert_eq!(PiiMode::from_db(""), PiiMode::MaskAndSend);
    }

    #[test]
    fn db_str_round_trip() {
        for &m in &[PiiMode::Off, PiiMode::DetectOnly, PiiMode::MaskAndSend, PiiMode::BlockOnPii] {
            assert_eq!(PiiMode::from_db(m.as_db_str()), m);
        }
    }
}
