# /// script
# requires-python = ">=3.13"
# dependencies = ["tqdm","opencv-python","pillow","numpy","ultralytics","wonderwords","onnxruntime"]
# ///

import argparse
import json
from pathlib import Path
from uuid import uuid4

import cv2
import numpy as np
from PIL import Image
from tqdm import tqdm
from ultralytics import YOLO
from wonderwords import RandomWord

CLASS_CONVERSION = [
    "Ball",
    "GoalPost",
    "LSpot",
    "PenaltySpot",
    "Robot",
    "TSpot",
    "XSpot",
    "Person",
]


def generate_random_chunk_name():
    rng = RandomWord()

    adjective = rng.word(
        include_categories=["adjective"], regex="[a-zA-Z]+"
    ).lower()
    noun = rng.word(include_categories=["noun"], regex="[a-zA-Z]+").lower()

    return f"{adjective}-{noun}"


def main(args):
    image_paths = list(Path(args.image_folder).glob("*.png"))
    if len(image_paths) == 0:
        raise RuntimeError("No images found in the image folder")

    yolo_model = None
    if args.yolo:
        yolo_model = YOLO(args.yolo)

    number_of_chunks = np.ceil(len(image_paths) / int(args.chunksize))

    output_path = Path("current")

    for chunk in tqdm(np.array_split(image_paths, number_of_chunks)):
        # create labelling folder
        chunk_annotations = {}
        chunk_name = generate_random_chunk_name()

        images_path = output_path.joinpath(chunk_name, "images")
        images_path.mkdir(parents=True, exist_ok=False)

        for image_path in chunk:
            image = cv2.imread(str(image_path))
            if args.convert_colors:
                image = cv2.cvtColor(image, cv2.COLOR_BGR2RGB)
                image = cv2.cvtColor(image, cv2.COLOR_YCrCb2RGB)

            image_name = str(uuid4()) + ".png"
            if yolo_model:
                detection = yolo_model(
                    image, verbose=False, conf=0.1, end2end=False, iou=0.3
                )

                chunk_annotations[image_name] = [
                    {
                        "class": CLASS_CONVERSION[int(box.cls)],
                        "points": box.xyxyn.reshape(2, 2).tolist(),
                    }
                    for box in detection[0].boxes
                ]

            cv2.imwrite(str(images_path.joinpath(image_name)), image)

        with open(images_path.parent.joinpath("data.json"), "w") as f:
            json.dump(chunk_annotations, f)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--image_folder", help="the images folder")
    parser.add_argument(
        "--yolo", help="the yolo checkpoint used for inference", default=None
    )
    parser.add_argument(
        "--chunksize", help="the chunksize of one labelling task", default=200
    )
    parser.add_argument(
        "--convert-colors",
        help="whether to convert ycbcr to rgb",
        default=False,
    )

    main(parser.parse_args())
