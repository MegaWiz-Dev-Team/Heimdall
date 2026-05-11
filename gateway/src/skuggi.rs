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
    // ORDER MATTERS: anchored patterns run FIRST so a "ThaiID:
    // 1111111111110" hit is audited as `thai_id_anchored` (high-fidelity
    // form context) rather than swallowed by the free-text
    // `thai_national_id` finder. After anchored patterns redact their
    // groups, the free-text finders mop up any bare PII that wasn't in
    // a labelled field. This was caught by the leak-contract test —
    // see `leak_contract_tests::every_positive_case_fires_expected_categories`.
    [
        // Tier 1b — anchored form-field patterns (run first for audit fidelity)
        ("patient_name",     "[REDACTED_PATIENT_NAME]", &RE_PATIENT_NAME_ANCHOR, ReplaceMode::Group1),
        ("doctor_name",      "[REDACTED_DOCTOR_NAME]",  &RE_DOCTOR_NAME_ANCHOR,  ReplaceMode::Group1),
        ("hn",               "[REDACTED_HN]",           &RE_HN_ANCHOR,           ReplaceMode::Group1),
        ("license_no",       "[REDACTED_LICENSE_NO]",   &RE_LICENSE_NO_ANCHOR,   ReplaceMode::Group1),
        ("thai_id_anchored", "[REDACTED_THAI_ID]",      &RE_THAI_ID_ANCHOR,      ReplaceMode::Group1),
        // Tier 1a — free-text finders (mop up unlabelled PII)
        ("thai_national_id", "[REDACTED_THAI_ID]",      &RE_THAI_NATIONAL_ID,    ReplaceMode::Whole),
        ("thai_phone",       "[REDACTED_PHONE]",        &RE_THAI_PHONE,          ReplaceMode::Whole),
        ("email",            "[REDACTED_EMAIL]",        &RE_EMAIL,               ReplaceMode::Whole),
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

// ─── Leak-contract tests ─────────────────────────────────────────────────
//
// Goal: prove that for every PII shape Skuggi claims to handle, the
// redacted output contains ZERO of the original PII strings. This is the
// pre-merge gate — if these assertions fire, PII can leak to external
// LLMs and the change is not safe to ship.
//
// Mirrors the corpus shape seeded in Mimir migration
// 20260512000000_pii_test_corpus.sql (asgard_insurance tenant). Markers
// are synthetic-only.

#[cfg(test)]
mod leak_contract_tests {
    use super::*;
    use serde_json::json;

    /// One leak-test case: a payload to send through Skuggi and a list of
    /// substrings that MUST NOT appear in the output (the leak markers).
    struct Case {
        name: &'static str,
        /// Substrings that, if present in the redacted output, prove PII
        /// leaked. Includes both the raw PII and any per-row marker.
        forbidden_substrings: &'static [&'static str],
        /// Categories Skuggi must have detected. Empty = negative control.
        expected_categories: &'static [&'static str],
        /// Negative-control flag — Skuggi must produce ZERO detections.
        is_negative: bool,
        /// Prompt text to feed into redact_chat_body via a single-message
        /// chat body.
        prompt: &'static str,
    }

    /// Build the chat-body JSON shape `redact_chat_body` expects and run it.
    /// Returns (redacted_text, detections).
    fn run(prompt: &str) -> (String, Vec<Detection>) {
        let mut body = json!({
            "messages": [
                {"role": "user", "content": prompt}
            ]
        });
        let detections = redact_chat_body(&mut body);
        let redacted = body["messages"][0]["content"].as_str().unwrap().to_string();
        (redacted, detections)
    }

    fn corpus() -> Vec<Case> {
        vec![
            // ── Bucket 1: free-text positives ──────────────────────────
            Case {
                name: "free_text_thai_national_id",
                prompt: "ผู้ป่วยรหัสประจำตัว 1-9001-00000-01-1 มา ER",
                expected_categories: &["thai_national_id"],
                forbidden_substrings: &["1-9001-00000-01-1"],
                is_negative: false,
            },
            Case {
                name: "free_text_thai_phone",
                prompt: "ติดต่อกลับที่เบอร์ 081-555-0002",
                expected_categories: &["thai_phone"],
                forbidden_substrings: &["081-555-0002"],
                is_negative: false,
            },
            Case {
                name: "free_text_intl_phone",
                prompt: "Contact +66 81 555 0004 anytime",
                expected_categories: &["thai_phone"],
                forbidden_substrings: &["+66 81 555 0004"],
                is_negative: false,
            },
            Case {
                name: "free_text_email",
                prompt: "Forward report to pii-test-003@example.com",
                expected_categories: &["email"],
                forbidden_substrings: &["pii-test-003@example.com"],
                is_negative: false,
            },
            // ── Bucket 2: anchored positives ───────────────────────────
            Case {
                name: "anchored_patient_name",
                prompt: "Patient Name: SYNTHETIC_PATIENT_006\nDiagnosis: flu",
                expected_categories: &["patient_name"],
                forbidden_substrings: &["SYNTHETIC_PATIENT_006"],
                is_negative: false,
            },
            Case {
                name: "anchored_doctor_name",
                prompt: "Doctor Name: SYNTHETIC_DOCTOR_007\nDiagnosis: pharyngitis",
                expected_categories: &["doctor_name"],
                forbidden_substrings: &["SYNTHETIC_DOCTOR_007"],
                is_negative: false,
            },
            Case {
                name: "anchored_hn",
                prompt: "HN: 90008\nAdmission Date: 2026-05-12",
                expected_categories: &["hn"],
                forbidden_substrings: &["90008"],
                is_negative: false,
            },
            Case {
                name: "anchored_license",
                prompt: "License Number: ว. 99009\nIssued: 2024",
                expected_categories: &["license_no"],
                forbidden_substrings: &["99009"],
                is_negative: false,
            },
            Case {
                name: "anchored_thai_id",
                prompt: "ThaiID: 1111111111110\nCitizenship verified",
                expected_categories: &["thai_id_anchored"],
                forbidden_substrings: &["1111111111110"],
                is_negative: false,
            },
            // ── Bucket 3: multi-category positives ─────────────────────
            Case {
                name: "form_4_anchored_categories",
                prompt: "Patient Name: SYNTHETIC_PATIENT_011\nDoctor Name: SYNTHETIC_DOCTOR_011\nHN: 90011\nLicense Number: 99011\nDiagnosis: hypertension",
                expected_categories: &["patient_name", "doctor_name", "hn", "license_no"],
                forbidden_substrings: &[
                    "SYNTHETIC_PATIENT_011",
                    "SYNTHETIC_DOCTOR_011",
                    "90011",
                    "99011",
                ],
                is_negative: false,
            },
            Case {
                // Known over-capture: `doctor_name` regex is non-greedy
                // up to newline — when the line continues with email +
                // phone, those get absorbed into group 1 and redacted
                // under the `doctor_name` category. Leak contract still
                // holds (all 3 PII substrings disappear from output),
                // but audit fidelity collapses to a single category.
                // Follow-up: span-based single-pass redaction would emit
                // all 3 categories. For now this is the documented
                // behavior; we trade audit granularity for redaction
                // simplicity.
                name: "anchored_plus_free_text_on_same_line",
                prompt: "Reach out to Doctor Name: SYNTHETIC_DOCTOR_012 at pii-test-012@example.com or 081-555-0012",
                expected_categories: &["doctor_name"],
                forbidden_substrings: &[
                    "SYNTHETIC_DOCTOR_012",
                    "pii-test-012@example.com",
                    "081-555-0012",
                ],
                is_negative: false,
            },
            Case {
                name: "all_eight_categories_stress",
                prompt: "Patient Name: SYNTHETIC_PATIENT_015\nDoctor Name: SYNTHETIC_DOCTOR_015\nHN: 90015\nLicense Number: 99015\nThaiID: 1111111111115\nphone 081-555-0015 email pii-test-015@example.com NatID 1-9001-00015-15-1",
                expected_categories: &[
                    "patient_name", "doctor_name", "hn", "license_no",
                    "thai_id_anchored", "thai_national_id", "thai_phone", "email",
                ],
                forbidden_substrings: &[
                    "SYNTHETIC_PATIENT_015",
                    "SYNTHETIC_DOCTOR_015",
                    "90015",
                    "99015",
                    "1111111111115",
                    "081-555-0015",
                    "pii-test-015@example.com",
                    "1-9001-00015-15-1",
                ],
                is_negative: false,
            },
            // ── Bucket 4: insurance-domain shapes ──────────────────────
            Case {
                name: "insurance_claim_form",
                prompt: "Claim ID: CL-2026-00016\nClaimant Patient Name: SYNTHETIC_PATIENT_016\nThaiID: 1111111111116\nDiagnosis: ICD K85.9",
                expected_categories: &["patient_name", "thai_id_anchored"],
                forbidden_substrings: &["SYNTHETIC_PATIENT_016", "1111111111116"],
                is_negative: false,
            },
            Case {
                name: "insurance_preauth",
                prompt: "Pre-Authorization Request\nProvider Doctor Name: SYNTHETIC_DOCTOR_018\nLicense Number: ว. 99018\nClaimant HN: 90018",
                expected_categories: &["doctor_name", "license_no", "hn"],
                forbidden_substrings: &["SYNTHETIC_DOCTOR_018", "99018", "90018"],
                is_negative: false,
            },
            // ── Bucket 5: negative controls (no PII expected) ──────────
            Case {
                name: "neg_clinical_no_anchor",
                prompt: "Patient is stable on metoprolol 25mg twice daily. No complications.",
                expected_categories: &[],
                forbidden_substrings: &[],
                is_negative: true,
            },
            Case {
                name: "neg_lab_values",
                prompt: "Lab results: glucose 95 mg/dL, sodium 140 mEq/L, potassium 4.1 mEq/L.",
                expected_categories: &[],
                forbidden_substrings: &[],
                is_negative: true,
            },
            Case {
                name: "neg_icd_codes",
                prompt: "Diagnosis codes: ICD K85.9, J18.9, I10. Procedure codes: 1234, 5678, 91234.",
                expected_categories: &[],
                forbidden_substrings: &[],
                is_negative: true,
            },
            Case {
                name: "neg_iso_dates",
                prompt: "Admit 2026-04-01, discharge 2026-04-05, follow-up 2026-04-19.",
                expected_categories: &[],
                forbidden_substrings: &[],
                is_negative: true,
            },
            Case {
                name: "neg_partial_label",
                prompt: "The Doctor recommended physical therapy. Patient agreed to follow up.",
                expected_categories: &[],
                forbidden_substrings: &[],
                is_negative: true,
            },
            Case {
                name: "neg_vitals",
                prompt: "Pain scale 7/10, RR 18, HR 72, BP 130/85.",
                expected_categories: &[],
                forbidden_substrings: &[],
                is_negative: true,
            },
        ]
    }

    #[test]
    fn no_pii_substring_survives_redaction() {
        // For every case, run through redact_chat_body and assert no
        // forbidden substring (= original PII or marker) survives.
        let mut failures: Vec<String> = Vec::new();
        for case in corpus() {
            let (redacted, _detections) = run(case.prompt);
            for forbidden in case.forbidden_substrings {
                if redacted.contains(forbidden) {
                    failures.push(format!(
                        "LEAK in '{}': '{}' survived redaction.\n  Output: {}",
                        case.name, forbidden, redacted
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "PII LEAK DETECTED in {} case(s):\n{}",
            failures.len(),
            failures.join("\n---\n"),
        );
    }

    #[test]
    fn every_positive_case_fires_expected_categories() {
        // For every positive case, every expected category MUST be in
        // detections. If a category is missing, Skuggi failed to catch it
        // — which means it could leak in some other input shape.
        let mut failures: Vec<String> = Vec::new();
        for case in corpus() {
            if case.is_negative {
                continue;
            }
            let (_redacted, detections) = run(case.prompt);
            let got: std::collections::HashSet<&str> =
                detections.iter().map(|d| d.category).collect();
            for expected in case.expected_categories {
                if !got.contains(expected) {
                    failures.push(format!(
                        "MISS in '{}': expected category '{}' not detected. Got: {:?}",
                        case.name, expected, got
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "Detection misses in {} case(s):\n{}",
            failures.len(),
            failures.join("\n"),
        );
    }

    #[test]
    fn negative_controls_have_zero_detections() {
        // Negative controls must NOT trigger any pattern. Over-redaction
        // breaks clinical context (e.g., a real "Patient is stable…"
        // gets garbled).
        let mut failures: Vec<String> = Vec::new();
        for case in corpus() {
            if !case.is_negative {
                continue;
            }
            let (_redacted, detections) = run(case.prompt);
            if !detections.is_empty() {
                let cats: Vec<&str> = detections.iter().map(|d| d.category).collect();
                failures.push(format!(
                    "OVER-REDACT in '{}': expected no detections, got {:?}.\n  Prompt: {}",
                    case.name, cats, case.prompt
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "Over-redaction in {} negative control(s):\n{}",
            failures.len(),
            failures.join("\n---\n"),
        );
    }

    #[test]
    fn block_on_pii_path_would_reject_positives() {
        // Skuggi's block-on-pii proxy mode rejects any request where
        // detections.len() > 0. This test asserts the contract holds for
        // every positive case — they would all be 422'd, which is what
        // we want for high-stakes tenants.
        for case in corpus() {
            if case.is_negative {
                continue;
            }
            let (_redacted, detections) = run(case.prompt);
            let total: usize = detections.iter().map(|d| d.count).sum();
            assert!(
                total > 0,
                "block-on-pii would FAIL OPEN for '{}': zero detections on a positive case",
                case.name
            );
        }
    }
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
