# Heimdall — Model Catalog
> MLX-compatible models for Mac Mini M4 Pro 64GB

## Quick Reference

| Symbol | Meaning |
|:--|:--|
| ⭐ | Recommended for Heimdall |
| 🏥 | Medical domain |
| 🖼️ | Vision / Multimodal |
| ⚡ | MoE (fast inference) |
| 🔍 | Embedding model |
| 🎯 | Reranking model |

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

## 7. 🔍 Embedding Models

> ใช้สำหรับ semantic search, RAG, similarity matching
> ใช้ RAM น้อยมาก (<1 GB) — รันพร้อม LLM ได้สบาย

### MLX-Native (ใช้ `mlx-embedding-models` หรือ `mlx-embeddings`)

| Model | Dim | Size | Lang | Notes |
|:--|:--|:--|:--|:--|
| ⭐🔍 `nomic-ai/nomic-embed-text-v1.5` | 768 | ~270 MB | EN | ดีที่สุดในรุ่นเล็ก, Matryoshka support |
| 🔍 `BAAI/bge-large-en-v1.5` | 1024 | ~330 MB | EN | ⭐ Top-tier ค้นหาแม่นยำ |
| 🔍 `BAAI/bge-m3` | 1024 | ~570 MB | 100+ | ⭐ Multilingual (รวม Thai), dense+sparse |
| 🔍 `Alibaba-NLP/gte-large-en-v1.5` | 1024 | ~330 MB | EN | แม่นยำดี, competitive กับ bge |
| 🔍 `jinaai/jina-embeddings-v3` | 1024 | ~570 MB | 100+ | Multilingual, Matryoshka, task-type control |
| 🔍 `jinaai/jina-embeddings-v2-base-en` | 768 | ~270 MB | EN | 8K context window, เร็ว |
| 🔍 `sentence-transformers/all-MiniLM-L6-v2` | 384 | ~80 MB | EN | จิ๋ว เร็วมาก, prototype ดี |
| 🔍 `sentence-transformers/all-mpnet-base-v2` | 768 | ~420 MB | EN | สมดุลขนาด/คุณภาพ |
| 🔍 `intfloat/multilingual-e5-large` | 1024 | ~560 MB | 100+ | Multilingual ดี, รวม Thai |
| 🔍 `mixedbread-ai/mxbai-embed-large-v1` | 1024 | ~330 MB | EN | State-of-the-art ตัวใหม่ |

### วิธีใช้ Embedding บน MLX

```bash
pip install mlx-embedding-models

python3 -c "
from mlx_embedding_models.embedding import EmbeddingModel
model = EmbeddingModel.from_registry('bge-m3')
embeds = model.encode(['สวัสดีครับ', 'Hello world'])
print(f'Shape: {embeds.shape}')  # (2, 1024)
"
```

### วิธีใช้ผ่าน sentence-transformers (CPU/MPS)

```bash
pip install sentence-transformers

python3 -c "
from sentence_transformers import SentenceTransformer
model = SentenceTransformer('BAAI/bge-m3', device='mps')
embeds = model.encode(['สวัสดีครับ', 'Hello world'])
print(f'Shape: {embeds.shape}')  # (2, 1024)
"
```

---

## 8. 🎯 Reranking Models

> ใช้สำหรับ rerank ผลลัพธ์จาก search ให้แม่นยำขึ้น (2-stage retrieval)
> เบา (<1 GB) — ใช้ร่วมกับ embedding model + LLM ได้

### MLX-Native

| Model | Size | Lang | Notes |
|:--|:--|:--|:--|
| ⭐🎯 `mlx-community/mxbai-rerank-large-v2` | ~1.3 GB | Multi | MixedBread v2 — SOTA, fast, multilingual |
| 🎯 `jinaai/jina-reranker-v3-mlx` | ~600 MB | 100+ | MLX-native port, listwise reranker |
| 🎯 `jinaai/jina-reranker-v2-base-multilingual` | ~280 MB | 100+ | เบา, multilingual, code-aware |

### ใช้ผ่าน sentence-transformers (CPU/MPS)

| Model | Size | Lang | Notes |
|:--|:--|:--|:--|
| ⭐🎯 `BAAI/bge-reranker-v2-m3` | ~570 MB | 100+ | ⭐ แม่นยำดีที่สุด, multilingual |
| 🎯 `BAAI/bge-reranker-v2-gemma` | ~2.5 GB | Multi | Gemma-based, accuracy สูง |
| 🎯 `cross-encoder/ms-marco-MiniLM-L-6-v2` | ~80 MB | EN | จิ๋ว เร็วมาก, classic |
| 🎯 `cross-encoder/ms-marco-MiniLM-L-12-v2` | ~130 MB | EN | แม่นยำกว่า L-6 |
| 🎯 `mixedbread-ai/mxbai-rerank-base-v2` | ~450 MB | Multi | Base version, เบากว่า large |

### วิธีใช้ Reranker

```bash
pip install sentence-transformers

python3 -c "
from sentence_transformers import CrossEncoder
model = CrossEncoder('BAAI/bge-reranker-v2-m3', device='mps')

query = 'What is diabetes?'
docs = [
    'Diabetes is a chronic metabolic disease.',
    'The weather today is sunny.',
    'Type 2 diabetes affects insulin resistance.',
]

scores = model.predict([(query, doc) for doc in docs])
ranked = sorted(zip(docs, scores), key=lambda x: x[1], reverse=True)
for doc, score in ranked:
    print(f'{score:.3f} | {doc}')
"
```

### 🔗 Pipeline ตัวอย่าง: Embedding → Rerank → LLM

```
Query → [Embedding Model] → search top-50 docs
      → [Reranker] → rerank → top-5 docs
      → [LLM (Heimdall)] → generate answer with context
```

> 💡 ทั้ง embedding + reranker ใช้ RAM รวมกันแค่ ~1-2 GB
> สามารถรันพร้อม Qwen3.5-35B (20 GB) ได้สบาย → ยังเหลือ ~26 GB สำหรับ KV cache

---

## 9. ⚡ RAM Planning Guide

| Available RAM | Model ที่แนะนำ | KV Cache เหลือ |
|:--|:--|:--|
| **~48 GB** (usable) | | |
| ใช้ ~4 GB | Qwen2.5-7B → **~44 GB** cache | สูงสุด, concurrency สูง |
| ใช้ ~8 GB | Gemma-3-12B, Llama-8B → **~40 GB** cache | ดีมาก |
| ใช้ ~18 GB | Qwen-Coder-32B → **~30 GB** cache | ดี |
| ⭐ ใช้ ~20 GB | **Qwen3.5-35B MoE** → **~28 GB** cache | สมดุลดีที่สุด |
| ใช้ ~22 GB | Qwen3.5-35B + embedding + reranker → **~26 GB** | ยังดี |
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
