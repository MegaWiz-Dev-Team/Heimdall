//! 🌑 Skuggi — PII Guardrail (Tier 1 regex matchers).
//!
//! v0 scaffolding for Sprint 50b. Pure-Rust, pre-LLM redaction for
//! cloud-bound payloads. See ADR-007:
//! `Asgard/docs/architecture/ADR-007-Skuggi-PII-Guardrail.md`.
//!
//! Tier 1 covers the predictable-pattern PII that handles ~80% of
//! redactions at <1ms latency. Tier 2 (PyThaiNLP NER sidecar for Thai
//! person names + addresses) is out of scope for this scaffolding —
//! lands in Sprint 50b proper once tenant policy + middleware wiring
//! are in place.
//!
//! NOT YET wired into `proxy.rs` or `router.rs`. This module exists so
//! that Sprint 50b kickoff has a foundation to build on rather than a
//! blank file.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

// Compiled Tier 1 regex patterns. Each is a separate `Lazy<Regex>`
// because the `regex::Regex` type has interior mutability, which Rust
// disallows in inline `static` array initialisers (E0492). Listing
// them out is also clearer for tenant-overridable per-pattern config
// when Sprint 50b proper wires this in.

// ─── Tier 1a — free-text finders (scan anywhere) ─────────────────────────

static RE_THAI_NATIONAL_ID: Lazy<Regex> = Lazy::new(|| {
    // 13 digits, optionally separated by dashes/spaces. First digit
    // is 1-8 per Thai citizen-ID spec (excludes test ranges 0/9).
    Regex::new(r"\b[1-8][- ]?\d{4}[- ]?\d{5}[- ]?\d{2}[- ]?\d\b").unwrap()
});

static RE_THAI_PHONE: Lazy<Regex> = Lazy::new(|| {
    // Thai mobile/landline: 0X + 8 more digits, or +66 international prefix.
    Regex::new(r"(?:\+66[- ]?|0)\d{1,2}[- ]?\d{3,4}[- ]?\d{4}\b").unwrap()
});

static RE_EMAIL: Lazy<Regex> = Lazy::new(|| {
    // RFC-5322 simplified — covers ~99% of clinical-doc emails.
    Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}").unwrap()
});

// ─── Tier 1b — anchored form-field patterns ──────────────────────────────
//
// Medical certificates / discharge summaries / referral letters in Thai
// hospitals use labeled fields ("Patient Name: …", "HN: …"). Anchored
// detectors give us high precision on these without the false-positive
// risk of free-form Thai-name regex (which would need NER — that's
// Tier 2 PyThaiNLP).
//
// Validated against the B-50h.0 fixture (30 Thai medical certificates):
// F1 ≥ 0.91 on all 5 categories (PATIENT_NAME / DOCTOR_NAME / HN /
// LICENSE_NO / THAI_ID), 100% after correcting 3 gold-labeling bugs in
// the source CSV. See Syn benchmarks/reports/2026-05-11_medical_certs_baseline.md
// (gitignored — PII content).

static RE_PATIENT_NAME_ANCHOR: Lazy<Regex> = Lazy::new(|| {
    // Captures up to end-of-line. Multi-line input is handled by callers
    // splitting on newlines OR by enabling the `m` flag (we choose
    // explicit `\n|$` so behaviour stays identical across input shapes).
    Regex::new(r"(?i)Patient\s*Name\s*[:：]?\s*([^\n]+?)(?:\n|$)").unwrap()
});

static RE_DOCTOR_NAME_ANCHOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)Doctor\s*Name\s*[:：]?\s*([^\n]+?)(?:\n|$)").unwrap()
});

static RE_HN_ANCHOR: Lazy<Regex> = Lazy::new(|| {
    // HN = Hospital Number. Some hospitals use slash/dash separators.
    Regex::new(r"(?i)\bHN\s*[:：]?\s*([0-9][\d\-/]*)").unwrap()
});

static RE_LICENSE_NO_ANCHOR: Lazy<Regex> = Lazy::new(|| {
    // Thai medical license: "License Number: ว.12345" or "License Number: 12345"
    Regex::new(r"(?i)License\s*Number\s*[:：]?\s*((?:ว\.?\s*)?\d[\w.\-\s]*?)(?:\n|$)").unwrap()
});

static RE_THAI_ID_ANCHOR: Lazy<Regex> = Lazy::new(|| {
    // Backup anchor for when the form uses an explicit "ThaiID:" label
    // instead of free-text 13-digit form. Captures the digits only.
    Regex::new(r"(?i)\bThai\s*ID\s*[:：]?\s*(\d{13})").unwrap()
});

