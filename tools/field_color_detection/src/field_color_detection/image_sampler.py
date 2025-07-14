import argparse
import os
import random
from collections import defaultdict
from pathlib import Path

import cv2
import pandas as pd
import plotly.graph_objects as go

from .data import (
    convert_BGR_to_YCrCb,
    convert_YCrCb_to_BGR,
)

SAMPLE = False

"""
Usage: uv run image_sampler.py
            --dataset-roots /path/to/dataset1 /path/to/dataset2
            --metadata-file /path/to/dataset_description.ods

This script samples 6,000 images from a hierarchical dataset of images
using metadata from an .ods file to enforce a target distribution based on:

- Lighting conditions: Indoor, Outdoor, Mixed
- Shadow/Reflection types: None, Light Reflections, Shadows, Both

It traverses specified dataset root directories, maps each game folder to its
metadata entry, and collects images accordingly. The sampled image paths are
saved to 'sampled_images.txt'. The script accepts command-line arguments for
dataset paths and the metadata file.
"""

parser = argparse.ArgumentParser(
    description="Sample images from dataset based on metadata distribution."
)
parser.add_argument(
    "--dataset-roots",
    nargs="+",
    required=True,
    help="List of dataset root directories",
)
parser.add_argument(
    "--metadata-file",
    required=True,
    help="Path to dataset_description.ods file",
)
parser.add_argument(
    "--output-dir",
    required=True,
    help="Path to output directory where images will be copied",
)
args = parser.parse_args()

DATASET_ROOTS = [Path(p) for p in args.dataset_roots]
METADATA_FILE = Path(args.metadata_file)
OUTPUT_DIR = Path(args.output_dir)
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
TARGET_SAMPLE_SIZE = 5000

