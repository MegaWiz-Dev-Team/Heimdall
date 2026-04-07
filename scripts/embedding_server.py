#!/usr/bin/env python3
"""MLX Embedding Server for Heimdall.

Provides a REST API for text embeddings using mlx-embedding-models.
Default model: BAAI/bge-m3 (registry key: bge-m3)

Endpoints:
  GET  /health           - Health check
  POST /v1/embeddings    - OpenAI-compatible embedding endpoint
"""

import os
import sys
import json
import time
import logging
from http.server import HTTPServer, BaseHTTPRequestHandler

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(message)s")
log = logging.getLogger("embedding")

# Configuration
MODEL_NAME = os.environ.get("EMBEDDING_MODEL", "BAAI/bge-m3")
PORT = int(os.environ.get("EMBEDDING_PORT", "8001"))

# Map full HuggingFace model names to mlx-embedding-models registry keys
REGISTRY_MAP = {
    "BAAI/bge-m3": "bge-m3",
    "BAAI/bge-small-en-v1.5": "bge-small",
    "BAAI/bge-large-en-v1.5": "bge-large",
}

registry_key = REGISTRY_MAP.get(MODEL_NAME, MODEL_NAME)

# Load model at startup
log.info(f"Loading embedding model: {MODEL_NAME} (registry: {registry_key})")
try:
    from mlx_embedding_models.embedding import EmbeddingModel
    import torch
    from transformers import AutoModelForSequenceClassification, AutoTokenizer
    
    model = EmbeddingModel.from_registry(registry_key)
    log.info(f"✅ Model loaded: {MODEL_NAME}")

    # Load Reranker Model
    RERANKER_NAME = os.environ.get("RERANKER_MODEL", "BAAI/bge-reranker-v2-m3")
    log.info(f"Loading reranker model: {RERANKER_NAME}")
    rerank_tokenizer = AutoTokenizer.from_pretrained(RERANKER_NAME)
    rerank_model = AutoModelForSequenceClassification.from_pretrained(RERANKER_NAME)
    try:
        device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
        rerank_model.to(device)
    except:
        device = "cpu"
        rerank_model.to(device)
    rerank_model.eval()
    log.info(f"✅ Reranker model loaded on {device}: {RERANKER_NAME}")

    # Patch for transformers >= 5.0 compatibility
    # transformers 5.x removed batch_encode_plus from PreTrainedTokenizerFast
    # but mlx-embedding-models 0.0.11 still calls it in _tokenize()
    if not hasattr(model.tokenizer, 'batch_encode_plus'):
        def _compat_batch_encode_plus(texts, **kwargs):
            return model.tokenizer(texts, **kwargs)
        model.tokenizer.batch_encode_plus = _compat_batch_encode_plus
        log.info("  ℹ️  Patched tokenizer.batch_encode_plus for transformers 5.x")

except KeyError:
    log.error(f"❌ Model '{registry_key}' not found in registry")
    log.error(f"   Try using from_pretrained() for HuggingFace models")
    log.info(f"   Falling back to from_pretrained('{MODEL_NAME}')...")
    try:
        model = EmbeddingModel.from_pretrained(MODEL_NAME)
        log.info(f"✅ Model loaded via from_pretrained: {MODEL_NAME}")
        # Same patch for from_pretrained path
        if not hasattr(model.tokenizer, 'batch_encode_plus'):
            def _compat_batch_encode_plus(texts, **kwargs):
                return model.tokenizer(texts, **kwargs)
            model.tokenizer.batch_encode_plus = _compat_batch_encode_plus
            log.info("  ℹ️  Patched tokenizer.batch_encode_plus for transformers 5.x")
    except Exception as e2:
        log.error(f"❌ from_pretrained also failed: {e2}")
        sys.exit(1)
except Exception as e:
    log.error(f"❌ Failed to load model: {e}")
    log.error("Install: pip install mlx-embedding-models")
    sys.exit(1)


class EmbeddingHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/health":
            self._json_response({"status": "ok", "embedding_model": MODEL_NAME, "reranker_model": "BAAI/bge-reranker-v2-m3"})
        else:
            self._json_response({"error": "Not found"}, 404)

    def do_POST(self):
        if self.path == "/v1/embeddings":
            self._handle_embeddings()
        elif self.path == "/v1/rerank" or self.path == "/rerank":
            self._handle_rerank()
        else:
            self._json_response({"error": "Not found"}, 404)

    def _handle_rerank(self):
        try:
            content_length = int(self.headers.get("Content-Length", 0))
            body = json.loads(self.rfile.read(content_length))
            query = body.get("query", "")
            texts = body.get("texts", [])
            if not query or not texts:
                return self._json_response({"error": "Missing query or texts"}, 400)

            pairs = [[query, text] for text in texts]
            
            import torch
            with torch.no_grad():
                inputs = rerank_tokenizer(pairs, padding=True, truncation=True, return_tensors='pt', max_length=512).to(device)
                scores = rerank_model(**inputs, return_dict=True).logits.view(-1,).float()
            
            scores_list = scores.cpu().tolist()
            results = [{"index": i, "score": score} for i, score in enumerate(scores_list)]
            # TEI default is to sort descending by score
            results.sort(key=lambda x: x["score"], reverse=True)
            
            self._json_response(results)
            log.info(f"Reranked {len(texts)} texts")
        except Exception as e:
            log.error(f"Rerank error: {e}")
            self._json_response({"error": str(e)}, 500)

    def _handle_embeddings(self):
        try:
            content_length = int(self.headers.get("Content-Length", 0))
            body = json.loads(self.rfile.read(content_length))

            input_text = body.get("input", "")
            if isinstance(input_text, str):
                input_text = [input_text]

            start = time.time()
            embeddings = model.encode(input_text)
            elapsed = time.time() - start

            data = []
            for i, emb in enumerate(embeddings):
                data.append({
                    "object": "embedding",
                    "index": i,
                    "embedding": emb.tolist()
                })

            response = {
                "object": "list",
                "data": data,
                "model": MODEL_NAME,
                "usage": {
                    "prompt_tokens": sum(len(t.split()) for t in input_text),
                    "total_tokens": sum(len(t.split()) for t in input_text)
                }
            }

            log.info(f"Embedded {len(input_text)} texts in {elapsed:.3f}s")
            self._json_response(response)

        except Exception as e:
            log.error(f"Embedding error: {e}")
            self._json_response({"error": str(e)}, 500)

    def _json_response(self, data, status=200):
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(data).encode())

    def log_message(self, format, *args):
        pass  # Suppress default access logs


if __name__ == "__main__":
    server = HTTPServer(("127.0.0.1", PORT), EmbeddingHandler)
    log.info(f"🧮 Embedding server running on http://127.0.0.1:{PORT}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        log.info("Shutting down")
        server.shutdown()
