import argparse
import json
from glob import glob
import os
from os import path

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

    adjective = rng.word(include_categories=["adjective"], regex="[a-zA-Z]+").lower()
    noun = rng.word(include_categories=["noun"], regex="[a-zA-Z]+").lower()

    return f"{adjective}-{noun}"

def main(args):
    image_paths = glob(path.join(args.image_folder, "*.png"))
    yolo_model = None
    if args.yolo:
        yolo_model = YOLO(args.yolo)

    number_of_chunks = len(image_paths) // int(args.chunksize)

    for chunk in tqdm(np.array_split(image_paths, number_of_chunks)):
        # create labelling folder
        chunk_annotations = {}
        chunk_name = generate_random_chunk_name()

        chunk_dir = path.join("output", chunk_name)
        os.mkdir(chunk_dir)
        chunk_images_path = path.join(chunk_dir, "images")
        os.mkdir(chunk_images_path)

        for image_path in chunk:
            img = cv2.imread(image_path)
            if args.convert_colors:
                img = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
                img = cv2.cvtColor(img, cv2.COLOR_YCrCb2RGB)
            if yolo_model:
                detection = yolo_model(img, verbose=False)

            image_name = str(uuid4()) + ".png"

            chunk_annotations[image_name] = [
                {
                    "class": CLASS_CONVERSION[int(box.cls)],
                    "points": box.xyxy.reshape(2,2).tolist(),
                }
                for box in detection[0].boxes
            ]

            cv2.imwrite(path.join(chunk_images_path, image_name), img)

        with open(path.join(chunk_dir, "data.json"), 'w') as f:
            json.dump(chunk_annotations, f)

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--image_folder", help="the images folder")
    parser.add_argument("--yolo", help="the yolo checkpoint used for inference", default=None)
    parser.add_argument("--chunksize", help="the chunksize of one labelling task", default=200)
    parser.add_argument("--convert-colors", help="whether to convert ycbcr to rgb", default=True)

    main(parser.parse_args())
