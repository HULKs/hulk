from ultralytics import YOLO
from pathlib import Path
from huggingface_hub import hf_hub_download

import click

@click.command()
@click.argument(
    "height",
    type=int,
)

@click.argument(
    "width",
    type=int,
)

@click.option(
    "--model-path",
    type=click.Path(exists=True),
)

@click.option(
    "--download-model",
    is_flag=True,
    default=False
)

def main(height: int, width: int, model_path: str = "", download_model: bool = False) -> None:
    if download_model:
        model_path = hf_hub_download(repo_id="Ultralytics/YOLO11", filename="yolo11n-pose.pt", )
    else:
        model_path = Path(model_path)
    model = YOLO(model_path)

    exported_model_path = model.export(format="openvino",imgsz=(height,width))
    print(f"Exported model to {exported_model_path}")

if __name__ == "__main__":
    main()