import os
import random
import shutil

import cv2

from .data import convert_YCrCb_to_BGR, read_YCrCb_image


def find_png_files(directory: str) -> list[str]:
    png_files = []
    for root, _, files in os.walk(directory):
        images = [
            os.path.join(root, f) for f in files if f.lower().endswith(".png")
        ]
        if images:
            png_files.extend(images)
    return png_files


def create_RGB_previews(
    source_dir: str,
    target_dir: str,
    images_per_folder: int = 50,
    start_id: int = 0,
) -> None:
    os.makedirs(target_dir, exist_ok=True)
    for subfolder in os.listdir(source_dir):
        subfolder_path = os.path.join(source_dir, subfolder)
        if not os.path.isdir(subfolder_path):
            continue

        png_files = find_png_files(subfolder_path)
        if len(png_files) == 0:
            continue

        selected_files = random.sample(
            list(png_files), min(images_per_folder, len(png_files))
        )

        new_subfolder_path = os.path.join(target_dir, subfolder[:5])
        dataset_id = int(subfolder[1:4])
        if dataset_id < start_id:
            continue

        os.makedirs(new_subfolder_path, exist_ok=True)

        for file_path in selected_files:
            new_path = os.path.join(
                new_subfolder_path, os.path.basename(file_path)
            )
            image_YCrCb = read_YCrCb_image(file_path)
            image_RGB = convert_YCrCb_to_BGR(image_YCrCb)
            cv2.imwrite(new_path, image_RGB)

        print(f"Processed {len(selected_files)} images from {subfolder}")


def count_top_bottom_images(parent_folder: str) -> None:
    subfolders = os.listdir(parent_folder)
    subfolders.sort()
    for subfolder in subfolders:
        subfolder_path = os.path.join(parent_folder, subfolder)
        if not os.path.isdir(subfolder_path):
            continue

        png_files = find_png_files(subfolder_path)
        if len(png_files) == 0:
            continue

        top_count = sum(
            1 for path in png_files if "top" in os.path.basename(path).lower()
        )
        bottom_count = sum(
            1
            for path in png_files
            if "bottom" in os.path.basename(path).lower()
        )
        dataset_id = os.path.basename(subfolder)[:5]

        subsubfolders = [
            folder
            for folder in os.listdir(subfolder_path)
            if os.path.isdir(os.path.join(subfolder_path, folder))
            and folder.startswith("10.1.24")
        ]

        print(
            f"{dataset_id} {len(png_files):d} {top_count + bottom_count:d} {top_count:d} {bottom_count:d} {len(subsubfolders)}"
        )


def rename_folders(parent_folder: str, start_number: int) -> None:
    subfolders = [
        f
        for f in os.listdir(parent_folder)
        if os.path.isdir(os.path.join(parent_folder, f))
    ]
    subfolders.sort()

    for index, folder in enumerate(subfolders, start=start_number):
        new_name = f"[{index:03d}]_{folder}"
        old_path = os.path.join(parent_folder, folder)
        new_path = os.path.join(parent_folder, new_name)

        os.rename(old_path, new_path)
        print(f'Renamed: "{folder}" → "{new_name}"')


def count_images_in_log_folders(root_directory: str) -> tuple[int, int]:
    top_count = 0
    bottom_count = 0

    for root, _, files in os.walk(root_directory):
        folder_name = os.path.basename(root)
        image_files = [f for f in files if f.lower().endswith(".png")]

        if folder_name == "log_top":
            top_count += len(image_files)
        elif folder_name == "log_bottom":
            bottom_count += len(image_files)

    return top_count, bottom_count


def count_upper_lower_per_subfolder(root_dir: str) -> None:
    counts = {}

    for subfolder in os.listdir(root_dir):
        subfolder_path = os.path.join(root_dir, subfolder)
        if os.path.isdir(subfolder_path):
            lower_count = 0
            upper_count = 0

            for _, _, files in os.walk(subfolder_path):
                for file in files:
                    if file.lower().endswith(".png"):
                        if "lower" in file.lower():
                            lower_count += 1
                        if "upper" in file.lower():
                            upper_count += 1

            print(subfolder)
            print(f"{lower_count} {upper_count}")
            counts[subfolder] = (lower_count, upper_count)


def split_images_into_camera_folders(root_dir: str):
    for level1 in os.listdir(root_dir):
        level1_path = os.path.join(root_dir, level1)
        if not os.path.isdir(level1_path):
            continue

        for level2 in os.listdir(level1_path):
            level2_path = os.path.join(level1_path, level2)
            if not os.path.isdir(level2_path):
                continue

            top_dir = os.path.join(level2_path, "topCamera")
            bottom_dir = os.path.join(level2_path, "bottomCamera")
            os.makedirs(top_dir, exist_ok=True)
            os.makedirs(bottom_dir, exist_ok=True)

            for filename in os.listdir(level2_path):
                file_path = os.path.join(level2_path, filename)
                if not os.path.isfile(file_path) or not filename.endswith(
                    ".png"
                ):
                    continue

                if filename.startswith("upper_"):
                    shutil.copy(file_path, os.path.join(top_dir, filename))
                elif filename.startswith("lower_"):
                    shutil.copy(file_path, os.path.join(bottom_dir, filename))


def analyze_dataset(root_dir: str):
    results = {}

    for match_folder in sorted(os.listdir(root_dir)):
        match_path = os.path.join(root_dir, match_folder)
        if not os.path.isdir(match_path):
            continue

        ip_set = set()
        vision_top_count = 0
        vision_bottom_count = 0

        for half in ["first-half", "second-half", "golden-goal"]:
            half_path = os.path.join(match_path, half)
            if not os.path.isdir(half_path):
                continue

            for ip_folder in os.listdir(half_path):
                if not ip_folder.startswith("10.1.24."):
                    continue

                ip_set.add(ip_folder)
                ip_path = os.path.join(half_path, ip_folder)

                # Look for timestamp folder inside
                for ts_folder in os.listdir(ip_path):
                    ts_path = os.path.join(ip_path, ts_folder)
                    if not os.path.isdir(ts_path):
                        continue

                    vision_top = os.path.join(ts_path, "VisionTop")
                    vision_bottom = os.path.join(ts_path, "VisionBottom")

                    if os.path.isdir(vision_top):
                        vision_top_count += len(
                            [
                                f
                                for f in os.listdir(vision_top)
                                if f.endswith(".png")
                            ]
                        )

                    if os.path.isdir(vision_bottom):
                        vision_bottom_count += len(
                            [
                                f
                                for f in os.listdir(vision_bottom)
                                if f.endswith(".png")
                            ]
                        )

        results[match_folder] = {
            "unique_ip_count": len(ip_set),
            "vision_top_images": vision_top_count,
            "vision_bottom_images": vision_bottom_count,
        }

        print(
            f"{match_folder[:5]} {vision_top_count} {vision_bottom_count} {len(ip_set)}"
        )
    return results


if __name__ == "__main__":
    source_directory = "/home/franziska-sophie/Documents/Datasets/unsorted/GO25"
    target_directory = "/home/franziska-sophie/Documents/Datasets/RGB_preview"

    folder = (
        "/home/franziska-sophie/Documents/Datasets/[154]-[159] LogsFuerHulks"
    )
    # split_images_into_camera_folders(folder)

    # rename_folders(source_directory, 160)
    # create_RGB_previews(source_directory, target_directory)
    # count_top_bottom_images(source_directory)

    analyze_dataset(source_directory)
