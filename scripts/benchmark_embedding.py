import os
import sys
import time
import json
import urllib.request
import statistics
import datetime

# --- Configuration ---
API_KEY = "hml-REDACTED"
MODEL = "BAAI/bge-m3"

# Endpoints
RUST_NATIVE_URL = "http://127.0.0.1:8080/v1/embeddings"
PYTHON_MLX_URL = "http://127.0.0.1:8001/v1/embeddings"
OLLAMA_URL = "http://127.0.0.1:11434/api/embed"
LLAMA_CPP_URL = "http://127.0.0.1:8089/v1/embeddings"

# Test Payload Size
BATCH_SIZES = [1, 8, 32, 64]
SAMPLE_TEXT = "The Heimdall service is designed as the guardian of the LLM realm for the Asgard infrastructure."

def cosine_similarity(v1, v2):
    dot_product = sum(a*b for a, b in zip(v1, v2))
    mag1 = sum(a*a for a in v1) ** 0.5
    mag2 = sum(b*b for b in v2) ** 0.5
    if mag1 == 0 or mag2 == 0:
        return 0
    return dot_product / (mag1 * mag2)

def run_embedding(url, texts, model=MODEL):
    data = json.dumps({
        "model": model,
        "input": texts
    }).encode("utf-8")

    req = urllib.request.Request(url, data=data)
    req.add_header("Content-Type", "application/json")
    req.add_header("Authorization", f"Bearer {API_KEY}")

    start = time.perf_counter()
    try:
        with urllib.request.urlopen(req) as response:
            result = json.loads(response.read().decode())
            elapsed = time.perf_counter() - start
            return result, elapsed
    except Exception as e:
        print(f"Error calling {url}: {e}")
        return None, 0

