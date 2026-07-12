#!/usr/bin/env python3
"""Run a fixed sanity-check prompt set against one backend (greedy/temp=0)
and save outputs. Run once vs llama, once vs mlx, then diff the two files.
Sends chat_template_kwargs.enable_thinking:false exactly like the Heimdall
gateway does, so this compares production-shaped behavior."""
import json, sys, time, urllib.request

PROMPTS = [
    ("factual",     "What is the capital of Australia? Answer with just the city name."),
    ("reasoning",   "If a train travels 60 km in 45 minutes, what is its speed in km/h? Show the calculation briefly."),
    ("format",      "List exactly the 3 additive primary colors of light as one comma-separated line, nothing else."),
    ("thai",        "อธิบายภาวะหยุดหายใจขณะหลับจากการอุดกั้น (OSA) แบบสั้น 2 ประโยค เป็นภาษาไทย"),
    ("clinical",    "A 55-year-old with BP 158/96 and no comorbidities. Per standard hypertension guidelines, give the initial lifestyle advice and one reasonable first-line antihypertensive drug class. Be concise."),
    ("json",        'Return ONLY a JSON object with keys "name" (string) and "age" (number) for a person named Alice who is 30. No prose.'),
    ("trap",        "Which is heavier: one kilogram of feathers or one kilogram of steel? Answer in one sentence."),
]

def ask(url, prompt, max_tokens=220):
    body = json.dumps({
        "model": "gemma-4-26b",
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens, "temperature": 0.0,
        "chat_template_kwargs": {"enable_thinking": False}, "stream": False,
    }).encode()
    r = urllib.request.Request(url, data=body, method="POST",
                               headers={"Content-Type": "application/json"})
    t0 = time.perf_counter()
    with urllib.request.urlopen(r, timeout=180) as resp:
        d = json.loads(resp.read())
    lat = time.perf_counter() - t0
    return d["choices"][0]["message"]["content"], lat

def main():
    url = sys.argv[1]; label = sys.argv[2]; out = sys.argv[3]
    results = {}
    for key, p in PROMPTS:
        try:
            txt, lat = ask(url, p)
        except Exception as e:
            txt, lat = f"<ERROR: {type(e).__name__}: {e}>", 0.0
        results[key] = {"prompt": p, "output": txt.strip(), "latency_s": round(lat, 2)}
        print(f"[{label}] {key} ({lat:.1f}s): {txt.strip()[:120].replace(chr(10),' ')}")
    json.dump(results, open(out, "w"), ensure_ascii=False, indent=2)
    print(f"[{label}] wrote {out}")

if __name__ == "__main__":
    main()
