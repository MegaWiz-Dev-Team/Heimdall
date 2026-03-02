---
description: TDD workflow — how to develop features using Test-Driven Development
---
// turbo-all

# TDD Development Workflow

Follow the **Red-Green-Refactor** cycle for every code change.

## Steps

### 1. Update Requirements (ISO 29110)
Before writing any code, check `docs/iso29110/SI/requirements_spec.md`:
- If this is a new feature → add a new `REQ-xxx` entry
- If this modifies existing → update the requirement

### 2. Write Test Case (ISO 29110)
Add test case to `docs/iso29110/SI/test_cases.md`:
- Assign a Test ID (e.g., `UT-XXX-001`)
- Link to requirement ID
- Define input and expected output

### 3. RED — Write Failing Test
```bash
# For Rust:
cargo test --lib <test_name>

# For Python:
pytest tests/<test_file>.py -k <test_name>

# For Shell scripts:
bats tests/<test_file>.bats
```
The test MUST fail. If it passes, the test is not testing new behavior.

### 4. GREEN — Write Minimum Code
Write just enough code to make the test pass:
```bash
# For Rust:
cargo test --lib <test_name>

# For Python:
pytest tests/<test_file>.py -k <test_name>
```
The test MUST pass now.

### 5. REFACTOR — Clean Up
- Improve code quality while keeping tests green
- Run full test suite to ensure nothing broke:
```bash
# For Rust:
cargo test

# For Python:
pytest tests/

# For Shell:
bats tests/
```

### 6. Update Test Report (ISO 29110)
Record results in `docs/iso29110/SI/test_report.md`:
- Date, Test ID, Result (Pass/Fail), Notes

### 7. Update Progress (ISO 29110)
Update `docs/iso29110/PM/progress_status.md`:
- Log what was done, status, date

### 8. Lint & Format
```bash
# Rust
cargo fmt --check
cargo clippy -- -D warnings

# Python
ruff check .
ruff format --check .

# Shell
shellcheck scripts/*.sh
```

## Checklist Before Committing
- [ ] Requirements updated (if new feature)
- [ ] Test case documented in `test_cases.md`
- [ ] Test written FIRST (was red)
- [ ] Code written to pass test (now green)
- [ ] Refactored with tests still green
- [ ] Test report updated
- [ ] Progress status updated
- [ ] Lint passes (zero warnings)
- [ ] Commit message includes `[REQ-xxx]` or `[BUG-xxx]`