lighting_distribution = {
    "Indoor": TARGET_SAMPLE_SIZE // 3,
    "Outdoor": TARGET_SAMPLE_SIZE // 3,
    "Mixed": TARGET_SAMPLE_SIZE - 2 * (TARGET_SAMPLE_SIZE // 3),
}

shadow_distribution = {
    "Neither": TARGET_SAMPLE_SIZE // 4,
    "Light Reflections": TARGET_SAMPLE_SIZE // 4,
    "Shadows": TARGET_SAMPLE_SIZE // 4,
    "Both": TARGET_SAMPLE_SIZE - 3 * (TARGET_SAMPLE_SIZE // 4),
}


def categorize_lighting(value: str) -> str:
    for condition in ["Indoor", "Outdoor", "Mixed"]:
        if value.startswith(condition):
            return condition
    return "Other"


def label_to_color(label: str) -> str:
    colors = {
        "Images": "#707070",
        "Outdoor": "#ff5151",
        "Indoor": "#560000",
        "Mixed": "#a62829",
        "Neither": "#8c61ff",
        "Light Reflections": "#5b3fbf",
        "Shadows": "#2e2083",
        "Both": "#01004b",
        "Spray-painted (fair)": "#5fff5c",
        "Taped": "#40a42a",
        "Spray-painted (poor)": "#1b5200",
        "Inconsistent Field Color": "#ffb800",
        "Consistent Field Color": "#c46800",
    }
    return colors[label]


def plot_icicle(distribution, title):
    df = build_icicle_dataframe(distribution)
    label_color_map = {label: label_to_color(label) for label in df["labels"]}
    df["color"] = df["labels"].map(label_color_map)
    fig = go.Figure(
        go.Icicle(
            ids=df["ids"],
            labels=df["labels"],
            parents=df["parents"],
            values=df["values"],
            marker={"colors": df["color"]},
            root_color="lightgrey",
            tiling={"orientation": "v", "flip": "y"},
            textinfo="percent parent",  # "label+percent parent",
        )
    )
    fig.update_layout(title=title, margin={"t": 50, "l": 25, "r": 25, "b": 25})
    fig.show()


def build_icicle_dataframe(distribution):
    counts = defaultdict(int)
    for row in distribution:
        key = (
            row["Light Conditions"],
            row["Shadows/Light Reflections"],
            row["Line Conditions"],
            row["Field Conditions"],
        )
        counts[key] += 1

    ids = []
    labels = []
    parents = []
    values = []

    ids.append("root")
    labels.append("Images")
    parents.append("")
    values.append(0)  # Placeholder, not used with tiling

    ids, labels, parents, values = ["root"], ["Images"], [""], [0]
    for (light, shadow, line, field), count in counts.items():
        id_light = light
        id_shadow = f"{light}/{shadow}"
        id_line = f"{id_shadow}/{line}"
        id_field = f"{id_line}/{field}"
        for i, (id_, label_, parent_) in enumerate(
            [
                (id_light, light, "root"),
                (id_shadow, shadow, id_light),
                (id_line, line, id_shadow),
                (id_field, field, id_line),
            ]
        ):
            if id_ not in ids:
                ids.append(id_)
                labels.append(label_)
                parents.append(parent_)
                values.append(0 if i < 3 else count)
    return pd.DataFrame(
        {"ids": ids, "labels": labels, "parents": parents, "values": values}
    )


def print_summary(distribution, label):
    df = pd.DataFrame(distribution)
    print(f"\n--- {label} Summary ---")
    print(df["File Format"].value_counts().to_string())
    print(df["Color Space"].value_counts().to_string())
    print("\nTop/Bottom Camera Count:")
    print(
        df["Path"]
        .str.contains("bottomCamera")
        .value_counts()
        .rename(index={True: "Bottom", False: "Top"})
    )


metadata = pd.read_excel(METADATA_FILE, engine="odf").set_index("ID")
bucket = defaultdict(list)
distribution_full_dataset = []

if SAMPLE:
    for root in DATASET_ROOTS:
        print(f"Root: {root}")
        for game_folder in root.iterdir():
            if not game_folder.is_dir():
                continue
            try:
                game_id = int(game_folder.name[1:4])
            except:
                continue
            if game_id not in metadata.index:
                continue
            print(f"  - Game ID: {game_id}")
            row = metadata.loc[game_id]
            lighting = categorize_lighting(str(row["Light Conditions"]))
            shadow = str(row["Shadows/Light Reflections"])
            line = str(row["Line Conditions"])
            field = str(row["Field Conditions"])
            resolution = str(row["Resolution"])
            color_space = str(row["Color Space"])
            if (
                lighting not in lighting_distribution
                or shadow not in shadow_distribution
            ):
                continue
            for robot_dir in game_folder.iterdir():
                cam_dirs = (
                    ["topCamera", "bottomCamera"]
                    if resolution == "480x640"
                    else ["topCamera"]
                )
                for cam_dir in cam_dirs:
                    img_dir = robot_dir / cam_dir
                    if img_dir.exists():
                        for img in img_dir.glob("*.*"):
                            ext = img.suffix.lower()
                            if ext in [".png", ".jpg", ".jpeg"]:
                                bucket[(lighting, shadow)].append(str(img))
                                distribution_full_dataset.append(
                                    {
                                        "Light Conditions": lighting,
                                        "Shadows/Light Reflections": shadow,
                                        "Line Conditions": line,
                                        "Field Conditions": field,
                                        "File Format": ext,
                                        "Color Space": color_space,
                                        "Path": str(img),
                                    }
                                )

    sampled_images = []
    for lighting in lighting_distribution:
        per_shadow = TARGET_SAMPLE_SIZE // 12
        # collect all images for this lighting condition, across all shadow categories
        all_candidates = [
            img
            for (l, _), imgs in bucket.items()
            if l == lighting
            for img in imgs
        ]
        random.shuffle(all_candidates)
        used = set()
        for shadow in shadow_distribution:
            key = (lighting, shadow)
            candidates = bucket.get(key, [])
            if len(candidates) >= per_shadow:
                sampled = random.sample(candidates, per_shadow)
            else:
                sampled = candidates
                remaining = per_shadow - len(candidates)
                extras = [
                    img
                    for img in all_candidates
                    if img not in sampled and img not in used
                ][:remaining]
                sampled += extras
            used.update(sampled)
            sampled_images.extend(sampled)

    # Create RGB and YCrCb subfolders
    rgb_dir = OUTPUT_DIR / "RGB"
    ycrcb_dir = OUTPUT_DIR / "YCbCr"
    rgb_dir.mkdir(exist_ok=True)
    ycrcb_dir.mkdir(exist_ok=True)

    random.shuffle(sampled_images)
    name_map = {}
    for i, img_path in enumerate(sampled_images):
        new_name = f"{i:05}.png"
        img = cv2.imread(str(img_path))
        if img is None:
            print(f"⚠️ Could not read image {img_path}")
            continue
        try:
            game_folder = Path(img_path).parents[2]
            game_id = int(game_folder.name[1:4])
            row = metadata.loc[game_id]
            color_space = row["Color Space"]
        except:
            print(f"⚠️ Metadata missing or error for {img_path}")
            continue

        rgb_target = rgb_dir / new_name
        ycrcb_target = ycrcb_dir / new_name

        if color_space == "RGB":
            cv2.imwrite(str(rgb_target), img)
            converted = convert_BGR_to_YCrCb(img)
            cv2.imwrite(str(ycrcb_target), converted)
        elif color_space == "YCbCr":
            img = img[..., [2, 0, 1]]
            cv2.imwrite(str(ycrcb_target), img)
            converted = convert_YCrCb_to_BGR(img)
            cv2.imwrite(str(rgb_target), converted)
        else:
            print(f"⚠️ Unknown color space '{color_space}' for image {img_path}")
            continue

        name_map[new_name] = img_path

    print(
        f"✅ Sampled {len(sampled_images)} images and copied to {OUTPUT_DIR}\n"
    )

    with open(OUTPUT_DIR / "sampled_images_map.txt", "w") as f:
        for new_name, old_path in name_map.items():
            f.write(f"{new_name} -> {old_path}\n")

    distribution_sampled_dataset = []
    for _, old_path in name_map.items():
        try:
            img_path = Path(old_path)
            game_folder = img_path.parents[2]
            game_id = int(game_folder.name[1:4])
            if game_id not in metadata.index:
                continue
            row = metadata.loc[game_id]
            distribution_sampled_dataset.append(
                {
                    "Light Conditions": categorize_lighting(
                        str(row["Light Conditions"])
                    ),
                    "Shadows/Light Reflections": str(
                        row["Shadows/Light Reflections"]
                    ),
                    "Line Conditions": row["Line Conditions"],
                    "Field Conditions": row["Field Conditions"],
                    "File Format": img_path.suffix.lower(),
                    "Color Space": row["Color Space"],
                    "Path": old_path,
                }
            )
        except Exception as e:
            print(f"⚠️ Skipping image {old_path}: {e}")
else:
    label_folder = os.path.join(OUTPUT_DIR, "labels_binary")
    labeled_names = {f.name for f in Path(label_folder).glob("*.png")}
    name_map_path = os.path.join(OUTPUT_DIR, "sampled_images_map.txt")

    new_to_old = {}
    with open(name_map_path) as f:
        for line in f:
            if "->" in line:
                new, old = line.strip().split(" -> ")
                new_to_old[new] = old

    distribution_sampled_dataset = []
    for new_name in labeled_names:
        if new_name not in new_to_old:
            print(f"⚠️ Couldn't find a mapping of file {new_name}")
            continue
        old_path = new_to_old[new_name]
        try:
            img_path = Path(old_path)
            game_folder = img_path.parents[2]
            game_id = int(game_folder.name[1:4])
            if game_id not in metadata.index:
                print(
                    f"⚠️ Couldn't find the metadata for game {game_id} (file {new_name})"
                )
                continue
            row = metadata.loc[game_id]
            distribution_sampled_dataset.append(
                {
                    "Light Conditions": categorize_lighting(
                        str(row["Light Conditions"])
                    ),
                    "Shadows/Light Reflections": str(
                        row["Shadows/Light Reflections"]
                    ),
                    "Line Conditions": row["Line Conditions"],
                    "Field Conditions": row["Field Conditions"],
                    "File Format": img_path.suffix.lower(),
                    "Color Space": row["Color Space"],
                    "Path": old_path,
                }
            )
        except Exception as e:
            print(
                f"⚠️ Could not load metadata for {new_name} -> {old_path}: {e}"
            )

# plot_icicle(distribution_full_dataset, "Full Dataset - Image Distribution")
plot_icicle(
    distribution_sampled_dataset, "Sampled Dataset - Image Distribution"
)

# print_summary(distribution_full_dataset, "Full Dataset")
print_summary(distribution_sampled_dataset, "Sampled Dataset")
