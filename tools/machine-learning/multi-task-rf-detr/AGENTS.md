# AGENTS.md

## Scope

- This file is for `tools/machine-learning/multi-task-rf-detr` only.
- Commands here run from this directory.

## Environment and Tooling

- Python is pinned to `3.12` (`.python-version`); `pyproject.toml` requires `>=3.12,<3.13`.
  (The sibling `multi-task-yolo` uses 3.13; we pin 3.12 because rfdetr / PyTorch are validated there.)
- Dependencies are managed with `uv` (`uv.lock` present). Prefer `uv run ...` over bare `python`/`pip`.
- `torch`/`torchvision` come from the CUDA 12.4 index declared in `pyproject.toml` (`[[tool.uv.index]]`).
- Lint config in `ruff.toml` (line length `100`, select `E/F/UP/C4`); can be tightened to the sibling's stricter set once full type annotations land.
- No `pytest`/CI config exists in this project directory.

## Reliable Commands

- Lint: `uv run ruff check src`
- Format: `uv run ruff format src`
- Convert dataset: `uv run -m data.convert`
- Verify dataset: `uv run -m data.verify --config assets/detection.yaml`
- Train (dry-run, no GPU): `uv run -m training.train --config assets/detection.yaml --dry-run`
- Export ONNX: `uv run -m export.to_onnx --config assets/detection.yaml`
- Validate ONNX: `uv run -m export.validate --config assets/detection.yaml`
- Env smoke test: `uv run python src/smoke.py`

## Code Layout (what actually runs)

- `src/training/config.py`: YAML dataclass config + resolution-÷56 validation.
- `src/training/train.py`: maps config → rfdetr `train()` kwargs; `--dry-run` needs no GPU/rfdetr.
- `src/data/convert.py`: NAO YOLO → COCO (lossless; hardlinks; keeps background frames as negatives).
- `src/data/verify.py`: COCO structure / image↔label pairing / class-distribution checks.
- `src/export/to_onnx.py`: static-batch ONNX export (TensorRT-safe).
- `src/export/validate.py`: structural + I/O contract + CPU/CUDA fp32 parity; writes `docs/parity_report.md`.

## Repo-Specific Gotchas

- Use `uv run -m <pkg>.<module>` for entrypoints (the package is installed by `uv sync`).
- `training.train --dry-run` validates config + kwargs without importing rfdetr or touching the GPU.
- Datasets, weights (`*.pt`/`*.pth`), ONNX (`*.onnx`), and `runs/`/`outputs/` are intentionally gitignored.
- `onnxruntime` is the CPU build (mirrors the sibling). For local GPU ONNX parity, also install
  `onnxruntime-gpu` and ensure cuDNN/cudart are on the DLL path — `export/validate.py` registers the
  env CUDA dirs automatically and reports honestly if the CUDA EP fails to bind.
- Resolution must be divisible by 56 (the config loader enforces this); default is `448`.
