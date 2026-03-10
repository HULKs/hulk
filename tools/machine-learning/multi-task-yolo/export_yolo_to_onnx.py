from pathlib import Path

from ultralytics import YOLO


def convert_model(model_path: Path, height: int, width: int) -> None:
    model = YOLO(model_path)
    model.export(format="onnx", imgsz=(height, width), simplify=True, nms=True)


if __name__ == "__main__":
    convert_model(Path("./yolo26m-finetune.pt"), 448, 544)
