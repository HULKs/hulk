# /// script
# requires-python = ">=3.13"
# dependencies = ["click"]
# ///

import json
import os
import shutil
from hashlib import sha256

import click

# Class mapping
CLASS_MAP = {
    "Ball": 0,
    "GoalPost": 1,
    "LSpot": 2,
    "PenaltySpot": 3,
    "Robot": 4,
    "TSpot": 5,
    "XSpot": 6,
    "Person": 7,
}
IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".bmp", ".webp", ".tif", ".tiff"}


def convert_annotations(json_path: str, filename: str) -> list[str]:
    with open(json_path) as f:
        annotations = json.load(f)

    yolo_lines = []

    for ann in annotations:
        class_name = ann.get("class")
        points = ann.get("points")

        if class_name not in CLASS_MAP:
            print(f"Skipping unknown class '{class_name}' in {filename}")
            continue

        if not points or len(points) != 2:
            print(f"Invalid points in {filename}")
            continue

        (x1, y1), (x2, y2) = points

        # Ensure correct ordering
        x_min = min(x1, x2)
        x_max = max(x1, x2)
        y_min = min(y1, y2)
        y_max = max(y1, y2)

        # Convert to YOLO format (normalized)
        x_center = (x_min + x_max) / 2
        y_center = (y_min + y_max) / 2
        width = x_max - x_min
        height = y_max - y_min

        class_id = CLASS_MAP[class_name]

        yolo_lines.append(
            f"{class_id} {x_center:.6f} {y_center:.6f} {width:.6f} {height:.6f}"
        )

    return yolo_lines


def find_images_by_stem(input_dir: str) -> dict[str, str]:
    images_by_stem: dict[str, str] = {}

    for filename in os.listdir(input_dir):
        path = os.path.join(input_dir, filename)
        if not os.path.isfile(path):
            continue

        stem, ext = os.path.splitext(filename)
        if ext.lower() not in IMAGE_EXTENSIONS:
            continue

        if stem in images_by_stem:
            current_name = os.path.basename(images_by_stem[stem])
            if filename < current_name:
                images_by_stem[stem] = path
            print(
                "Found multiple images for "
                f"'{stem}', using '{os.path.basename(images_by_stem[stem])}'."
            )
            continue

        images_by_stem[stem] = path

    return images_by_stem


@click.command()
@click.argument(
    "input_dir",
    type=click.Path(
        exists=True,
        file_okay=False,
        path_type=str,
    ),
)
@click.argument(
    "output_dir",
    type=click.Path(
        file_okay=False,
        path_type=str,
    ),
)
@click.option(
    "--train-split",
    type=click.FloatRange(0.0, 1.0),
    default=0.8,
    show_default=True,
    help="Fraction of matched samples to put into train. Val gets the rest.",
)
@click.option(
    "--seed",
    type=int,
    default=42,
    show_default=True,
    help="Random seed used for shuffling before the train/val split.",
)
def main(
    input_dir: str,
    output_dir: str,
    train_split: float,
    seed: int,
) -> None:
    """Convert Annotato labels to YOLO and create a COCO-style train/val layout.

    INPUT_DIR must contain JSON annotations and image files with matching stems.
    OUTPUT_DIR will be created with images/{train,val} and labels/{train,val}.
    """
    labels_root = os.path.join(output_dir, "labels")
    images_root = os.path.join(output_dir, "images")

    for split_name in ("train", "val"):
        os.makedirs(os.path.join(labels_root, split_name), exist_ok=True)
        os.makedirs(os.path.join(images_root, split_name), exist_ok=True)

    images_by_stem = find_images_by_stem(input_dir)
    json_filenames = sorted(
        filename
        for filename in os.listdir(input_dir)
        if filename.endswith(".json")
    )

    samples: list[tuple[str, str, str, str]] = []
    for filename in json_filenames:
        base = os.path.splitext(filename)[0]
        image_path = images_by_stem.get(base)

        if image_path is None:
            print(f"No matching image found for {filename}. Skipping...")
            continue

        samples.append(
            (filename, base, os.path.join(input_dir, filename), image_path)
        )

    if not samples:
        print("No matching JSON/image samples were found.")
        return

    samples.sort(
        key=lambda item: sha256(f"{seed}:{item[1]}".encode()).hexdigest()
    )

    train_count = int(len(samples) * train_split)
    split_samples = {
        "train": samples[:train_count],
        "val": samples[train_count:],
    }

    processed = 0
    for split_name, items in split_samples.items():
        split_labels_dir = os.path.join(labels_root, split_name)
        split_images_dir = os.path.join(images_root, split_name)

        for filename, base, json_path, image_path in items:
            out_txt_path = os.path.join(split_labels_dir, base + ".txt")
            out_img_path = os.path.join(
                split_images_dir, os.path.basename(image_path)
            )

            try:
                yolo_lines = convert_annotations(json_path, filename)

                # Write output TXT (even if empty, to stay consistent)
                with open(out_txt_path, "w") as f:
                    f.write("\n".join(yolo_lines))

                shutil.copy2(image_path, out_img_path)
                processed += 1
            except Exception as e:
                print(f"Error processing {filename}: {e}. Skipping...")

    print(
        "Done generating YOLO dataset with split: "
        f"{processed} samples "
        "("
        f"{len(split_samples['train'])} train / "
        f"{len(split_samples['val'])} val"
        ")."
    )


if __name__ == "__main__":
    main()
