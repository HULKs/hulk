import configparser
import json
import math
import os
import click
import cv2
import numpy as np
import tfrecord


def convert_to_ycbcr(config, image):
    if config['image_color_space'] == 'YCbCr':
        image[..., :] = image[..., [2, 0, 1]]
    elif config['image_color_space'] == 'RGB' or config['image_color_space'] == 'Grayscale':
        image = cv2.cvtColor(image, cv2.COLOR_BGR2YCrCb)
    else:
        raise RuntimeError(
            f'Unexpected color space {config["image_color_space"]}')

    return image


def generate_random_sample_positions(config, random_generator, image, circle):
    sample_size = np.array([[config['sample_size']],
                            [config['sample_size']]])
    annotation_box_upper_left = np.array([[circle['centerX'] - circle['radius']],
                                          [circle['centerY'] - circle['radius']]])
    annotation_box_size = np.array([[2 * circle['radius']],
                                    [2 * circle['radius']]])
    annotation_box_size_x = np.array([[2 * circle['radius']],
                                      [                   0]])

    rotation = random_generator.uniform(
        -config['maximum_sample_rotation'], config['maximum_sample_rotation'])
    scale_factor = random_generator.uniform(
        config['minimum_sample_scale_factor'], config['maximum_sample_scale_factor'])
    translation_factor = np.array([[random_generator.uniform(-config['maximum_sample_translation_factor'], config['maximum_sample_translation_factor'])],
                                   [random_generator.uniform(-config['maximum_sample_translation_factor'], config['maximum_sample_translation_factor'])]])

    def translate2d(t):
        return np.array([[1, 0, t[0, 0]],
                         [0, 1, t[1, 0]],
                         [0, 0,       1]])

    def scale2d(s):
        return np.array([[s[0, 0],       0, 0],
                         [      0, s[1, 0], 0],
                         [      0,       0, 1]])

    def rotate(a):
        return np.array([[np.cos(np.deg2rad(a)), -np.sin(np.deg2rad(a)), 0],
                         [np.sin(np.deg2rad(a)),  np.cos(np.deg2rad(a)), 0],
                         [                    0,                      0, 1]])

    def to3d(x):
        return np.array([[x[0, 0]],
                         [x[1, 0]],
                         [      1]])

    def to2d(x):
        return x[0:2]

    transformed_circle_to_sample = np.matmul(
        translate2d(sample_size / 2),
        np.matmul(
            translate2d(sample_size * -translation_factor),
            np.matmul(
                rotate(rotation),
                np.matmul(
                    scale2d(sample_size / scale_factor / annotation_box_size),
                    translate2d(-annotation_box_upper_left -
                                (annotation_box_size / 2)),
                ),
            ),
        ),
    )

    # circle positions on the cropped sample
    circle_center = to2d(
        np.matmul(
            transformed_circle_to_sample,
            to3d(annotation_box_upper_left + (annotation_box_size / 2)),
        ),
    )
    circle_middle_right = to2d(
        np.matmul(
            transformed_circle_to_sample,
            to3d(annotation_box_upper_left +
                 ((annotation_box_size + annotation_box_size_x) / 2)),
        ),
    )
    circle_radius = np.linalg.norm(circle_middle_right - circle_center)

    return circle_center, circle_radius, to2d(transformed_circle_to_sample).astype(np.float)


def crop_sample(config, image, transform):
    # apply transform to image
    sample_image = cv2.warpAffine(
        src=image,
        M=transform,
        dsize=(config['sample_size'], config['sample_size']),
        flags=cv2.INTER_NEAREST,
        borderMode=cv2.BORDER_CONSTANT,
        borderValue=[
            config['default_gray'],
            128,
            128,
        ],
    )

    # convert color space
    if config['sample_color_space'] == 'RGB':
        sample_image = cv2.cvtColor(sample_image, cv2.COLOR_YCrCb2BGR)
    elif config['sample_color_space'] == 'Grayscale':
        sample_image = sample_image[..., 0]

    # resize to sample size
    return np.atleast_3d(cv2.resize(sample_image, (config['sample_size'], config['sample_size']), interpolation=cv2.INTER_NEAREST))


def write_sample_to_tfrecord(config, sample_image):
    components = 0
    if config['sample_color_space'] == 'YCbCr' or config['sample_color_space'] == 'RGB':
        components = 3
    elif config['sample_color_space'] == 'Grayscale':
        components = 1
    else:
        raise RuntimeError(
            f'Unexpected color space {config["sample_color_space"]}')

    return tfrecord.serialize(data_shape=(config['sample_size'], config['sample_size'], components), data=sample_image.ravel())


def sample_intersection_ratio(config, image, circle, transform):
    image_upper_left = np.zeros((2, 1))
    image_lower_right = np.array([[image.shape[1]],
                                  [image.shape[0]]])

    # approximate intersection ratio with monte carlo method
    radius = circle['radius'] * np.sqrt(
        np.random.uniform(
            0,
            1,
            config['intersection_test_point_amount'],
        ),
    )
    angle = np.random.uniform(
        0,
        2 * np.pi,
        config['intersection_test_point_amount'],
    )
    x = circle['centerX'] + radius * np.cos(angle)
    y = circle['centerY'] + radius * np.sin(angle)
    transformed_positions = np.matmul(
        transform, np.array([x, y, np.ones_like(x)]))
    intersected_amount = np.count_nonzero((x >= image_upper_left[0, 0]) &
                                          (x <= image_lower_right[0, 0]) &
                                          (y >= image_upper_left[1, 0]) &
                                          (y <= image_lower_right[1, 0]) &
                                          (transformed_positions[0] >= 0) &
                                          (transformed_positions[0] <= config['sample_size']) &
                                          (transformed_positions[1] >= 0) &
                                          (transformed_positions[1] <= config['sample_size']))

    return intersected_amount / config['intersection_test_point_amount']


