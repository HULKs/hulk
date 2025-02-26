from ultralytics import YOLO
from pathlib import Path
from huggingface_hub import hf_hub_download

import click


@click.command(
    help="The first argument is the height of the input image, the second is the width. The [REPO_ID] is the Hugging Face repository ID, and the [FILENAME] is the name of the model file in the repository."
)
@click.argument(
    "input_image_height",
    type=click.INT,
)
@click.argument(
    "input_image_width",
    type=click.INT,
)
@click.argument("repo_id", type=str, default="Ultralytics/YOLO11")
@click.argument("filename", type=str, default="yolo11n-pose.pt")
@click.option(
    "--model-path",
    type=click.Path(exists=True),
    default=".",
    help="Path to the model file. Can be any format supported by Openvino.",
)
@click.option("--download-model", is_flag=True, default=False)
def main(
    input_image_height: int,
    input_image_width: int,
    repo_id: str,
    filename: str,
    model_path: str = "",
    download_model: bool = False,
) -> None:
    if download_model:
        model_path = hf_hub_download(repo_id=repo_id, filename=filename)
    else:
        model_path = Path(model_path)
    model = YOLO(model_path)

    exported_model_path = model.export(
        format="openvino", imgsz=(input_image_height, input_image_width)
    )
    print(f"Exported model to {exported_model_path}")


if __name__ == "__main__":
    main()
