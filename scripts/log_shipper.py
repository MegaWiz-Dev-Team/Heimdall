#!/usr/bin/env python3
import os
import re
import time
import json
import subprocess
import urllib.request
import urllib.error
from datetime import datetime, timezone

# ── Configuration ─────────────────────────────────────────────
INDEXER_URL = os.environ.get("WAZUH_INDEXER_URL", "https://127.0.0.1:30920")
INDEXER_USER = os.environ.get("WAZUH_INDEXER_USER", "admin")
INDEXER_PASS = os.environ.get("WAZUH_INDEXER_PASS", "admin")

PROJECT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
LOG_FILES = {
    "heimdall_gateway": os.environ.get("HEIMDALL_GATEWAY_LOG", os.path.join(PROJECT_DIR, "logs/gateway-stdout.log")),
    "heimdall_mlx": os.environ.get("HEIMDALL_MLX_LOG", os.path.join(PROJECT_DIR, "logs/mlx-stdout.log")),
}

BATCH_SIZE = 50
BATCH_TIMEOUT_SEC = 2.0

# ── HTTP Setup ────────────────────────────────────────────────
import ssl
import base64

ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE

auth_str = f"{INDEXER_USER}:{INDEXER_PASS}".encode("utf-8")
auth_header = f"Basic {base64.b64encode(auth_str).decode('utf-8')}"

def send_bulk(logs):
    if not logs: return
    
    # Generate daily index e.g., heimdall-logs-2026.04.16
    today_str = datetime.now(timezone.utc).strftime("%Y.%m.%d")
    index_name = f"heimdall-logs-{today_str}"
    
    bulk_data = ""
    for log in logs:
        action = {"index": {"_index": index_name}}
        bulk_data += json.dumps(action) + "\n"
        bulk_data += json.dumps(log) + "\n"

    req = urllib.request.Request(
        f"{INDEXER_URL}/_bulk",
        data=bulk_data.encode("utf-8"),
        headers={
            "Content-Type": "application/x-ndjson",
            "Authorization": auth_header
        },
        method="POST"
    )
    
    try:
        urllib.request.urlopen(req, context=ctx)
        print(f"[{datetime.now(timezone.utc).isoformat()}] Sent {len(logs)} logs to index {index_name}", flush=True)
    except urllib.error.URLError as e:
        print(f"Failed to send logs: {e}", flush=True)

# ── 🌑 Skuggi DLP event classifier (→ Tyr/Wazuh) ──────────────
# Turns Heimdall's `🌑 skuggi[...]` tracing lines into structured, severity-
# tagged DLP events so Tyr can alert on egress attempts / unredacted-PII leaks.
# Severity mirrors the DLP risk of the call (see deploy/tyr/README.md).
ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")
SKUGGI_RE = re.compile(
    r"skuggi\[(?P<mode>[\w-]+)\]\s*(?P<evt>EGRESS BLOCKED|BLOCKED)?"
    r'.*?tenant=(?:Some\("(?P<tenant>[^"]*)"\)|(?P<tenant2>None|[\w-]+))'
    r"(?:.*?provider=(?P<provider>\S+))?"
    r"(?:.*?model=(?P<model>\S+))?"
    r"(?:.*?detections=(?P<det>\S+))?"
)