def avoidance_intersection_ratio(config, image_path, transform):
    maximal_avoidance_intersection_ratio = 0

    if image_path in config['avoidance_annotations']:
        for avoidance_circle in config['avoidance_annotations'][image_path]:
            # approximate intersection ratio with monte carlo method
            radius = avoidance_circle['radius'] * np.sqrt(
                np.random.uniform(
                    0,
                    1,
                    config['avoidance_intersection_test_point_amount'],
                ),
            )
            angle = np.random.uniform(
                0,
                2 * np.pi,
                config['avoidance_intersection_test_point_amount'],
            )
            x = avoidance_circle['centerX'] + radius * np.cos(angle)
            y = avoidance_circle['centerY'] + radius * np.sin(angle)
            transformed_positions = np.matmul(
                transform, np.array([x, y, np.ones_like(x)]))
            intersected_amount = np.count_nonzero((transformed_positions[0] >= 0) &
                                                  (transformed_positions[0] <= config['sample_size']) &
                                                  (transformed_positions[1] >= 0) &
                                                  (transformed_positions[1] <= config['sample_size']))

            maximal_avoidance_intersection_ratio = max(
                maximal_avoidance_intersection_ratio,
                intersected_amount /
                config['avoidance_intersection_test_point_amount'],
            )

    return maximal_avoidance_intersection_ratio


@click.command()
@click.option('--image-random-seed', default=42, help='Random seed for images', show_default=True)
@click.option('--minimum-sample-scale-factor', default=1.35, help='Minimum sample scale factor for annotations', show_default=True)
@click.option('--maximum-sample-scale-factor', default=1.85, help='Maximum sample scale factor for annotations', show_default=True)
@click.option('--maximum-sample-translation-factor', default=0.6, help='Maximum sample translation factor for annotations', show_default=True)
@click.option('--maximum-sample-rotation', default=30, help='Maximum sample rotation for annotations (degree)', show_default=True)
@click.option('--samples-per-annotation', default=15, help='Samples per annotation', show_default=True)
@click.option('--sample-size', default=32, help='Sample size', show_default=True)
@click.option('--image-color-space', type=click.Choice(['YCbCr', 'RGB', 'Grayscale'], case_sensitive=False), default='YCbCr', help='The color space of source images', show_default=True)
@click.option('--sample-color-space', type=click.Choice(['YCbCr', 'RGB', 'Grayscale'], case_sensitive=False), default='Grayscale', help='The color space of source samples', show_default=True)
@click.option('--default-gray', default=128, help='Default gray (uint8 Y component in [0,255])', show_default=True)
@click.option('--maximum-attempts-factor-per-annotation', default=5, help='The maximum attempts factor per annotation', show_default=True)
@click.option('--label-type', type=click.Choice(['positive', 'negative']), default='positive', help='Type of label of generated samples', show_default=True)
@click.option('--intersection-test-point-amount', default=128, help='Test point amount for intersection ratio approximation', show_default=True)
@click.option('--intersection-ratio-threshold', default=0.25, help='Threshold for intersection ratio', show_default=True)
@click.option('--avoidance-annotations-file', type=click.File(mode='r'), help='Annotations file containing circles to avoid')
@click.option('--avoidance-intersection-test-point-amount', default=128, help='Test point amount for avoidance intersection ratio approximation', show_default=True)
@click.option('--avoidance-intersection-ratio-threshold', default=0, help='Threshold for intersection ratio', show_default=True)
@click.argument('input_annotations_file', type=click.File(mode='r'))
@click.argument('output_samples_file', type=click.File(mode='wb'))
def main(**config):
    config['avoidance_annotations'] = json.load(
        config['avoidance_annotations_file']) if config['avoidance_annotations_file'] is not None else {}

    random_generator = np.random.default_rng(config['image_random_seed'])
    components = 0
    if config['sample_color_space'] == 'YCbCr' or config['sample_color_space'] == 'RGB':
        components = 3
    elif config['sample_color_space'] == 'Grayscale':
        components = 1

    tfrecord_examples = []
    annotations = json.load(config['input_annotations_file'])
    with click.progressbar(annotations.items()) as annotation_items:
        for image_path, circles in annotation_items:
            if len(circles) <= 0:
                continue

            image = cv2.imread(image_path)

            convert_to_ycbcr(config, image)

            for circle in circles:
                valid_samples_found = 0
                for _ in range(int(config['samples_per_annotation'] * config['maximum_attempts_factor_per_annotation'])):
                    if valid_samples_found >= config['samples_per_annotation']:
                        break

                    circle_center, circle_radius, transform = generate_random_sample_positions(
                        config, random_generator, image, circle)

                    if avoidance_intersection_ratio(config, image_path, transform) > config['avoidance_intersection_ratio_threshold']:
                        continue

                    if sample_intersection_ratio(config, image, circle, transform) < config['intersection_ratio_threshold']:
                        continue

                    sample_image = crop_sample(config, image, transform)

                    valid_samples_found += 1
                    tfrecord_examples.append(
                        tfrecord.serialize(
                            is_positive=config['label_type'] == 'positive',
                            circle=(
                                circle_center[0, 0],
                                circle_center[1, 0],
                                circle_radius,
                            ) if config['label_type'] == 'positive' else (0, 0, 0),
                            data_shape=(
                                config['sample_size'],
                                config['sample_size'],
                                components,
                            ),
                            data=sample_image.ravel(),
                        ),
                    )

    random_generator.shuffle(tfrecord_examples)

    for example in tfrecord_examples:
        config['output_samples_file'].write(example)
