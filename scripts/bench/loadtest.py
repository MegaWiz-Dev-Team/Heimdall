#!/usr/bin/env python3
"""
Heimdall gateway load test — measures behavior under concurrency.

Fires `--requests` chat-completions at `--url` with `--concurrency` in flight
at once (a burst that overloads a single-sequence backend on purpose), then
reports latency percentiles, status-class breakdown, throughput, and how the
gateway shed excess load (429/503 + queue-wait header).

Stdlib only (urllib) so it runs anywhere. Non-streaming so total latency is
the clean single number for the admission-queue A/B.
"""
import argparse, json, time, sys, statistics
from concurrent.futures import ThreadPoolExecutor, as_completed
import urllib.request, urllib.error

def one_request(url, api_key, model, prompt, max_tokens):
    body = json.dumps({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
        "temperature": 0.0,
        "stream": False,
    }).encode()
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    t0 = time.perf_counter()
    try:
        req = urllib.request.Request(url, data=body, headers=headers, method="POST")
        with urllib.request.urlopen(req, timeout=600) as resp:
            qw = resp.headers.get("x-heimdall-queue-wait-ms")
            resp.read()
            return {"status": resp.status, "latency": time.perf_counter() - t0,
                    "queue_wait_ms": float(qw) if qw else None, "err": None}
    except urllib.error.HTTPError as e:
        qw = e.headers.get("x-heimdall-queue-wait-ms") if e.headers else None
        try: e.read()
        except Exception: pass
        return {"status": e.code, "latency": time.perf_counter() - t0,
                "queue_wait_ms": float(qw) if qw else None, "err": None}
    except Exception as e:
        return {"status": 0, "latency": time.perf_counter() - t0,
                "queue_wait_ms": None, "err": type(e).__name__}

def pct(xs, p):
    if not xs: return None
    xs = sorted(xs); k = (len(xs)-1)*p/100.0
    f = int(k); c = min(f+1, len(xs)-1)
    return xs[f] + (xs[c]-xs[f])*(k-f)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default="http://127.0.0.1:8090/v1/chat/completions")
    ap.add_argument("--api-key", default="")
    ap.add_argument("--model", default="gemma-4-26b")
    ap.add_argument("--concurrency", type=int, default=30)
    ap.add_argument("--requests", type=int, default=60)
    ap.add_argument("--max-tokens", type=int, default=16)
    ap.add_argument("--prompt", default="Reply with a single word: ok.")
    ap.add_argument("--tag", default="run")
    ap.add_argument("--out", default="")
    a = ap.parse_args()

    # warmup (not counted) — pays the first-token / route-resolution cost once
    print(f"[{a.tag}] warmup…", file=sys.stderr)
    w = one_request(a.url, a.api_key, a.model, a.prompt, a.max_tokens)
    print(f"[{a.tag}] warmup status={w['status']} latency={w['latency']:.2f}s err={w['err']}", file=sys.stderr)

    print(f"[{a.tag}] firing {a.requests} reqs @ concurrency {a.concurrency} "
          f"(max_tokens={a.max_tokens})…", file=sys.stderr)
    results = []
    wall0 = time.perf_counter()
    with ThreadPoolExecutor(max_workers=a.concurrency) as ex:
        futs = [ex.submit(one_request, a.url, a.api_key, a.model, a.prompt, a.max_tokens)
                for _ in range(a.requests)]
        for f in as_completed(futs):
            results.append(f.result())
    wall = time.perf_counter() - wall0

    ok = [r for r in results if 200 <= r["status"] < 300]
    c2 = len(ok)
    c4 = len([r for r in results if 400 <= r["status"] < 500])
    c5 = len([r for r in results if 500 <= r["status"] < 600])
    cerr = len([r for r in results if r["status"] == 0])
    n429 = len([r for r in results if r["status"] == 429])
    n503 = len([r for r in results if r["status"] == 503])
    ok_lat = [r["latency"] for r in ok]
    all_lat = [r["latency"] for r in results]
    qwaits = [r["queue_wait_ms"] for r in ok if r["queue_wait_ms"] is not None]

    summary = {
        "tag": a.tag, "url": a.url, "concurrency": a.concurrency,
        "requests": a.requests, "max_tokens": a.max_tokens,
        "wall_s": round(wall, 3),
        "throughput_rps": round(len(results)/wall, 3) if wall else None,
        "ok_2xx": c2, "client_4xx": c4, "server_5xx": c5, "conn_err": cerr,
        "n_429": n429, "n_503": n503,
        "ok_latency_s": {
            "min": round(min(ok_lat), 3) if ok_lat else None,
            "p50": round(pct(ok_lat, 50), 3) if ok_lat else None,
            "p90": round(pct(ok_lat, 90), 3) if ok_lat else None,
            "p95": round(pct(ok_lat, 95), 3) if ok_lat else None,
            "p99": round(pct(ok_lat, 99), 3) if ok_lat else None,
            "max": round(max(ok_lat), 3) if ok_lat else None,
            "mean": round(statistics.mean(ok_lat), 3) if ok_lat else None,
        },
        "all_latency_p99_s": round(pct(all_lat, 99), 3) if all_lat else None,
        "queue_wait_ms": {
            "n": len(qwaits),
            "p50": round(pct(qwaits, 50), 1) if qwaits else None,
            "p99": round(pct(qwaits, 99), 1) if qwaits else None,
            "max": round(max(qwaits), 1) if qwaits else None,
        },
    }
    print(json.dumps(summary, indent=2))
    if a.out:
        with open(a.out, "w") as fh:
            json.dump(summary, fh, indent=2)
        print(f"[{a.tag}] wrote {a.out}", file=sys.stderr)

if __name__ == "__main__":
    main()