/// Tier 1 pattern dispatch table. `(category, placeholder, regex,
/// replacement_strategy)`.
///
/// `Whole` replaces the full match with the placeholder (used by free-text
/// finders — keeping the label-free token form). `Group1` keeps the label
/// intact and replaces only capture group 1 (used by anchors — the form
/// label "Patient Name:" itself is not PII, only the value).
///
/// Order matters only for placeholder labelling — Tier 1 categories
/// are mutually exclusive in practice. Sprint 50b proper will fold in
/// per-tenant `pii_custom_patterns` from the `tenant_configs` table
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplaceMode {
    /// Replace the entire match (free-text finders).
    Whole,
    /// Replace only capture group 1 — leaves the form label intact.
    Group1,
}

fn patterns() -> [(&'static str, &'static str, &'static Regex, ReplaceMode); 8] {
    [
        // Tier 1a — free-text finders
        ("thai_national_id", "[REDACTED_THAI_ID]",      &RE_THAI_NATIONAL_ID,    ReplaceMode::Whole),
        ("thai_phone",       "[REDACTED_PHONE]",        &RE_THAI_PHONE,          ReplaceMode::Whole),
        ("email",            "[REDACTED_EMAIL]",        &RE_EMAIL,               ReplaceMode::Whole),
        // Tier 1b — anchored form-field patterns
        ("patient_name",     "[REDACTED_PATIENT_NAME]", &RE_PATIENT_NAME_ANCHOR, ReplaceMode::Group1),
        ("doctor_name",      "[REDACTED_DOCTOR_NAME]",  &RE_DOCTOR_NAME_ANCHOR,  ReplaceMode::Group1),
        ("hn",               "[REDACTED_HN]",           &RE_HN_ANCHOR,           ReplaceMode::Group1),
        ("license_no",       "[REDACTED_LICENSE_NO]",   &RE_LICENSE_NO_ANCHOR,   ReplaceMode::Group1),
        ("thai_id_anchored", "[REDACTED_THAI_ID]",      &RE_THAI_ID_ANCHOR,      ReplaceMode::Group1),
    ]
}

/// Per-call result from `redact_text`.
#[derive(Debug, Serialize, Default, Clone)]
pub struct RedactionResult {
    /// Text with every Tier 1 PII match replaced by the category placeholder.
    pub redacted_text: String,
    /// Non-empty when at least one match was found. Sorted by category.
    pub detections: Vec<Detection>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Detection {
    pub category: &'static str,
    /// Number of matches replaced for this category.
    pub count: usize,
}

/// Apply all Tier 1 patterns to `text`. Returns redacted text + audit
/// trail. **Always succeeds** — Tier 1 never raises; if a pattern
/// matches nothing, no detection is emitted for that category.
///
/// Free-text patterns replace the whole match; anchored patterns keep
/// the form label intact and replace only the captured value (so the
/// LLM still knows "this is the patient name field" — useful structural
/// context — but the actual name is gone).
pub fn redact_text(text: &str) -> RedactionResult {
    let mut current = text.to_string();
    let mut detections = Vec::new();

    for (category, placeholder, regex, mode) in patterns() {
        // Use captures_iter so we can count the matches up front. We need
        // the count for the audit row and to decide whether to do the
        // replace at all (avoids unnecessary allocation when nothing matches).
        let count = regex.find_iter(&current).count();
        if count == 0 {
            continue;
        }
        match mode {
            ReplaceMode::Whole => {
                current = regex.replace_all(&current, placeholder).into_owned();
            }
            ReplaceMode::Group1 => {
                // For anchored patterns: replace only group 1. We use a
                // closure-based replacement so the rest of the match
                // (the label, whitespace, colon) is preserved verbatim.
                current = regex.replace_all(&current, |caps: &regex::Captures| {
                    let whole = &caps[0];
                    match caps.get(1) {
                        Some(g1) => {
                            // Preserve everything before group 1, swap in
                            // the placeholder, then everything after.
                            let start = g1.start() - caps.get(0).unwrap().start();
                            let end = g1.end() - caps.get(0).unwrap().start();
                            format!("{}{}{}", &whole[..start], placeholder, &whole[end..])
                        }
                        None => whole.to_string(),
                    }
                }).into_owned();
            }
        }
        detections.push(Detection { category, count });
    }

    RedactionResult { redacted_text: current, detections }
}

// ─── External-payload helpers ──────────────────────────────────────────────

/// Walk an OpenAI-style chat-completions JSON body and Tier-1-redact every
/// user-visible text field. Modifies `body` in place. Returns aggregate
/// detections so the caller can log/audit.
///
/// Handles two `messages[*].content` shapes:
///   - `"content": "string"`           — redacted directly
///   - `"content": [{"type":"text","text":"…"}, {"type":"image_url",…}]`
///     — redacts only the `text` fields; image_url is left untouched
///       (image PII redaction is Sprint 50b Phase 2 — OpenCV YuNet)
///
/// All other JSON shapes are passed through unchanged. Bodies that don't
/// parse as JSON are passed through unchanged with an empty detection list.
pub fn redact_chat_body(body: &mut serde_json::Value) -> Vec<Detection> {
    let mut totals: std::collections::HashMap<&'static str, usize> =
        std::collections::HashMap::new();

    let Some(messages) = body.get_mut("messages").and_then(|v| v.as_array_mut()) else {
        return Vec::new();
    };

    for msg in messages.iter_mut() {
        let Some(content) = msg.get_mut("content") else { continue };
        match content {
            serde_json::Value::String(s) => {
                let r = redact_text(s);
                for d in r.detections {
                    *totals.entry(d.category).or_insert(0) += d.count;
                }
                *s = r.redacted_text;
            }
            serde_json::Value::Array(parts) => {
                for part in parts.iter_mut() {
                    let Some(text) = part.get_mut("text").and_then(|v| match v {
                        serde_json::Value::String(s) => Some(s),
                        _ => None,
                    }) else { continue };
                    let r = redact_text(text);
                    for d in r.detections {
                        *totals.entry(d.category).or_insert(0) += d.count;
                    }
                    *text = r.redacted_text;
                }
            }
            _ => {}
        }
    }

    let mut out: Vec<Detection> = totals.into_iter()
        .map(|(category, count)| Detection { category, count })
        .collect();
    out.sort_by_key(|d| d.category);
    out
}

// ─── Tier 2 client (PyThaiNLP NER sidecar) ────────────────────────────────
//
// Sprint 50b W2. Heimdall calls the sidecar conditionally — only when
// Tier 1 regex finds little/nothing but the payload is long Thai text
// suspicious of containing person names / addresses. This keeps the
// 50-100ms NER latency off the hot path for the ~80% of calls Tier 1
// can decide on alone (per ADR-007 §4 estimates).
//
// The sidecar is configured via `PYTHAINLP_URL` (default
// `http://pythainlp.asgard.svc:8086`). When the env var is unset OR
// the request fails, Heimdall logs a warning and falls back to the
// Tier 1 result — text-only redaction is still better than no
// redaction.

/// Returned by the Tier 2 sidecar `/detect` endpoint. Note: the wire
/// shape uses dynamic Vec<Value> so we don't have to keep parity with
/// every PyThaiNLP version's tag set.
#[derive(Debug, serde::Deserialize)]
pub struct Tier2Response {
    #[serde(default)]
    pub detections: Vec<Tier2Detection>,
    #[serde(default)]
    pub redacted_text: String,
    #[serde(default)]
    pub latency_ms: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct Tier2Detection {
    pub category: String,
    pub count: usize,
}

/// Run a single text payload through the Tier 2 sidecar. Returns
/// `Ok(None)` when the sidecar is unreachable or the URL is unset
/// (caller can decide to proceed with Tier 1 only). `Ok(Some(_))` on
/// success.
pub async fn tier2_detect(
    http_client: &reqwest::Client,
    text: &str,
) -> Result<Option<Tier2Response>, reqwest::Error> {
    let base = match std::env::var("PYTHAINLP_URL") {
        Ok(v) if !v.is_empty() => v,
        _ => return Ok(None),
    };
    let url = format!("{}/detect", base.trim_end_matches('/'));
    let body = serde_json::json!({ "text": text });
    let resp = http_client
        .post(url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await?;
    if !resp.status().is_success() {
        tracing::warn!("🌑 skuggi tier2 sidecar returned {}", resp.status());
        return Ok(None);
    }
    let parsed: Tier2Response = resp.json().await?;
    Ok(Some(parsed))
}

/// Decide whether the Tier 2 NER call is worth firing. Heuristic:
/// payload looks suspicious of containing Thai names but Tier 1 caught
/// nothing/little. Cheap regex scan — no allocation past the slice.
pub fn should_fire_tier2(text: &str, tier1_pii_total: usize) -> bool {
    // Skip if tier 1 already found PII — request is already going
    // through the audit path. Tier 2 still adds value here, but for v0
    // we keep its scope strictly on "tier 1 missed it" cases per
    // ADR-007 latency budget.
    if tier1_pii_total > 0 {
        return false;
    }
    if text.len() < 200 {
        return false; // not long enough to plausibly hide a name
    }
    // Cheap ASCII-only check: if every char is < 128, no Thai script.
    text.chars().any(|c| c as u32 >= 0x0E00 && c as u32 <= 0x0E7F)
}

#[cfg(test)]
mod tier2_heuristic_tests {
    use super::*;

    #[test]
    fn skip_when_tier1_already_found_pii() {
        let long_thai = "คนไข้ชื่อสมชาย ใจดี อายุ 65 ปี".repeat(20);
        assert!(!should_fire_tier2(&long_thai, 1));
    }

    #[test]
    fn skip_short_text() {
        assert!(!should_fire_tier2("คนไข้ชื่อสมชาย", 0));
    }

    #[test]
    fn skip_pure_english_long_text() {
        let long_en = "Patient stable on metoprolol 25mg twice daily, no complications. ".repeat(10);
        assert!(!should_fire_tier2(&long_en, 0));
    }

    #[test]
    fn fire_when_long_thai_and_no_tier1_hits() {
        let long_thai = "คนไข้รายนี้มีประวัติเบาหวานและความดันโลหิตสูง ".repeat(10);
        assert!(should_fire_tier2(&long_thai, 0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_thai_national_id_with_dashes() {
        let r = redact_text("ผู้ป่วย ID 1-2345-67890-12-3 มาตรวจ");
        assert!(r.redacted_text.contains("[REDACTED_THAI_ID]"));
        assert_eq!(r.detections.len(), 1);
        assert_eq!(r.detections[0].category, "thai_national_id");
    }

    #[test]
    fn redacts_thai_national_id_continuous() {
        let r = redact_text("MRN 1234567890123 admit ER");
        assert!(r.redacted_text.contains("[REDACTED_THAI_ID]"));
    }

    #[test]
    fn redacts_thai_phone_local() {
        let r = redact_text("contact 081-234-5678 for f/u");
        assert!(r.redacted_text.contains("[REDACTED_PHONE]"));
    }

    #[test]
    fn redacts_thai_phone_international() {
        let r = redact_text("call +66 81 234 5678 anytime");
        assert!(r.redacted_text.contains("[REDACTED_PHONE]"));
    }

    #[test]
    fn redacts_email() {
        let r = redact_text("send report to doctor@hospital.co.th today");
        assert!(r.redacted_text.contains("[REDACTED_EMAIL]"));
        assert_eq!(r.detections[0].category, "email");
    }

    #[test]
    fn no_match_returns_original_text() {
        let r = redact_text("Patient stable, vitals normal, discharge tomorrow.");
        assert_eq!(r.redacted_text, "Patient stable, vitals normal, discharge tomorrow.");
        assert!(r.detections.is_empty());
    }

    #[test]
    fn multiple_categories_in_one_pass() {
        let r = redact_text("ID 1-2345-67890-12-3 phone 081-234-5678 email a@b.co");
        assert!(r.redacted_text.contains("[REDACTED_THAI_ID]"));
        assert!(r.redacted_text.contains("[REDACTED_PHONE]"));
        assert!(r.redacted_text.contains("[REDACTED_EMAIL]"));
        assert_eq!(r.detections.len(), 3);
    }

    #[test]
    fn count_reflects_multiple_matches() {
        let r = redact_text("emails: alice@a.com, bob@b.com, eve@c.org");
        let email_det = r.detections.iter().find(|d| d.category == "email").unwrap();
        assert_eq!(email_det.count, 3);
    }

    #[test]
    fn chat_body_redacts_string_content() {
        let mut body: serde_json::Value = serde_json::json!({
            "messages": [
                {"role": "user", "content": "MRN 1-2345-67890-12-3 came to ER"}
            ]
        });
        let dets = redact_chat_body(&mut body);
        let content = body["messages"][0]["content"].as_str().unwrap();
        assert!(content.contains("[REDACTED_THAI_ID]"));
        assert_eq!(dets.len(), 1);
        assert_eq!(dets[0].category, "thai_national_id");
    }

    #[test]
    fn chat_body_redacts_array_content_text_parts() {
        let mut body: serde_json::Value = serde_json::json!({
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "patient phone 081-234-5678"},
                        {"type": "image_url", "image_url": {"url": "data:image/png;base64,…"}}
                    ]
                }
            ]
        });
        let dets = redact_chat_body(&mut body);
        let text = body["messages"][0]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("[REDACTED_PHONE]"));
        // image_url field must be untouched — image PII is a separate phase
        assert!(body["messages"][0]["content"][1]["image_url"]["url"].as_str()
            .unwrap().contains("base64"));
        assert_eq!(dets[0].category, "thai_phone");
    }

    #[test]
    fn chat_body_no_messages_array_returns_empty() {
        let mut body: serde_json::Value = serde_json::json!({"prompt": "ID 1-2345-67890-12-3"});
        let dets = redact_chat_body(&mut body);
        // Tier 1 only walks `messages` for now; legacy `prompt` field is
        // out of scope for v0 (caller should send chat-completions shape).
        assert!(dets.is_empty());
    }

    #[test]
    fn anchored_patient_name_preserves_label() {
        // Anchored detector keeps "Patient Name:" intact (form structure
        // is useful context for the LLM); only the value is redacted.
        let r = redact_text("Patient Name: นายสมชาย ใจดี\nDiagnosis: ไข้หวัด");
        assert!(r.redacted_text.contains("Patient Name: [REDACTED_PATIENT_NAME]"));
        assert!(r.redacted_text.contains("Diagnosis: ไข้หวัด"));
        let det = r.detections.iter().find(|d| d.category == "patient_name").unwrap();
        assert_eq!(det.count, 1);
    }

    #[test]
    fn anchored_doctor_name_with_title() {
        let r = redact_text("Doctor Name: พญ. อรวรรณ คงตระกูล\n");
        assert!(r.redacted_text.contains("Doctor Name: [REDACTED_DOCTOR_NAME]"));
    }

    #[test]
    fn anchored_hn_with_slash() {
        let r = redact_text("HN: 59453/45");
        assert!(r.redacted_text.contains("HN: [REDACTED_HN]"));
        assert!(!r.redacted_text.contains("59453"));
    }

    #[test]
    fn anchored_license_thai_prefix() {
        let r = redact_text("License Number: ว. 16358\n");
        assert!(r.redacted_text.contains("License Number: [REDACTED_LICENSE_NO]"));
    }

    #[test]
    fn anchored_thai_id_via_label() {
        // When the document uses the explicit "ThaiID:" label, the
        // anchored detector beats the free-text finder to the punch.
        let r = redact_text("ThaiID: 3470300256711\n");
        assert!(r.redacted_text.contains("ThaiID: [REDACTED_THAI_ID]"));
    }

    #[test]
    fn medical_certificate_full_redaction() {
        // Synthetic shape — same field layout as the B-50h.0 fixture but
        // with safe placeholder values (no real PII checked into git).
        let input = "Patient Name: SAMPLE NAME\n\
                     Doctor Name: SAMPLE DOCTOR\n\
                     HN: 12345678\n\
                     License Number: 99999\n\
                     Diagnosis: ไข้หวัดใหญ่\n";
        let r = redact_text(input);
        let cats: std::collections::HashSet<_> =
            r.detections.iter().map(|d| d.category).collect();
        assert!(cats.contains("patient_name"));
        assert!(cats.contains("doctor_name"));
        assert!(cats.contains("hn"));
        assert!(cats.contains("license_no"));
        // Diagnosis line untouched — clinical content stays
        assert!(r.redacted_text.contains("Diagnosis: ไข้หวัดใหญ่"));
    }

    #[test]
    fn no_label_no_anchored_match() {
        // Anchored patterns are HIGH-PRECISION by design — free-text
        // mentions of "Patient" without a colon don't trigger redaction.
        // The free-text finders (ID, phone, email) still catch their
        // own patterns in arbitrary text.
        let r = redact_text("The patient is doing well after surgery.");
        let cats: std::collections::HashSet<_> =
            r.detections.iter().map(|d| d.category).collect();
        assert!(!cats.contains("patient_name"));
    }

    #[test]
    fn chat_body_aggregates_across_multiple_messages() {
        let mut body: serde_json::Value = serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are a helpful clinical assistant."},
                {"role": "user", "content": "ID 1-2345-67890-12-3, phone 081-234-5678"},
                {"role": "user", "content": "follow up email patient@hospital.co.th"}
            ]
        });
        let dets = redact_chat_body(&mut body);
        // 1 ID + 1 phone + 1 email
        assert_eq!(dets.len(), 3);
        let cats: Vec<&str> = dets.iter().map(|d| d.category).collect();
        assert!(cats.contains(&"thai_national_id"));
        assert!(cats.contains(&"thai_phone"));
        assert!(cats.contains(&"email"));
    }
}