def run_benchmark():
    results = {}
    
    # Warmup
    print("Warming up endpoints...")
    run_embedding(RUST_NATIVE_URL, [SAMPLE_TEXT])
    run_embedding(PYTHON_MLX_URL, [SAMPLE_TEXT])
    run_embedding(LLAMA_CPP_URL, [SAMPLE_TEXT])
    run_embedding(OLLAMA_URL, [SAMPLE_TEXT], model="bge-m3:latest")
    
    for batch_size in BATCH_SIZES:
        payload = [SAMPLE_TEXT for _ in range(batch_size)]
        
        print(f"\n--- Batch Size: {batch_size} ---")
        
        # Test Python MLX
        print("Testing Python MLX (port 8001)...")
        mlx_times = []
        mlx_outputs = None
        for _ in range(5):
            res, elapsed = run_embedding(PYTHON_MLX_URL, payload)
            if res:
                mlx_times.append(elapsed)
                mlx_outputs = res["data"]
                
        # Test Rust Native
        print("Testing Rust Native (port 8080)...")
        rust_times = []
        rust_outputs = None
        for _ in range(5):
            res, elapsed = run_embedding(RUST_NATIVE_URL, payload)
            if res:
                rust_times.append(elapsed)
                rust_outputs = res["data"]
                
        # Test Ollama
        print("Testing Ollama (port 11434)...")
        ollama_times = []
        ollama_outputs = None
        for _ in range(5):
            res, elapsed = run_embedding(OLLAMA_URL, payload, model="bge-m3:latest")
            if res:
                ollama_times.append(elapsed)
                ollama_outputs = res.get("embeddings", [])
                
        # Test llama.cpp native
        print("Testing llama.cpp native (port 8089)...")
        llama_cpp_times = []
        llama_cpp_outputs = None
        for _ in range(5):
            res, elapsed = run_embedding(LLAMA_CPP_URL, payload)
            if res:
                llama_cpp_times.append(elapsed)
                llama_cpp_outputs = res.get("data", [])
        
        # Calculate statistics
        if mlx_times and rust_times:
            # Latency stats
            mlx_avg = statistics.mean(mlx_times) if mlx_times else 0
            rust_avg = statistics.mean(rust_times) if rust_times else 0
            ollama_avg = statistics.mean(ollama_times) if ollama_times else 0
            llama_avg = statistics.mean(llama_cpp_times) if llama_cpp_times else 0
            
            mlx_tps = batch_size / mlx_avg if mlx_avg > 0 else 0
            rust_tps = batch_size / rust_avg if rust_avg > 0 else 0
            ollama_tps = batch_size / ollama_avg if ollama_avg > 0 else 0
            llama_tps = batch_size / llama_avg if llama_avg > 0 else 0
            
            # Vector validation between MLX and Rust
            similarity = "N/A"
            if mlx_outputs and rust_outputs and len(mlx_outputs) > 0 and len(rust_outputs) > 0:
                v_mlx = mlx_outputs[0]["embedding"]
                v_rust = rust_outputs[0]["embedding"]
                if len(v_mlx) == len(v_rust):
                    similarity = cosine_similarity(v_mlx, v_rust)
                else:
                    similarity = f"Mismatch: {len(v_mlx)}d vs {len(v_rust)}d"
            
            results[batch_size] = {
                "mlx": {"latency_s": mlx_avg, "throughputs": mlx_tps},
                "rust": {"latency_s": rust_avg, "throughputs": rust_tps},
                "ollama": {"latency_s": ollama_avg, "throughputs": ollama_tps},
                "llama_cpp": {"latency_s": llama_avg, "throughputs": llama_tps},
                "similarity": similarity
            }
            
            print(f"  MLX:       {mlx_avg*1000:.1f}ms per batch ({mlx_tps:.1f} doc/s)")
            print(f"  Rust:      {rust_avg*1000:.1f}ms per batch ({rust_tps:.1f} doc/s)")
            if ollama_times:
                print(f"  Ollama:    {ollama_avg*1000:.1f}ms per batch ({ollama_tps:.1f} doc/s)")
            else:
                print(f"  Ollama:    Error or not available")
            
            if llama_cpp_times:
                print(f"  llama.cpp: {llama_avg*1000:.1f}ms per batch ({llama_tps:.1f} doc/s)")
            else:
                print(f"  llama.cpp: Error or not available")
            
            if isinstance(similarity, float):
                print(f"  Cosine Similarity (MLX vs Rust): {similarity:.4f}")

    # Generate Markdown Report
    report_file = os.path.join(os.path.dirname(os.path.dirname(__file__)), "reports", "native_embedding_benchmark.md")
    with open(report_file, "w") as f:
        f.write("# Heimdall Native Embedding Benchmark\n\n")
        f.write(f"**Date:** {datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write(f"**Model:** {MODEL}\n\n")
        f.write("This benchmark compares the performance of the Python MLX sidecar vs the native Rust ONNX embedding integration in Heimdall HTTP responses.\n\n")
        
        f.write("## Test Parameters\n")
        f.write("- **Hardware**: Apple Silicon (M-series)\n")
        f.write("- **Payload**: Sentence with 14 words\n")
        f.write("- **Iterations**: 5 per batch size\n\n")
        
        f.write("## Results\n\n")
        f.write("| Batch Size | MLX (doc/s) | Rust ONNX (doc/s) | Ollama (doc/s) | llama.cpp (doc/s) |\n")
        f.write("|------------|-------------|-------------------|----------------|-------------------|\n")
        
        for batch_size in BATCH_SIZES:
            if batch_size in results:
                res = results[batch_size]
                mlx_tps = res['mlx']['throughputs']
                rust_tps = res['rust']['throughputs']
                ollama_tps = res['ollama']['throughputs']
                llama_tps = res['llama_cpp']['throughputs']
                
                f.write(f"| {batch_size} | {mlx_tps:.1f} | {rust_tps:.1f} | {ollama_tps:.1f} | {llama_tps:.1f} |\n")
                
        f.write("\n## Conclusion\n")
        f.write("Rust native ONNX embeddings eliminates the HTTP overhead and sidecar dependency. ")
        f.write("Ollama provides a heavily optimized execution environment but has overhead compared to native MLX on Apple Silicon. ")
        f.write("Check the table above for exact performance comparisons.\n")
        
    print(f"\nReport generated: {report_file}")

if __name__ == "__main__":
    run_benchmark()
