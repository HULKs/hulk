from ultralytics import YOLO
from pathlib import Path

import click
import openvino as ov
import torch

@click.command()
@click.argument(
    "model_path",
    type=click.Path(exists=True),
)

@click.argument(
    "height",
    type=int,
)

@click.argument(
    "width",
    type=int,
)

def main(model_path: str, height: int, width: int) -> None:
    model_path = Path(model_path)
    # Load a YOLOv8n PyTorch model
    model = YOLO(model_path)

    # Export the model
    model.export(format="openvino",imgsz=(height,width))  # creates 'yolov8n_openvino_model/'

if __name__ == "__main__":
    main()