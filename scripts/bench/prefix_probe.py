#!/usr/bin/env python3
"""
Measure the KV / prompt-prefix cache win — the thing Tier-2 exploits.

Medical agents (Eir/CDS) resend a big system prompt every turn. If the backend
reuses the KV of that shared prefix, the 2nd+ turn skips re-prefilling it.

Two conditions, sequential (so the backend's single-slot prompt cache is in play):
  WARM: same long system prompt every request -> prefix should be cached
  COLD: a UNIQUE long system prompt every request -> cache-busted, full prefill

Reports per-request latency + usage.prompt_tokens / usage.cached_tokens so we
can see the cache actually engage. Latency delta WARM vs COLD = the KV-reuse win.
"""
import argparse, json, time, sys
import urllib.request, urllib.error

def make_system(n_para, tag=""):
    para = ("You are a careful clinical decision-support assistant. Follow the "
            "hypertension and dyslipidemia guidelines. Always cite the guideline "
            "section. Never invent drug doses. Consider contraindications and "
            "renal function before recommending. ")
    return (f"[ctx {tag}] " if tag else "") + (para * n_para)

def one(url, api_key, system, user, max_tokens):
    body = json.dumps({
        "model": "gemma-4-26b",
        "messages": [{"role": "system", "content": system},
                     {"role": "user", "content": user}],
        "max_tokens": max_tokens, "temperature": 0.0, "stream": False,
    }).encode()
    headers = {"Content-Type": "application/json"}
    if api_key: headers["Authorization"] = f"Bearer {api_key}"
    t0 = time.perf_counter()
    req = urllib.request.Request(url, data=body, headers=headers, method="POST")
    with urllib.request.urlopen(req, timeout=600) as resp:
        d = json.loads(resp.read())
    lat = time.perf_counter() - t0
    u = d.get("usage", {})
    return lat, u.get("prompt_tokens"), (u.get("prompt_tokens_details") or {}).get("cached_tokens")

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default="http://127.0.0.1:8090/v1/chat/completions")
    ap.add_argument("--api-key", default="")
    ap.add_argument("--para", type=int, default=60, help="system-prompt size (paragraph repeats)")
    ap.add_argument("--rounds", type=int, default=5)
    ap.add_argument("--max-tokens", type=int, default=8)
    a = ap.parse_args()

    questions = [f"Question {i}: summarize in one word." for i in range(a.rounds + 2)]

    print(f"system-prompt size ~= {len(make_system(a.para))} chars\n")

    # WARM: identical system prompt each round -> prefix cache should hit
    warm = []
    sys_shared = make_system(a.para)
    for i in range(a.rounds):
        lat, pt, ct = one(a.url, a.api_key, sys_shared, questions[i], a.max_tokens)
        warm.append(lat)
        print(f"WARM round {i}: {lat:.3f}s  prompt_tokens={pt} cached_tokens={ct}")

    print()
    # COLD: unique system prompt each round -> cache busted every time
    cold = []
    for i in range(a.rounds):
        lat, pt, ct = one(a.url, a.api_key, make_system(a.para, tag=f"u{i}-{time.time_ns()}"),
                          questions[i], a.max_tokens)
        cold.append(lat)
        print(f"COLD round {i}: {lat:.3f}s  prompt_tokens={pt} cached_tokens={ct}")

    # steady-state = drop the first (both pay one full prefill)
    wm = sum(warm[1:]) / max(1, len(warm) - 1)
    cm = sum(cold[1:]) / max(1, len(cold) - 1)
    print("\n=== KV / prompt-prefix reuse ===")
    print(f"WARM steady mean: {wm:.3f}s   (shared prefix reused)")
    print(f"COLD steady mean: {cm:.3f}s   (prefix re-prefilled each time)")
    if wm > 0:
        print(f"speedup: {cm/wm:.2f}x  ·  saved {cm-wm:.3f}s/req on a {len(sys_shared)}-char system prompt")

if __name__ == "__main__":
    main()
