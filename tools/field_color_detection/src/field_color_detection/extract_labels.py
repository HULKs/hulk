import json
import os
import re
from collections import defaultdict

import cv2
import numpy as np
from PIL import Image, ImageDraw

from .settings import ColorValues


def get_category_colors(label: str, *, binary_mask: bool) -> tuple:
    if binary_mask:
        return (
            ColorValues.FIELD_COLOR.value
            if label == "Field"
            else ColorValues.NOT_FIELD_COLOR.value
        )
    colors = {
        "Field": (5, 41, 245),
        "NotField": (0, 213, 255),
        "Lines": (10, 199, 70),
        "Robots": (199, 193, 10),
        "Other": (199, 10, 45),
        "Goal": (193, 10, 199),
        "Balls": (10, 92, 199),
    }
    return colors[label]


class InputStream:
    def __init__(self, data):
        self.data = data
        self.i = 0

    def read(self, size):
        out = self.data[self.i : self.i + size]
        self.i += size
        return int(out, 2)


def access_bit(data, num):
    """from bytes array to bits by num position"""
    base = int(num // 8)
    shift = 7 - int(num % 8)
    return (data[base] & (1 << shift)) >> shift


def bytes2bit(data):
    """get bit string from bytes data"""
    return "".join([str(access_bit(data, i)) for i in range(len(data) * 8)])


def rle2mask(rle, shape=(480, 640)):
    """from LS RLE to numpy uint8 3d image [width, height, channel]"""
    input = InputStream(bytes2bit(rle))
    num = input.read(32)
    word_size = input.read(5) + 1
    rle_sizes = [input.read(4) + 1 for _ in range(4)]
    i = 0
    out = np.zeros(num, dtype=np.uint8)
    while i < num:
        x = input.read(1)
        j = i + 1 + input.read(rle_sizes[input.read(2)])
        if x:
            val = input.read(word_size)
            out[i:j] = val
            i = j
        else:
            while i < j:
                val = input.read(word_size)
                out[i] = val
                i += 1
    return np.reshape(out, [shape[0], shape[1], 4])[:, :, 3]


def extract_coco(
    json_file_path,
    output_dir="output_color_masks",
    *,
    binary_mask: bool = False,
):
    os.makedirs(output_dir, exist_ok=True)

    with open(json_file_path) as f:
        data = json.load(f)

    for item in data:
        image_path = item["image"].split("=")[-1]
        image_name = os.path.splitext(os.path.basename(image_path))[0]
        height, width = 480, 640

        mask = np.zeros((height, width, 3), dtype=np.uint8)
        for tag in list(item.keys()):
            if not tag.startswith("tag"):
                continue

            annotations = item[tag]
            for ann in annotations:
                keys = ann.keys()
                if "polygonlabels" in keys:
                    label = ann["polygonlabels"][0]
                    color = get_category_colors(label, binary_mask=binary_mask)
                    if "points" in keys:
                        points = ann["points"]
                        if len(points) < 2:
                            continue
                        polygon = [
                            (p[0] / 100 * width, p[1] / 100 * height)
                            for p in points
                        ]
                        decoded_mask = Image.new("L", (width, height), 0)
                        ImageDraw.Draw(decoded_mask).polygon(
                            polygon, outline=1, fill=1
                        )
                        poly_mask = np.array(decoded_mask)
                        mask[poly_mask == 1] = color
                elif "brushlabels" in keys:
                    if "rle" not in keys:
                        continue
                    label = ann["brushlabels"][0]
                    color = get_category_colors(label, binary_mask=binary_mask)
                    decoded_mask = rle2mask(ann["rle"])
                    mask[decoded_mask != 0] = color
                else:
                    continue

        new_file_path = os.path.join(output_dir, f"{image_name}.png")
        cv2.imwrite(new_file_path, mask)
        # cv2.imshow("mask", mask)
        # cv2.waitKey()
        # cv2.destroyAllWindows()


def combine_masks(input_dir):
    output_dir = os.path.join(input_dir, "masks")
    os.makedirs(output_dir, exist_ok=True)

    LABEL_COLORS = {
        "Field": (0, 255, 0),  # Green
        "NotField": (255, 0, 0),  # Blue
    }
    OVERLAP_COLOR = (0, 0, 255)  # Red

    pattern = re.compile(
        r"task-(\d+)-annotation-\d+-by-\d+-tag-([^-]+)-\d+\.png"
    )
    tasks = defaultdict(lambda: defaultdict(list))

    for filename in os.listdir(input_dir):
        if not filename.endswith(".png"):
            continue
        match = pattern.match(filename)
        if match:
            task_id, tag = match.groups()
            tasks[task_id][tag].append(filename)

    for task_id, tag_files in tasks.items():
        combined_mask = None
        field_mask = None
        notfield_mask = None

        for tag, files in tag_files.items():
            mask_sum = None
            for file in files:
                path = os.path.join(input_dir, file)
                mask = cv2.imread(path, cv2.IMREAD_GRAYSCALE)
                mask_bin = (mask > 0).astype(np.uint8)

                if mask_sum is None:
                    mask_sum = mask_bin
                else:
                    mask_sum = np.clip(mask_sum + mask_bin, 0, 1)

            if combined_mask is None:
                height, width = mask_sum.shape
                combined_mask = np.zeros((height, width, 3), dtype=np.uint8)
                field_mask = np.zeros((height, width), dtype=np.uint8)
                notfield_mask = np.zeros((height, width), dtype=np.uint8)

            # Supports both binary and multi-class classification
            if tag == "Field":
                field_mask = np.clip(field_mask + mask_sum, 0, 1)
            elif tag != "Field":
                notfield_mask = np.clip(notfield_mask + mask_sum, 0, 1)

        overlap = field_mask * notfield_mask
        if np.any(overlap):
            print(f"⚠️  Overlap detected in task-{task_id}")

            preview = np.zeros_like(combined_mask)
            preview[np.where(field_mask == 1)] = LABEL_COLORS["Field"]
            preview[np.where(notfield_mask == 1)] = LABEL_COLORS["NotField"]
            preview[np.where(overlap == 1)] = OVERLAP_COLOR

            cv2.imshow(f"Resolve Overlap for Task {task_id}", preview)
            print(
                "Enter resolution: 0 for NotField, 1 for Field, 2 for neither"
            )
            key = -1
            while key not in [48, 49, 50]:  # ASCII for '0', '1', '2'
                key = cv2.waitKey(0)
            cv2.destroyAllWindows()

            if key == 48:  # '0'
                notfield_mask = np.clip(notfield_mask + overlap, 0, 1)
                field_mask[overlap == 1] = 0
            elif key == 49:  # '1'
                field_mask = np.clip(field_mask + overlap, 0, 1)
                notfield_mask[overlap == 1] = 0
            elif key == 50:  # '2'
                field_mask[overlap == 1] = 0
                notfield_mask[overlap == 1] = 0

        combined_mask[np.where(field_mask == 1)] = LABEL_COLORS["Field"]
        combined_mask[np.where(notfield_mask == 1)] = LABEL_COLORS["NotField"]

        output_path = os.path.join(
            output_dir, f"task-{task_id}_combined_mask.png"
        )
        cv2.imwrite(output_path, combined_mask)

    print("✅ Mask combination complete.")


if __name__ == "__main__":
    # parser = argparse.ArgumentParser(
    #     description="Combine binary label masks into a single colored mask."
    # )
    # parser.add_argument(
    #     "input_dir", type=str, help="Path to directory containing mask PNGs"
    # )
    # args = parser.parse_args()

    # combine_masks(args.input_dir)

    coco_file_path = "/home/franziska-sophie/Downloads/json_min_2.json"
    output_dir = (
        "/home/franziska-sophie/Downloads/sampled_images/labels_binary_2"
    )
    extract_coco(coco_file_path, output_dir, binary_mask=True)
