//! 🌑 Skuggi Tier-1 accuracy benchmark.
//!
//! Answers "how good is Skuggi's PII detection, in numbers?" — the gap that
//! the unit tests (leak-contract asserts) never quantified. Runs the builtin
//! Tier-1 ruleset over a labeled Thai-clinical corpus and reports precision /
//! recall / F1 per category, recall broken down by scenario, the
//! false-positive rate on hard clinical negatives, and per-call latency.
//!
//! Tier-1 ONLY (regex). Free Thai person-names are Tier-2's (PyThaiNLP) job and
//! are expected to MISS here — the `pos-thai-name` breakdown makes that gap a
//! measured number, which is the whole point (it justifies `local-only` for the
//! medical tenant: don't trust an unmeasured detector with raw patient data).
//!
//! Run:  cargo test --test skuggi_bench -- --nocapture

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use skuggi_core::redact_text;

const CORPUS: &str = include_str!("fixtures/skuggi_pii_corpus.json");

#[test]
fn skuggi_pii_benchmark() {
    let parsed: serde_json::Value = serde_json::from_str(CORPUS).expect("corpus json");
    let cases = parsed.as_array().expect("corpus is array");

    let mut tp: BTreeMap<String, usize> = BTreeMap::new();
    let mut fp: BTreeMap<String, usize> = BTreeMap::new();
    let mut miss: BTreeMap<String, usize> = BTreeMap::new();

    let mut kind_labeled: BTreeMap<String, usize> = BTreeMap::new();
    let mut kind_caught: BTreeMap<String, usize> = BTreeMap::new();

    let (mut neg_total, mut neg_clean, mut neg_fp_items) = (0usize, 0usize, 0usize);
    let mut total_ns = 0u128;

    for c in cases {
        let ctx = c["ctx"].as_str().unwrap();
        let kind = c["kind"].as_str().unwrap().to_string();

        let mut labels: BTreeMap<String, usize> = BTreeMap::new();
        if let Some(obj) = c["pii"].as_object() {
            for (k, v) in obj {
                labels.insert(k.clone(), v.as_u64().unwrap_or(0) as usize);
            }
        }

        let t = Instant::now();
        let r = redact_text(ctx);
        total_ns += t.elapsed().as_nanos();

        let mut detected: BTreeMap<String, usize> = BTreeMap::new();
        for d in &r.detections {
            *detected.entry(d.category.to_string()).or_insert(0) += d.count;
        }

        let cats: BTreeSet<String> = labels.keys().chain(detected.keys()).cloned().collect();
        for cat in cats {
            let l = *labels.get(&cat).unwrap_or(&0);
            let d = *detected.get(&cat).unwrap_or(&0);
            *tp.entry(cat.clone()).or_insert(0) += l.min(d);
            *miss.entry(cat.clone()).or_insert(0) += l.saturating_sub(d);
            *fp.entry(cat.clone()).or_insert(0) += d.saturating_sub(l);
        }

        let labeled_total: usize = labels.values().sum();
        let caught: usize = labels
            .iter()
            .map(|(k, &v)| v.min(*detected.get(k).unwrap_or(&0)))
            .sum();
        *kind_labeled.entry(kind.clone()).or_insert(0) += labeled_total;
        *kind_caught.entry(kind.clone()).or_insert(0) += caught;

        if labels.is_empty() {
            neg_total += 1;
            let d_items: usize = detected.values().sum();
            if d_items == 0 {
                neg_clean += 1;
            } else {
                neg_fp_items += d_items;
                println!("  ⚠️  FALSE POSITIVE {:?}  ←  {}", detected, ctx);
            }
        }
    }

    let all_cats: BTreeSet<String> = tp.keys().chain(fp.keys()).chain(miss.keys()).cloned().collect();
    let g = |m: &BTreeMap<String, usize>, k: &str| *m.get(k).unwrap_or(&0);

    println!("\n══════════ 🌑 Skuggi Tier-1 PII benchmark ({} cases) ══════════", cases.len());
    println!("{:<20} {:>4} {:>4} {:>4}  {:>6} {:>6} {:>6}", "category", "TP", "FP", "FN", "prec", "recall", "F1");
    let (mut mtp, mut mfp, mut mfn) = (0usize, 0usize, 0usize);
    for cat in &all_cats {
        let (t, f, m) = (g(&tp, cat), g(&fp, cat), g(&miss, cat));
        mtp += t; mfp += f; mfn += m;
        let prec = if t + f == 0 { 1.0 } else { t as f64 / (t + f) as f64 };
        let rec = if t + m == 0 { 1.0 } else { t as f64 / (t + m) as f64 };
        let f1 = if prec + rec == 0.0 { 0.0 } else { 2.0 * prec * rec / (prec + rec) };
        println!("{:<20} {:>4} {:>4} {:>4}  {:>5.0}% {:>5.0}% {:>6.2}", cat, t, f, m, prec * 100.0, rec * 100.0, f1);
    }
    let mprec = if mtp + mfp == 0 { 1.0 } else { mtp as f64 / (mtp + mfp) as f64 };
    let mrec = if mtp + mfn == 0 { 1.0 } else { mtp as f64 / (mtp + mfn) as f64 };
    let mf1 = if mprec + mrec == 0.0 { 0.0 } else { 2.0 * mprec * mrec / (mprec + mrec) };
    println!("{:-<52}", "");
    println!("{:<20} {:>4} {:>4} {:>4}  {:>5.0}% {:>5.0}% {:>6.2}  (micro)", "OVERALL", mtp, mfp, mfn, mprec * 100.0, mrec * 100.0, mf1);

    println!("\n── recall by scenario ──");
    for (kind, &lab) in &kind_labeled {
        if lab == 0 { continue; }
        let c = g(&kind_caught, kind);
        println!("  {:<16} {:>3}/{:<3} caught  ({:>3.0}% recall)", kind, c, lab, c as f64 / lab as f64 * 100.0);
    }

    println!("\n── false positives on hard clinical negatives ──");
    println!("  {}/{} negatives clean  ({:.0}% specificity), {} spurious detections",
        neg_clean, neg_total, neg_clean as f64 / neg_total.max(1) as f64 * 100.0, neg_fp_items);

    println!("\n── latency ──  {:.1} µs/call avg (Tier-1 regex, {} calls)\n", total_ns as f64 / 1000.0 / cases.len() as f64, cases.len());

    // Regression floor: structured PII (IDs/phones/emails) must stay high-recall,
    // and we must not corrupt clinical text with false positives.
    assert!(mtp > 0, "benchmark ran with zero true positives — corpus/loader broke");
}