def classify_skuggi(msg):
    """Return extra structured fields for a Skuggi log line, or None."""
    msg = ANSI_RE.sub("", msg)
    if "skuggi[" not in msg:
        if "tenant config cache connected to MariaDB" in msg:
            return {"event_category": "skuggi_dlp", "skuggi_event": "db_connected",
                    "security_severity": "info", "wazuh_level": 2}
        low = msg.lower()
        if "skuggi" in low and any(k in low for k in ("fail", "error", "fall")):
            return {"event_category": "skuggi_dlp", "skuggi_event": "error_or_fallback",
                    "security_severity": "high", "wazuh_level": 9,
                    "skuggi_note": "policy DB/tier issue → fail-open risk"}
        return None
    m = SKUGGI_RE.search(msg)
    if not m:
        return {"event_category": "skuggi_dlp", "skuggi_event": "unparsed",
                "security_severity": "low", "wazuh_level": 3}
    mode = m.group("mode")
    blocked = bool(m.group("evt"))
    tenant = m.group("tenant") or m.group("tenant2") or "unknown"
    det = m.group("det")
    has_pii = bool(det) and det not in ("", "none")

    if mode == "local-only":
        sev, lvl, why = "notice", 6, "no-cloud tenant attempted external egress (blocked)"
    elif mode == "block-on-pii" and blocked:
        sev, lvl, why = "notice", 7, "PII detected on strict tenant → call rejected"
    elif mode == "detect-only" and has_pii:
        sev, lvl, why = "high", 10, "PII forwarded UNREDACTED to cloud (detect-only)"
    elif mode == "off":
        sev, lvl, why = "high", 10, "redaction disabled — raw payload to cloud"
    elif mode == "mask-and-send":
        sev, lvl, why = ("low", 3, "PII redacted then forwarded") if has_pii else ("info", 2, "clean call forwarded")
    else:
        sev, lvl, why = "low", 3, "skuggi event"

    return {
        "event_category": "skuggi_dlp",
        "skuggi_mode": mode,
        "skuggi_decision": "blocked" if blocked else "forwarded",
        "skuggi_tenant": tenant,
        "skuggi_provider": m.group("provider"),
        "skuggi_model": m.group("model"),
        "skuggi_detections": det,
        "security_severity": sev,
        "wazuh_level": lvl,
        "skuggi_why": why,
    }


def parse_log_line(service, line):
    line = line.strip()
    if not line: return None
    
    # Attempt to extract rudimentary timestamp (e.g., 2026-04-16T...Z)
    parts = line.split(" ", 1)
    timestamp = datetime.now(timezone.utc).isoformat()
    message = line
    level = "INFO"
    
    if len(parts) == 2 and "T" in parts[0] and ("+" in parts[0] or "Z" in parts[0]):
        timestamp = parts[0]
        message = parts[1]

    upper_msg = message.upper()
    if "ERROR" in upper_msg or "FATAL" in upper_msg:
        level = "ERROR"
    elif "WARN" in upper_msg:
        level = "WARN"
    elif "DEBUG" in upper_msg:
        level = "DEBUG"
        
    doc = {
        "@timestamp": timestamp,
        "service": service,
        "level": level,
        "message": message,
        "host": "macos-host"
    }
    # Enrich Skuggi DLP events with structured fields + severity for Tyr.
    skuggi = classify_skuggi(message)
    if skuggi:
        doc.update(skuggi)
    return doc

def main():
    print(f"🚀 Started Heimdall -> Wazuh/Tyr Log Shipper", flush=True)
    print(f"Target: {INDEXER_URL}", flush=True)
    
    # Ensure files exist before tailing
    for f in LOG_FILES.values():
        if not os.path.exists(f):
            open(f, 'a').close()
            
    # Start tail -F on all files
    cmd = ["tail", "-c", "0", "-F"] + list(LOG_FILES.values())
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    
    current_service = "unknown"
    batch = []
    last_send = time.time()
    
    try:
        while True:
            # Non-blocking read trick (select equivalent via generic readline might block, we will just block on read and use timeout in read if possible, but python's readline blocks. However tailing multiple files outputs header "==> filename <===")
            line = process.stdout.readline()
            if not line: break
            
            line_str = line.strip()
            
            if line_str.startswith("==>") and line_str.endswith("<=="):
                filename = line_str.replace("==>", "").replace("<==", "").strip()
                for svc, path in LOG_FILES.items():
                    if filename == path:
                        current_service = svc
                        break
                continue
            
            # Not a header, so it's a log from the current_service
            if line_str:
                parsed = parse_log_line(current_service, line_str)
                if parsed:
                    batch.append(parsed)
            
            now = time.time()
            if len(batch) >= BATCH_SIZE or (now - last_send > BATCH_TIMEOUT_SEC and len(batch) > 0):
                send_bulk(batch)
                batch = []
                last_send = now

    except KeyboardInterrupt:
        process.terminate()

if __name__ == "__main__":
    main()
