# AGENTS.md

## Scope
- This file is for `tools/machine-learning/multi-task-yolo` only.
- The git root is `/home/alexschmander/hulk`, but commands here should usually run from this directory.

## Environment and Tooling
- Python is pinned to `3.13` (`.python-version`) and `pyproject.toml` requires `>=3.13`.
- Dependencies are managed with `uv` (`uv.lock` is present). Prefer `uv run ...` over bare `python`/`pip`.
- Lint config is in `ruff.toml` (line length `80`, broad strict rule set, tests get limited `S101`/`S603` ignores).
- No `pytest`/`mypy`/`pre-commit`/CI config exists in this project directory.

## Reliable Commands
- Lint: `uv run ruff check src`
- Format: `uv run ruff format src`
- Validation CLI help: `uv run -m validation.validator --help`
- Compare validation runs help: `uv run -m validation.compare_results --help`
- ONNX export help: `uv run -m utils.export_yolo_to_onnx --help`
- Single-task training CLI help: `uv run python src/model/train.py --help`

## Code Layout (what actually runs)
- `src/model/hydra.py`: core multi-head model assembly (shared backbone + task heads).
- `src/validation/validator.py`: main validation pipeline; can validate original models and Hydra heads; writes `metrics.json`, `metadata.json`, and `config.json` under `runs/val/...`.
- `src/validation/compare_results.py`: compares two saved validation run directories and emits a comparison report JSON.
- `src/validation/predictor.py`: inference + visualization helper for detection/pose Hydra outputs.
- `src/utils/export_yolo_to_onnx.py`: Click CLI that wraps NV12 preprocessing and exports ONNX.

## Repo-Specific Gotchas
- Use `uv run -m ...` for Python module entrypoints in this repo.
- `src/model/train.py` is a Click CLI for finetuning a single task model with configurable paths/flags.
- `src/model/train.py` `--device` expects a comma-separated list (for example `--device 0,1`) and is parsed to `list[int]`.
- `src/model/train.py` defaults to full training settings; dev settings run only when `--dev-mode` is provided.
- `src/model/train.py` tuning runs only when `--do-tuning` is provided.
- `src/validation/predictor.py` `main()` uses a hardcoded example image path under `assets/datasets/...`; it is a local smoke script, not a generic entrypoint.
- Large/generated artifacts are intentionally ignored (`runs/`, `assets/datasets/`, `assets/output/`, most `*.pt` weights).
