import argparse
import json
from pathlib import Path

import cv2
from PIL import Image
from tqdm import tqdm
import numpy as np
from ultralytics import YOLO
from uuid import uuid4
from wonderwords import RandomWord

CLASS_CONVERSION = [
    "Ball",
    "Robot",
    "GoalPost",
    "PenaltySpot",
]


def generate_random_chunk_name():
    rng = RandomWord()

    adjective = rng.word(include_categories=[
                         "adjective"], regex="[a-zA-Z]+").lower()
    noun = rng.word(include_categories=["noun"], regex="[a-zA-Z]+").lower()

    return f"{adjective}-{noun}"


def main(args):
    image_paths = Path(args.image_folder).glob("*.png")
    if len(image_paths) == 0:
        raise RuntimeError("No images found in the image folder")

    yolo_model = None
    if args.yolo:
        yolo_model = YOLO(args.yolo)

    number_of_chunks = len(image_paths) // int(args.chunksize)

    output_path = Path("output")

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

            if yolo_model:
                detection = yolo_model(image, verbose=False)

                chunk_annotations[image_name] = [
                    {
                        "class": CLASS_CONVERSION[int(box.cls)],
                        "points": box.xyxy.reshape(2, 2).tolist(),
                    }
                    for box in detection[0].boxes
                ]

            image_name = str(uuid4()) + ".png"
            cv2.imwrite(str(images_path.joinpath(image_name)), image)

        with open(images_path.parent.joinpath("data.json"), 'w') as f:
            json.dump(chunk_annotations, f)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--image_folder", help="the images folder")
    parser.add_argument(
        "--yolo", help="the yolo checkpoint used for inference", default=None)
    parser.add_argument(
        "--chunksize", help="the chunksize of one labelling task", default=200)
    parser.add_argument("--convert-colors",
                        help="whether to convert ycbcr to rgb", default=True)

    main(parser.parse_args())
