#!/usr/bin/env python3
"""
Multi-tenant KV-warmth benchmark for Tier-2 affinity routing.

N tenants, each a distinct long system prompt. Warm each once, then interleave
them for several rounds and measure the prompt-cache HIT RATE (cached_tokens vs
prompt_tokens) + mean latency. Sends X-Tenant-Id so the gateway routes by
affinity. Single-slot (all tenants → one cache) thrashes; multi-slot spreads
them so each slot keeps its subset warm.
"""
import argparse, json, time, sys
import urllib.request

def make_system(n_para, tag):
    para = ("You are a careful clinical decision-support assistant for a specific "
            "institution. Follow local hypertension and dyslipidemia protocols and "
            "cite the section. Never invent doses; check renal function first. ")
    return f"[{tag}] " + para * n_para

def call(url, tenant, system, user, max_tokens):
    body = json.dumps({
        "model": "mlx-community/gemma-3-1b-it-4bit",
        "messages": [{"role": "system", "content": system},
                     {"role": "user", "content": user}],
        "max_tokens": max_tokens, "temperature": 0.0, "stream": False,
    }).encode()
    req = urllib.request.Request(url, data=body, method="POST",
        headers={"Content-Type": "application/json", "X-Tenant-Id": tenant})
    t0 = time.perf_counter()
    with urllib.request.urlopen(req, timeout=120) as r:
        d = json.loads(r.read())
    lat = time.perf_counter() - t0
    u = d.get("usage", {})
    pt = u.get("prompt_tokens") or 1
    ct = (u.get("prompt_tokens_details") or {}).get("cached_tokens") or 0
    return lat, pt, ct

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default="http://127.0.0.1:8090/v1/chat/completions")
    ap.add_argument("--tenants", type=int, default=4)
    ap.add_argument("--rounds", type=int, default=3)
    ap.add_argument("--para", type=int, default=30)
    ap.add_argument("--max-tokens", type=int, default=4)
    ap.add_argument("--tag", default="run")
    a = ap.parse_args()

    tenants = [(f"tenant-{i}", make_system(a.para, f"INST{i}")) for i in range(a.tenants)]
    # warm pass
    for name, sysp in tenants:
        call(a.url, name, sysp, "warm", a.max_tokens)
    # measured interleaved passes
    hits = tot = 0; lats = []
    for r in range(a.rounds):
        for name, sysp in tenants:
            lat, pt, ct = call(a.url, name, sysp, f"q{r}", a.max_tokens)
            warm = ct >= 0.8 * pt
            hits += warm; tot += 1; lats.append(lat)
    hr = 100.0 * hits / tot
    ml = sum(lats) / len(lats)
    print(f"[{a.tag}] tenants={a.tenants} rounds={a.rounds} → "
          f"HIT RATE {hits}/{tot} = {hr:.0f}%  ·  mean latency {ml:.3f}s")
    print(json.dumps({"tag": a.tag, "tenants": a.tenants, "hit_rate_pct": round(hr, 1),
                      "mean_latency_s": round(ml, 3), "hits": hits, "total": tot}))

if __name__ == "__main__":
    main()
