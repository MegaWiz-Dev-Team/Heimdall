# Requirements Specification
> ISO/IEC 29110 Basic Profile — Software Implementation Process

## 1. Document Control

| Field | Value |
|:--|:--|
| **Document ID** | SI-REQ-001 |
| **Version** | 0.1 (Draft) |
| **Last Updated** | 2026-03-02 |
| **Author** | — |
| **Status** | 🟡 Draft |

---

## 2. Functional Requirements

### Engine Layer

| Req ID | Priority | Requirement | Acceptance Criteria |
|:--|:--|:--|:--|
| REQ-001 | MUST | ระบบต้องรัน LLM inference ผ่าน **vllm-mlx** ได้ | Start server → call `/v1/chat/completions` → ได้ response |
| REQ-002 | MUST | ระบบต้องรัน LLM inference ผ่าน **mistral.rs** ได้ | Start server → call `/v1/chat/completions` → ได้ response |
| REQ-003 | SHOULD | ระบบต้องรัน LLM inference ผ่าน **mlx_lm.server** ได้ | Start server → call API → ได้ response |
| REQ-004 | MUST | ทุก engine ต้อง expose **OpenAI-compatible API** | `/v1/models`, `/v1/chat/completions` ทำงานได้ |
| REQ-005 | MUST | ต้องรองรับ **streaming** response (SSE) | ส่ง `"stream": true` → ได้ chunked response |
| REQ-006 | SHOULD | ต้อง load/unload model ได้โดยไม่ต้อง restart server | API call เปลี่ยน model → model ใหม่ active |

### API Gateway Layer

| Req ID | Priority | Requirement | Acceptance Criteria |
|:--|:--|:--|:--|
| REQ-010 | MUST | มี **Rust API Gateway** ทำ reverse proxy ไปยัง backend engines | Request ผ่าน gateway → forward ไปยัง engine → response กลับ |
| REQ-011 | MUST | Gateway ต้อง route request ไปยัง engine ที่เลือกได้ | ส่ง header `X-Engine: vllm-mlx` → route ไป vllm-mlx |
| REQ-012 | MUST | Gateway ต้องมี **API key authentication** | Request ไม่มี key → 401, มี key ถูก → pass through |
| REQ-013 | SHOULD | Gateway ต้องมี **rate limiting** | เกิน limit → 429 Too Many Requests |
| REQ-014 | MUST | Gateway ต้อง **health check** backend engines | Backend ล่ม → auto remove, กลับมา → auto add |
| REQ-015 | MUST | Gateway ต้อง proxy **SSE streaming** ได้ | Streaming response pass-through ถูกต้อง |
| REQ-016 | SHOULD | Gateway ต้องมี **metrics endpoint** `/metrics` | Prometheus scrape ได้ — latency, throughput, errors |

### Operations

| Req ID | Priority | Requirement | Acceptance Criteria |
|:--|:--|:--|:--|
| REQ-020 | MUST | มี script **setup** environment ได้ด้วย 1 command | `./scripts/setup.sh` → environment พร้อมใช้ |
| REQ-021 | MUST | มี script **start/stop** server ได้ | `./scripts/start.sh` / `./scripts/stop.sh` ทำงาน |
| REQ-022 | MUST | มี **benchmark script** เปรียบเทียบ engine ได้ | `./scripts/benchmark.sh` → output TTFT, TPS, throughput |
| REQ-023 | SHOULD | Network clients ใน LAN ต้องเข้าถึง API ได้ | `curl http://<ip>:3000/v1/models` จาก machine อื่น → response |

---

## 3. Non-Functional Requirements

| Req ID | Priority | Requirement | Target |
|:--|:--|:--|:--|
| NFR-001 | MUST | Gateway latency overhead | < 1ms per request |
| NFR-002 | MUST | Memory — model + gateway + OS | ≤ 60GB (เหลือ buffer 4GB) |
| NFR-003 | SHOULD | Gateway concurrent connections | ≥ 100 simultaneous |
| NFR-004 | MUST | First token latency (TTFT) | < 500ms for default model |
| NFR-005 | MUST | Test coverage (Rust code) | ≥ 80% |

---

## 4. Traceability Matrix

> จะเติมเมื่อ Design + Test Cases พร้อม

| Req ID | Design Component | Test Case ID |
|:--|:--|:--|
| REQ-001 | — | — |
| REQ-010 | — | — |
| ... | ... | ... |
