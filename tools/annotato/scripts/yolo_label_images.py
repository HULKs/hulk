import sys
from ultralytics import YOLO
from PIL import Image
import json
from os import path
from tqdm import tqdm

CLASS_CONVERSION = [
    "Ball",
    "Robot",
    "GoalPost",
    "PenaltySpot",
]

def main():
    yolo_model = YOLO("best-2021.pt")
    annotations = {}

    for image_path in tqdm(sys.argv[1:]):
        image = Image.open(image_path)
        detection = yolo_model(image)

        annotations[path.basename(image_path)] = [
            {
                "class": CLASS_CONVERSION[int(box.cls)],
                "points": box.xyxy.reshape(2,2).tolist(),
            }
            for box in detection[0].boxes
        ]


    with open('data.json', 'w') as f:
        json.dump(annotations, f)

if __name__ == "__main__":
    main()