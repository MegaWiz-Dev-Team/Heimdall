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

*Add new records below this line*
