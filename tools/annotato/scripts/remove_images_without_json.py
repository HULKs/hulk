import glob
import os
from os import path
import sys

image_paths = glob.glob(path.join(sys.argv[1], "*.png"))

not_existing = 0

for image_path in image_paths:
    json_path = path.splitext(image_path)[0] + ".json"
    if not path.exists(json_path):
        not_existing += 1
        print(image_path, json_path)
        os.remove(image_path)

print(f"Existing: {len(image_paths)-not_existing}/{len(image_paths)}")
print(f"{100 * (len(image_paths) - not_existing) / len(image_paths)}%")