# Encoders on the Neural Engine (CoreML EP) — measured, rejected

**Question.** Arxiv 2606.22283 (an Apple Neural Engine study) concludes that encoders and
embedding models are the ANE's strong regime (1.5–4.4× faster than the GPU below batch ≈ 23).
The gateway serves `bge-m3` and `bge-reranker-base` through `fastembed` (ONNX Runtime). Can
those move to the ANE through ONNX Runtime's CoreML execution provider?

**Answer: no.** Through the CoreML EP the ANE is either not used at all or slower than the CPU.
The gateway stays on the ONNX CPU provider. (The old comments saying "ONNX Runtime + CoreML" were
wrong: no execution provider was ever configured, so it was always CPU.)

Measured 2026-10-03 on the M4 Pro Mac mini, MLX LLM resident, `ort 2.0.0-rc.11` (Rust) and
`onnxruntime 1.30.0` (Python). Rankings were identical in every configuration; this is a speed
finding, not a quality one.

## 1 — CoreML EP as-is (dynamic shapes, Rust, fastembed)

| model | CPU p50 | CoreML (CPUAndNeuralEngine, MLProgram) |
|---|---|---|
| bge-m3 | short 18–22 ms · medium 31–44 ms · 32×medium 0.65–0.96 s | **fails to load**: the fp32 model is > 2 GB, so its weights are external (`model.onnx_data`) and the CoreML EP resolves them as `model.onnx/model.onnx_data` ("Not a directory"); a retry fails compiling the model package |
| bge-reranker-base, 8 docs | 49.9 ms | **248.8 ms (5× slower)** — the ANE runtime rejects every layer ("unbounded dimension is not supported"), CoreML falls back to its CPU path |

## 2 — Static shapes (Python, reranker fixed to 1×128)

| execution | p50 | logit | on the ANE? |
|---|---|---|---|
| ORT CPU (what the gateway does) | 19.5–21.8 ms | -2.28221345 | — |
| CoreML MLProgram, CPUAndNeuralEngine | 20.4 ms | -2.28222132 | **no** — bit-identical to CPUOnly (same logit, same 7.9e-6 drift, same speed) |
| CoreML MLProgram, CPUOnly | 20.6 ms | -2.28222132 | — |
| CoreML MLProgram, ALL | 11.3 ms | -2.28221226 | uses the GPU — rejected, it would queue behind the MLX LLM |
| CoreML NeuralNetwork, CPUAndNeuralEngine | **57.7 ms** | -2.27763128 | **yes** (fp16 drift 0.0046) — and 2.6× slower than CPU |

Likely cause (inferred, not traced): ORT builds MLProgram models at fp32 precision and the ANE
executes fp16 only. The NeuralNetwork format does reach it, but the dispatch cost outweighs the compute at these sizes (2606.22283 measures a
~0.23 ms per-dispatch floor on M1). Every CoreML session also spends 75–85 s compiling on first load.

## If this is revisited

Only a native Core ML model would change the result: convert with coremltools in fp16 with
enumerated sequence lengths and the ANE-friendly layout (Apple's `ml-ane-transformers` recipe),
served by a Swift component, and confirm placement with `MLComputePlan` (whisperkittools writes one
`*.mlcomputeplan.json` per model). That is a project, not a flag, and the CPU path already answers a query in ~20 ms.
