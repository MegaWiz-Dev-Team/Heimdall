---
description: Project-wide development rules for LLM Server — ISO 29110 + TDD
---

# LLM Server — Development Rules

## 1. ISO 29110 Basic Profile Compliance

This project follows **ISO/IEC 29110 Basic Profile** for Very Small Entities (VSE).
All development MUST maintain the two core processes:

### Project Management (PM)
- Every task MUST be tracked in `docs/iso29110/PM/project_plan.md`
- Progress MUST be logged in `docs/iso29110/PM/progress_status.md` with dates
- Changes to scope MUST be documented in `docs/iso29110/PM/change_request.md`
- Meetings/decisions MUST be recorded in `docs/iso29110/PM/meeting_records.md`

### Software Implementation (SI)
- Requirements MUST be documented in `docs/iso29110/SI/requirements_spec.md` **before** coding
- Design MUST be documented in `docs/iso29110/SI/software_design.md` **before** coding
- All code MUST have corresponding tests documented in `docs/iso29110/SI/test_cases.md`
- Test results MUST be recorded in `docs/iso29110/SI/test_report.md`
- Each release MUST have a `docs/iso29110/SI/release_notes.md` entry

### Document Update Rules
- When modifying requirements → update `requirements_spec.md` FIRST
- When modifying design/architecture → update `software_design.md` FIRST
- When adding features → add test cases to `test_cases.md` FIRST (TDD)
- After running tests → update `test_report.md` with results
- After merging work → update `progress_status.md`

---

## 2. Test-Driven Development (TDD)

ALL development MUST follow the **Red-Green-Refactor** cycle:

### TDD Workflow
```
1. RED    — Write a failing test FIRST
2. GREEN  — Write minimum code to pass the test
3. REFACTOR — Clean up code while keeping tests green
```

### TDD Rules
- **NEVER write production code without a failing test first**
- Test files MUST mirror source structure: `src/foo.rs` → `tests/foo_test.rs`
- For shell scripts: create corresponding test in `tests/` using `bats` or `shunit2`
- For Rust code: use built-in `#[test]` and `#[cfg(test)]` modules
- For Python code: use `pytest` with files in `tests/`
- Test names MUST be descriptive: `test_gateway_routes_to_correct_backend`
- Each PR/change MUST include tests — no exceptions

### Test Categories
| Category | Location | Runner | When |
|:--|:--|:--|:--|
| Unit tests | `tests/unit/` | `cargo test` / `pytest` | Every code change |
| Integration tests | `tests/integration/` | Custom scripts | Before merge |
| API smoke tests | `tests/api/` | `./tests/api/test_api.sh` | After deployment |
| Benchmark tests | `tests/benchmark/` | `./scripts/benchmark.sh` | On demand |

### Coverage Requirements
- Minimum **80%** test coverage for Rust code
- All public API endpoints MUST have integration tests
- All error paths MUST have dedicated tests

---

## 3. Code Style & Structure

### Rust
- Use `rustfmt` for formatting
- Use `clippy` for linting — zero warnings policy
- Follow Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
- Error handling: use `thiserror` for library errors, `anyhow` for application errors

### Python
- Use `ruff` for formatting and linting
- Use type hints on all function signatures
- Follow PEP 8

### Shell Scripts
- Use `shellcheck` for linting
- Always use `set -euo pipefail` at the top
- Quote all variables

---

## 4. Documentation Standards

### Code Documentation
- All public functions MUST have doc comments
- Complex logic MUST have inline comments explaining WHY, not WHAT
- README.md MUST be kept in sync with actual functionality

### ISO 29110 Traceability
- Every requirement in `requirements_spec.md` MUST have a unique ID (e.g., `REQ-001`)
- Every test case MUST reference a requirement ID
- Every design component MUST trace back to requirements

---

## 5. Git Workflow

- Branch naming: `feature/REQ-xxx-description`, `fix/BUG-xxx-description`
- Commit messages: `[REQ-xxx] description` or `[BUG-xxx] description`
- No direct commits to `main` — use feature branches
- All changes MUST pass tests before merge
