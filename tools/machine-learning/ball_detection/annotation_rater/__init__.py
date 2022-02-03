import json
import math
import click
import numpy as np


def squared_norm(a, b):
    return (a["centerX"] - b["centerX"]) ** 2 + (a["centerY"] - b["centerY"]) ** 2


def is_matching(ground_truth, subject):
    """Returns true if the two annotations are considered to mark the same object

    This implementation considers circles with any overlap to be matching"""
    maximum_squared_distance = (ground_truth["radius"] + subject["radius"]) ** 2
    squared_distance = (squared_norm(ground_truth, subject))
    return squared_distance < maximum_squared_distance


def find_matching_annotations(ground_truth, subject):
    matched = []
    false_positives = []
    false_negatives = []
    for annotation in ground_truth:
        found = False
        for subject_annotation in subject:
            if is_matching(annotation, subject_annotation):
                matched.append({
                    "ground_truth": annotation,
                    "subject": subject_annotation,
                    "radius_error": abs(subject_annotation["radius"] - annotation["radius"]),
                    "square_position_error": squared_norm(annotation, subject_annotation)
                })
                found = True
                break

        if not found:
            false_negatives.append(annotation)

    for subject_annotation in subject:
        found = False
        for annotation in ground_truth:
            if is_matching(annotation, subject_annotation):
                found = True
                break

        if not found:
            false_positives.append(subject_annotation)

    return matched, false_positives, false_negatives



def merge_annotations(annotations):
    """Remove duplicate annotations"""
    result = []
    for i, annotation in enumerate(annotations):
        found = False
        for second_annotation in annotations[:i]:
            if is_matching(annotation, second_annotation):
                break
                found = True

        if not found:
            result.append(annotation)

    return result


def compare(ground_truth, subject):
    total_matched = []
    total_false_positives = []
    total_false_negatives = []
    for image in ground_truth.keys():
        if image not in subject.keys():
            print(f"\"{image}\" missing from subject annotations file")
            continue
        merged_ground_truth = merge_annotations(ground_truth[image])
        merged_subject = merge_annotations(subject[image])
        matched, false_positives, false_negatives = find_matching_annotations(
            merged_ground_truth, merged_subject)
        total_matched += matched
        total_false_positives += false_positives
        total_false_negatives += false_negatives

    return total_matched, total_false_positives, total_false_negatives


def calculate_metrics(data, config):
    matched, false_positives, false_negatives = data
    position_errors = np.sqrt([m["square_position_error"] for m in matched])
    radius_errors = np.array([m["radius_error"] for m in matched])
    ground_truth_radii = [m["ground_truth"]["radius"] for m in matched]
    incorrect_position = (position_errors / ground_truth_radii) > config["correctness_tolerance"] / 100
    incorrect_radius = (radius_errors / ground_truth_radii) > config["correctness_tolerance"] / 100
    incorrect_annotation = np.logical_or(incorrect_position, incorrect_radius)
    metrics = {
        "total_annotations": len(matched) + len(false_positives) + len(false_positives),
        "matched": len(matched),
        "false_positives": len(false_positives),
        "false_negatives": len(false_negatives),
        "position_error_average": float(np.average(position_errors)),
        "position_error_variance": float(np.var(position_errors)),
        "radius_error_average": float(np.average(radius_errors)),
        "radius_error_variance": float(np.var(radius_errors)),
        "incorrect_positions": int(incorrect_position.sum()),
        "incorrect_radii": int(incorrect_radius.sum()),
        "incorrect_annotations": int(incorrect_annotation.sum())
    }
    return metrics


def print_metrics(metrics):

    def percentage_format(value):
        fraction = value/metrics["total_annotations"] if metrics["total_annotations"] != 0 else math.nan
        return f"{value:8} {fraction*100:8.2G}%"

    print(f"Matched Annotations:   " + percentage_format(metrics["matched"]))
    print(f"False Positives:       " + percentage_format(metrics["false_positives"]))
    print(f"False Negatives:       " + percentage_format(metrics["false_negatives"]))
    print()
    print(f"Position Error:        avg {metrics['position_error_average']:4.3G}  var {metrics['position_error_variance']:4.3G}")
    print(f"Radius Error:          avg {metrics['radius_error_average']:4.3G}  var {metrics['radius_error_variance']:4.3G}")
    print()
    print(f"Incorrect Positions:   " + percentage_format(metrics["incorrect_positions"]))
    print(f"Incorrect Radii:       " + percentage_format(metrics["incorrect_radii"]))
    print(f"Incorrect Annotations: " + percentage_format(metrics["incorrect_annotations"]))


@click.command()
@click.option("--json/--human-readable", default=False, help="Print json instead of formatted text", show_default=True)
@click.option("--correctness-tolerance", default=10, type=float, help="Highest tolerated radius adjusted error in percent", show_default=True)
@click.argument("ground_truth_annotations_file", type=click.File(mode="r"))
@click.argument("subject_annotations_file", type=click.File(mode="r"))
def main(**config):
    ground_truth = json.load(config["ground_truth_annotations_file"])
    subject = json.load(config["subject_annotations_file"])
    data = compare(ground_truth, subject)
    metrics = calculate_metrics(data, config)

    if config["json"]:
        print(json.dumps(metrics, indent=4))
        return
    print_metrics(metrics)
