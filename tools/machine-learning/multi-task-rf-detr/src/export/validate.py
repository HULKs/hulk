"""Phase 4 - validate an exported RF-DETR ONNX and emit the hand-off contract.

    uv run -m export.validate --config assets/detection.yaml

Checks structural validity, the I/O contract, finite outputs on a real image, and
(if a working CUDA EP is present) CPU-vs-CUDA fp32 parity. onnxruntime-gpu lists
CUDAExecutionProvider even when it can't load; we register the env CUDA dirs on the DLL
path and verify the EP actually bound (no false PASS). Deployment is FP16 TensorRT, so this
certifies the graph is valid/runnable, not deployment accuracy. Writes docs/parity_report.md.
"""

import argparse
import os
import sys
from pathlib import Path

import numpy as np


def _register_cuda_dll_dirs() -> list:
    cands = [os.path.join(sys.prefix, "bin"), os.path.join(sys.prefix, "Library", "bin")]
    try:
        import torch

        cands.append(os.path.join(os.path.dirname(torch.__file__), "lib"))
    except Exception:  # noqa: BLE001 - torch optional here
        pass
    added = []
    for d in cands:
        if os.path.isdir(d):
            try:
                os.add_dll_directory(d)
                added.append(d)
            except OSError:
                pass
    return added


_DLL_DIRS = _register_cuda_dll_dirs()

from training.config import load_config  # noqa: E402 - after DLL dirs registered

_MEAN = np.array([0.485, 0.456, 0.406], dtype=np.float32)
_STD = np.array([0.229, 0.224, 0.225], dtype=np.float32)


def make_input(cfg, spec):
    res = cfg.training.resolution
    shape = [d if isinstance(d, int) and d > 0 else res for d in spec.shape]
    valid_dir = Path(cfg.data.dataset_dir) / "valid"
    imgs = (
        (list(valid_dir.glob("*.png")) + list(valid_dir.glob("*.jpg")))
        if valid_dir.exists()
        else []
    )
    if imgs and len(shape) == 4 and shape[1] == 3:
        from PIL import Image

        im = Image.open(imgs[0]).convert("RGB").resize((shape[3], shape[2]))
        arr = (np.asarray(im, dtype=np.float32) / 255.0 - _MEAN) / _STD
        return arr.transpose(2, 0, 1)[None].astype(np.float32), imgs[0].name
    rng = np.random.default_rng(0)
    return rng.standard_normal(shape).astype(np.float32), "random-noise"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--config", default="assets/detection.yaml")
    ap.add_argument("--onnx", default=None)
    args = ap.parse_args()

    cfg = load_config(args.config)
    onnx_path = args.onnx or cfg.export.output_path
    if not Path(onnx_path).exists():
        print(f"ERROR: onnx not found: {onnx_path}")
        sys.exit(1)

    import onnx
    import onnxruntime as ort

    onnx.checker.check_model(onnx_path)
    available = ort.get_available_providers()

    sess_cpu = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])
    in_specs, out_specs = sess_cpu.get_inputs(), sess_cpu.get_outputs()
    x, input_src = make_input(cfg, in_specs[0])
    feeds = {in_specs[0].name: x}
    out_cpu = sess_cpu.run(None, feeds)
    cpu_finite = all(np.all(np.isfinite(o)) for o in out_cpu)

    cuda_active, gpu_finite, parity_ok = False, None, None
    parity_line = "CUDAExecutionProvider not available - GPU run skipped"
    if "CUDAExecutionProvider" in available:
        sess_gpu = ort.InferenceSession(
            onnx_path,
            providers=[("CUDAExecutionProvider", {"use_tf32": "0"}), "CPUExecutionProvider"],
        )
        active = sess_gpu.get_providers()
        cuda_active = bool(active) and active[0] == "CUDAExecutionProvider"
        if not cuda_active:
            parity_line = (
                f"CUDA EP listed but did NOT bind (active={active}) - GPU parity NOT validated"
            )
        else:
            out_gpu = sess_gpu.run(None, feeds)
            gpu_finite = all(np.all(np.isfinite(o)) for o in out_gpu)
            max_diff = max(float(np.max(np.abs(a - b))) for a, b in zip(out_cpu, out_gpu))
            parity_ok = max_diff < cfg.export.parity_tolerance
            parity_line = (
                f"CPU vs CUDA(fp32) max abs diff = {max_diff:.3e} "
                f"(tol {cfg.export.parity_tolerance:.0e}) -> {'PASS' if parity_ok else 'WARN'}"
            )

    shapes_ok = all(len(o.shape) == len(s.shape) for o, s in zip(out_cpu, out_specs))

    lines = [
        "# ONNX Hand-off Contract",
        "",
        f"- File: `{Path(onnx_path).name}`  ({Path(onnx_path).stat().st_size / 1e6:.1f} MB)",
        f"- Variant: {cfg.model.variant} @ {cfg.training.resolution}^2, static batch={cfg.export.static_batch}",
        f"- onnxruntime {ort.__version__}; providers: {available}; CUDA EP bound: {cuda_active}",
        f"- Validation input: {input_src}",
        "",
        "## Classes  (ONNX `labels` has num_classes+1 columns; last = no-object)",
        *[f"  {i}: {n}" for i, n in enumerate(cfg.model.class_names)],
        "",
        "## Inputs",
        *[f"  {s.name}: {s.shape} {s.type}" for s in in_specs],
        "",
        "## Outputs",
        *[f"  {s.name}: {s.shape} {s.type}" for s in out_specs],
        "",
        "  dets = (cx,cy,w,h) normalized per query; labels = class logits (apply sigmoid).",
        "",
        "## Checks",
        f"  CPU outputs finite: {cpu_finite}",
        f"  CUDA outputs finite: {gpu_finite}",
        f"  Output shapes match graph: {shapes_ok}",
        f"  {parity_line}",
        "",
        "## Robot-side preprocessing",
        "  - NCHW float32, ImageNet mean/std, letterbox to square resolution.",
        "  - Build TensorRT engine with fixed 1x3xRxR input (static batch).",
        "  - Deployment uses FP16 TensorRT; accuracy parity vs PyTorch is validated on-robot.",
    ]
    Path("docs/parity_report.md").write_text("\n".join(lines), encoding="utf-8")

    ok = cpu_finite and shapes_ok and (gpu_finite is not False)
    print(f"Structural: PASS | input: {input_src}")
    print(f"Inputs:  {[(s.name, s.shape) for s in in_specs]}")
    print(f"Outputs: {[(s.name, s.shape) for s in out_specs]}")
    print(
        f"CPU finite={cpu_finite} CUDA bound={cuda_active} finite={gpu_finite} shapes_ok={shapes_ok}"
    )
    print(parity_line)
    print(f"Overall: {'PASS' if ok else 'FAIL'} | report: docs/parity_report.md")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
