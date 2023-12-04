import glob
import os
from os import path
import sys
import shutil

not_existing = 0

if input(f"Convert {sys.argv[1]} to a dataset? This action is DESTRUCTIVE!!! [Y/N]") != "Y":
    quit()

image_paths = glob.glob(path.join(sys.argv[1], "*.png"))
os.mkdir(path.join(sys.argv[1], "images"))
os.mkdir(path.join(sys.argv[1], "labels"))

for image_path in image_paths:
    json_path = path.splitext(image_path)[0] + ".json"
    if not path.exists(json_path):
        not_existing += 1
        print(image_path, json_path)
        os.remove(image_path)

print(f"Existing: {len(image_paths)-not_existing}/{len(image_paths)}")
print(f"{100 * (len(image_paths) - not_existing) / len(image_paths)}%")

image_paths = glob.glob(path.join(sys.argv[1], "*.png"))
json_paths = glob.glob(path.join(sys.argv[1], "*.json"))

for image_path in image_paths:
    shutil.move(image_path, path.join(sys.argv[1], "images"))

for json_path in json_paths:
    shutil.move(json_path, path.join(sys.argv[1], "labels"))