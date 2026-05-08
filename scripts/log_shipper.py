#!/usr/bin/env python3
import os
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
        
    return {
        "@timestamp": timestamp,
        "service": service,
        "level": level,
        "message": message,
        "host": "macos-host"
    }

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
