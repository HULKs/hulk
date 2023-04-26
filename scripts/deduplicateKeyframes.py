import argparse
import json
from copy import deepcopy


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("path", help="path to JSON motion file")
    parser.add_argument("-i", "--inplace", action="store_true")

    args = parser.parse_args()

    with open(args.path, encoding="utf-8") as fd:
        parsed = json.load(fd)

    current = deepcopy(parsed["initial_positions"])
    new_frames = []
    for frame in parsed["frames"]:
        new_frame = {"duration": frame["duration"], "positions": {}}
        for group_name, group in frame["positions"].items():
            new_group = {}
            for joint_name, joint in group.items():
                if current[group_name][joint_name] != joint:
                    new_group[joint_name] = joint
                    current[group_name][joint_name] = joint
            if new_group:
                new_frame["positions"][group_name] = new_group
        new_frames.append(new_frame)

    result = json.dumps(
        {"initial_positions": parsed["initial_positions"], "frames": new_frames},
        indent=2,
    )

    if args.inplace:
        with open(args.path, "w", encoding="utf-8") as fd:
            fd.write(result)
            fd.write("\n")
    else:
        print(result)


if __name__ == "__main__":
    main()
