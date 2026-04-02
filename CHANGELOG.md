# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-04-02
### Added
- Created generic `upload_to_hf.py` script featuring `argparse` for standard public MLX model deployment to Hugging Face Hub.
- Included comprehensive generation benchmarks for `Qwen3.5-27B-Opus-Reasoning-MLX-4bit` natively evaluated on Apple M4 Pro architecture arrays.

### Fixed
- Fixed critical authentication bug in `benchmark.sh` where metric API calls failed when `API_KEYS` were enforced at gateway levels.
- Fixed `benchmark.sh` rendering issue where unformatted Array indices crashed bash JSON string concatenation.

### Changed
- Standardized `convert_qwen_27b.sh` structure to deduce dynamic execution paths, detaching dependencies on user-specific `$HOME/Developer/` roots.
- Removed hardcoded external SSD (`/Volumes/T7 Shield/...`) overrides from scripts, favoring dynamically resolved environmental standards (`HF_HUB_CACHE`).
- Overhauled README documentation specifically readying Apple Silicon operations for open-source public access.

## [0.1.0] - 2026-03-01
### Added
- Initial project layout for Heimdall AI Framework.
- MLX inference integration for lightweight LLM serving on Apple Silicon.
- Baseline benchmark pipelines.
