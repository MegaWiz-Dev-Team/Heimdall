# Meeting / Decision Records
> ISO/IEC 29110 Basic Profile — Project Management Process

## Records

### MR-001: Project Kickoff — 2026-03-02

**Participants**: User, Antigravity AI

**Decisions**:
1. Project จะใช้ MLX + vLLM เป็น inference backend หลัก
2. เครื่อง target: Mac Mini M4 Pro 64GB RAM
3. ต้องการเปรียบเทียบ Rust-based inference (mistral.rs / Candle)
4. ต้องการ Rust API Gateway จัดการ traffic
5. ใช้ ISO 29110 Basic Profile สำหรับ project documentation
6. ใช้ TDD approach ตลอดการพัฒนา

**Action Items**:
- [ ] Finalize engine selection (vllm-mlx vs mistral.rs)
- [ ] Finalize gateway framework (Axum vs Actix-web)
- [ ] Complete requirements specification
- [ ] Complete software design document

---

### MR-002: Architecture Decision — 2026-03-02

**Participants**: User, Antigravity AI

**Context**: Deep analysis revealed 64GB RAM = only ~48GB usable for GPU. Cannot run 3 engines simultaneously.

**Decision**: **Option A — Single Engine + Rust Gateway**
- Primary engine: **vllm-mlx** (fastest, multimodal, MoE support)
- Default model: **Qwen3.5-35B-A3B-Instruct-4bit** (~20GB, MoE = fast)
- Gateway: **Rust (Axum)** at `:3000` → proxy to vllm-mlx `:8000`
- No multi-engine concurrent — single engine for max RAM utilization

**Rationale**:
- MoE model uses only ~20GB → leaves ~28GB for KV cache = high concurrency
- Axum has `axum-reverse-proxy` crate for SSE streaming
- Single engine = simpler, more reliable, full RAM available

**Action Items**:
- [/] Build Rust API gateway (Axum)
- [ ] Setup vllm-mlx with Qwen3.5-35B-A3B
- [ ] Create operation scripts
- [ ] Benchmark on actual hardware

---

*Add new records below this line*
