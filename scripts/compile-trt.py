# /// script
# requires-python = "==3.10.*"
# dependencies = [
#     "onnxruntime-gpu",
#     "click",
#     "numpy<2",
# ]
#
# [tool.uv.sources]
# onnxruntime-gpu = { index = "onnxruntime-jetson" }
#
# [[tool.uv.index]]
# name = "onnxruntime-jetson"
# url = "https://pypi.jetson-ai-lab.io/jp6/cu126"
# explicit = true
# ///

from pathlib import Path
from smtplib import SMTP_PORT

import click
import onnxruntime


@click.command()
@click.argument("onnx_file_path", type=click.Path(exists=True, path_type=Path))
@click.option(
    "--cache_directory_path",
    type=click.Path(path_type=Path),
    default="/home/booster/.cache/hulk/tensor-rt",
)
def main(onnx_file_path: Path, cache_directory_path: Path) -> None:
    cache_directory_path.mkdir(parents=True, exist_ok=True)
    large_height = 1088
    large_width = 1280
    small_height = large_height // 2
    small_width = large_width // 2

    provider_options = {
        "trt_engine_cache_enable": True,
        # Required because ONNX Runtime strictly expects string values in provider options dictionaries
        "trt_engine_cache_path": str(cache_directory_path),
        "trt_profile_min_shapes": f"raw_bytes_input:{small_height // 2}x{small_width // 2}x6",
        "trt_profile_max_shapes": f"raw_bytes_input:{large_height // 2}x{large_width // 2}x6",
        "trt_profile_opt_shapes": f"raw_bytes_input:{small_height // 2}x{small_width // 2}x6",
    }

    execution_providers = [
        ("TensorrtExecutionProvider", provider_options),
        # Required as a fallback for graph nodes that TensorRT cannot natively process
        "CUDAExecutionProvider",
    ]

    onnxruntime.InferenceSession(onnx_file_path, providers=execution_providers)

    print("Session created successfully.")


if __name__ == "__main__":
    main()
