# Heimdall — Model Catalog
> MLX-compatible models for Mac Mini M4 Pro 64GB

## Quick Reference

| Symbol | Meaning |
|:--|:--|
| ⭐ | Recommended for Heimdall |
| 🏥 | Medical domain |
| 🖼️ | Vision / Multimodal |
| ⚡ | MoE (fast inference) |

---

## 📦 Model Storage

- **Location**: `~/.cache/huggingface/hub/`
- **Download**: Auto on first use, or `huggingface-cli download <model-id>`
- **Format**: MLX (safetensors), quantized by [mlx-community](https://huggingface.co/mlx-community)

---

## 1. Alibaba — Qwen

| Model | Size (4-bit) | Type | Notes |
|:--|:--|:--|:--|
| ⭐⚡ `mlx-community/Qwen3.5-35B-A3B-Instruct-4bit` | ~20 GB | MoE | **Default** — เร็วมาก, ฉลาด, MoE ใช้ RAM น้อย |
| ⚡ `mlx-community/Qwen3.5-122B-A10B-Instruct-4bit` | ~60 GB | MoE | ตัวใหญ่สุด — ใช้เกือบเต็ม RAM |
| `mlx-community/Qwen2.5-72B-Instruct-4bit` | ~42 GB | Dense | Thai ดี, reasoning ดี |
| `mlx-community/Qwen2.5-32B-Instruct-4bit` | ~18 GB | Dense | สมดุลขนาด/คุณภาพ |
| `mlx-community/Qwen2.5-14B-Instruct-4bit` | ~8 GB | Dense | เบา, ตอบเร็ว |
| `mlx-community/Qwen2.5-7B-Instruct-4bit` | ~4 GB | Dense | เร็วมาก (~100+ tok/s) |
| `mlx-community/Qwen2.5-Coder-32B-Instruct-4bit` | ~18 GB | Dense | ⭐ เขียน code เก่งสุด |
| `mlx-community/Qwen2.5-Coder-7B-Instruct-4bit` | ~4 GB | Dense | Code model เบา |

---

## 2. Google — Gemma

| Model | Size (4-bit) | Type | Notes |
|:--|:--|:--|:--|
| ⭐ `mlx-community/gemma-3-27b-it-4bit` | ~15 GB | Dense | Gemma 3 ตัวใหญ่ — ฉลาด, multimodal ready |
| `mlx-community/gemma-3-12b-it-4bit` | ~7 GB | Dense | สมดุลขนาด/คุณภาพ |
| `mlx-community/gemma-3-4b-it-4bit` | ~2.5 GB | Dense | เล็ก เร็ว, เหมาะ edge |
| `mlx-community/gemma-3-1b-it-4bit` | ~0.6 GB | Dense | จิ๋วสุด — สำหรับ prototype |
| 🖼️ `mlx-community/gemma-3-27b-it-4bit` | ~15 GB | Vision | รองรับ image input (ใช้กับ mlx-vlm) |
| 🖼️ `mlx-community/gemma-3-12b-it-4bit` | ~7 GB | Vision | Vision เบา |

---

## 3. Meta — Llama

| Model | Size (4-bit) | Type | Notes |
|:--|:--|:--|:--|
| ⭐ `mlx-community/Llama-3.3-70B-Instruct-4bit` | ~40 GB | Dense | ตัวท็อป — เทียบ GPT-4o, reasoning เก่ง |
| `mlx-community/Meta-Llama-3.1-70B-Instruct-4bit` | ~40 GB | Dense | 3.1 version, multilingual ดี |
| `mlx-community/Meta-Llama-3.1-8B-Instruct-4bit` | ~5 GB | Dense | เบา เร็ว |
| `mlx-community/Meta-Llama-3.1-8B-Instruct-8bit` | ~9 GB | Dense | 8-bit ความแม่นยำสูง |
| 🖼️ `mlx-community/Llama-3.2-11B-Vision-Instruct-4bit` | ~8 GB | Vision | วิเคราะห์ภาพ |
| 🖼️ `mlx-community/Llama-3.2-90B-Vision-Instruct-4bit` | ~50 GB | Vision | Vision ตัวใหญ่ |
| `mlx-community/Llama-3.2-3B-Instruct-4bit` | ~2 GB | Dense | จิ๋ว เร็ว |
| `mlx-community/Llama-3.2-1B-Instruct-4bit` | ~0.7 GB | Dense | เล็กสุด Llama |

---

## 4. DeepSeek

| Model | Size (4-bit) | Type | Notes |
|:--|:--|:--|:--|
| ⭐ `mlx-community/DeepSeek-R1-Distill-Llama-70B-4bit` | ~40 GB | Dense | Reasoning, math, logic เก่งมาก |
| `mlx-community/DeepSeek-R1-Distill-Qwen-32B-4bit` | ~18 GB | Dense | R1 reasoning ขนาดกลาง |
| `mlx-community/DeepSeek-R1-Distill-Qwen-14B-4bit` | ~8 GB | Dense | R1 reasoning เบา |
| `mlx-community/DeepSeek-R1-Distill-Qwen-7B-4bit` | ~4 GB | Dense | R1 reasoning จิ๋ว |
| ⚡ `mlx-community/DeepSeek-V3-0324-4bit` | ~40 GB | MoE | V3 เต็ม, MoE |

---

## 5. Mistral / Mixtral

| Model | Size (4-bit) | Type | Notes |
|:--|:--|:--|:--|
| ⚡ `mlx-community/Mixtral-8x7B-Instruct-v0.1-4bit` | ~25 GB | MoE | MoE ตัวแรกๆ |
| `mlx-community/Mistral-7B-Instruct-v0.3-4bit` | ~4 GB | Dense | เล็ก เร็ว คุณภาพดี |
| `mlx-community/Mistral-Small-24B-Instruct-2501-4bit` | ~14 GB | Dense | Mistral Small ใหม่ |

---

## 6. 🏥 Medical / Healthcare Models

> ⚠️ **หมายเหตุ**: Medical models ส่วนใหญ่ยังไม่มี MLX format โดยตรง — ต้อง convert ด้วย `mlx_lm.convert`

### พร้อมใช้บน MLX (มี mlx-community version)

| Model | Base | Size | Notes |
|:--|:--|:--|:--|
| `mlx-community/Llama-3.1-8B-Instruct-4bit` + RAG | Llama 3.1 | ~5 GB | ใช้กับ medical RAG (PubMed, UpToDate) |
| `mlx-community/Qwen2.5-72B-Instruct-4bit` + RAG | Qwen 2.5 | ~42 GB | Thai medical + RAG ดี |

### ต้อง Convert เป็น MLX (ใช้ `mlx_lm.convert`)

| Model | Source | Base | Notes |
|:--|:--|:--|:--|
| 🏥 **MedGemma** | `google/medgemma-4b-it` | Gemma | Google medical model, MedQA ~91% |
| 🏥 **Meditron-7B** | `epfl-llm/meditron-7b` | Llama 2 | Train บน PubMed + medical guidelines |
| 🏥 **Meditron-70B** | `epfl-llm/meditron-70b` | Llama 2 | ตัวใหญ่ medical — by EPFL + Yale |
| 🏥 **BioMistral-7B** | `BioMistral/BioMistral-7B` | Mistral | Biomedical domain |
| 🏥 **OpenBioLLM-70B** | `aaditya/OpenBioLLM-Llama3-70B` | Llama 3 | Medical + bio, ผลดีใน benchmarks |
| 🏥 **OpenBioLLM-8B** | `aaditya/OpenBioLLM-Llama3-8B` | Llama 3 | เวอร์ชันเบา |
| 🏥 **Med42-v2-70B** | `m42-health/Llama3-Med42-70B` | Llama 3 | Clinical reasoning |
| 🏥 **ClinicalCamel-70B** | `wanglab/ClinicalCamel-70B` | Llama 2 | Clinical NLP |
| 🏥 **PMC-LLaMA-13B** | `axiong/PMC_LLaMA_13B` | Llama | Train บน 4.8M PubMed papers |

### วิธี Convert Medical Model เป็น MLX

```bash
source ~/mlx_env/bin/activate
pip install mlx-lm

# Convert model เป็น MLX 4-bit
mlx_lm.convert \
  --hf-path epfl-llm/meditron-7b \
  -q \
  --q-bits 4 \
  --upload-repo mimir/meditron-7b-mlx-4bit
```

---

## 7. ⚡ RAM Planning Guide

| Available RAM | Model ที่แนะนำ | KV Cache เหลือ |
|:--|:--|:--|
| **~48 GB** (usable) | | |
| ใช้ ~4 GB | Qwen2.5-7B → **~44 GB** cache | สูงสุด, concurrency สูง |
| ใช้ ~8 GB | Gemma-3-12B, Llama-8B → **~40 GB** cache | ดีมาก |
| ใช้ ~18 GB | Qwen-Coder-32B → **~30 GB** cache | ดี |
| ⭐ ใช้ ~20 GB | **Qwen3.5-35B MoE** → **~28 GB** cache | สมดุลดีที่สุด |
| ใช้ ~40 GB | Llama-70B → **~8 GB** cache | จำกัด concurrency |
| ใช้ ~60 GB | Qwen3.5-122B MoE → **~0 GB** cache | ⚠️ อาจ swap |

---

## 8. วิธี Download

```bash
# Download model ล่วงหน้า (ไม่ต้อง start server)
huggingface-cli download mlx-community/Qwen3.5-35B-A3B-Instruct-4bit

# ดู models ที่ download แล้ว
ls ~/.cache/huggingface/hub/ | grep models

# ลบ model ที่ไม่ใช้
huggingface-cli delete-cache
```

---

*Last updated: 2026-03-03 | Heimdall v0.1.0*
