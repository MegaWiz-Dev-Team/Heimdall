#!/usr/bin/env python3
import os
import sys
import argparse
from huggingface_hub import HfApi, login

def main():
    parser = argparse.ArgumentParser(description="Upload an MLX model to Hugging Face")
    parser.add_argument(
        "--model-dir", 
        type=str, 
        required=True,
        help="Path to local model directory (e.g. ./models/Qwen3.5-27B)"
    )
    parser.add_argument(
        "--repo-id", 
        type=str, 
        required=True,
        help="Hugging Face Target Repository ID (e.g. username/Model-Name)"
    )
    parser.add_argument(
        "--token", 
        type=str,
        default=None,
        help="Hugging Face Access Token (optional, uses cached token or prompts if not provided)"
    )

    args = parser.parse_args()
    model_dir = os.path.abspath(os.path.expanduser(args.model_dir))
    repo_id = args.repo_id

    print("============================================")
    print(f"🚀 Hugging Face Model Uploader (Python API)")
    print(f"Target Repository: https://huggingface.co/{repo_id}")
    print(f"Model Directory: {model_dir}")
    print("============================================\n")

    if not os.path.exists(model_dir):
        print(f"❌ Error: Model directory not found at {model_dir}")
        sys.exit(1)

    print("Step 1: Authentication")
    token = args.token
    if not token:
        # Check if HF_TOKEN env var is set
        token = os.environ.get("HF_TOKEN")
        if not token:
            try:
                token = input("Please paste your Hugging Face Token (with Write access): ").strip()
            except EOFError:
                # Handle non-interactive environments
                pass
            
    if not token:
        print("⚠️ No token provided. Assuming you are already authenticated globally via huggingface-cli.")
        try:
            login()
        except Exception as e:
            print(f"❌ Login verification failed: {e}")
            sys.exit(1)
    else:
        try:
            login(token=token)
        except Exception as e:
            print(f"❌ Login failed: {e}")
            sys.exit(1)

    api = HfApi()

    try:
        print("\nStep 2: Checking/Creating Repository...")
        api.create_repo(repo_id=repo_id, repo_type="model", exist_ok=True)
        print(f"✅ Repository {repo_id} is ready.")
    except Exception as e:
        print(f"❌ Failed to create/access repository: {e}")
        sys.exit(1)

    print(f"\nStep 3: Uploading Model Files...")
    print("This may take a while depending on your internet upload speed.\n")

    try:
        api.upload_folder(
            folder_path=model_dir,
            repo_id=repo_id,
            repo_type="model",
        )
        print("\n✅ Upload Completed Successfully!")
        print(f"View your model at: https://huggingface.co/{repo_id}")
    except Exception as e:
        print(f"\n❌ Upload failed: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
