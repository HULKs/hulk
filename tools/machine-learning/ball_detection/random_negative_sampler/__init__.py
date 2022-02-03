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
        raise RuntimeError(f'Unexpected color space {config["image_color_space"]}')
    
    return image


def replace_circles_with_gray(config, image, circles):
    for circle in circles:
        minimum_x = max(0, math.floor(circle['centerX'] - circle['radius']))
        maximum_x = min(image.shape[1], math.ceil(circle['centerX'] + circle['radius']))
        minimum_y = max(0, math.floor(circle['centerY'] - circle['radius']))
        maximum_y = min(image.shape[0], math.ceil(circle['centerY'] + circle['radius']))
        for y in range(minimum_y, maximum_y):
            for x in range(minimum_x, maximum_x):
                image[y, x, :] = [
                    config['default_gray'],
                    128,
                    128,
                ]
    
    return image


def generate_random_sample(config, random_generator, image):
    size = int(random_generator.uniform(config['minimum_sample_size_in_image'], config['maximum_sample_size_in_image']))
    center_x = int(random_generator.uniform(0, image.shape[1]))
    center_y = int(random_generator.uniform(0, image.shape[0]))
    
    sample_image = cv2.copyMakeBorder(
        image,
        top=size,
        bottom=size,
        left=size,
        right=size,
        borderType=cv2.BORDER_CONSTANT,
        value=[
            config['default_gray'],
            128,
            128,
        ],
    )
    
    crop_x = int(center_x + (size / 2))
    crop_y = int(center_y + (size / 2))
    crop_width = int(size)
    crop_height = int(size)
    sample_image = sample_image[crop_y:crop_y + crop_height, crop_x:crop_x+crop_width]
    
    if config['sample_color_space'] == 'RGB':
        sample_image = cv2.cvtColor(sample_image, cv2.COLOR_YCrCb2BGR)
    elif config['sample_color_space'] == 'Grayscale':
        sample_image = sample_image[..., 0]  # TODO: Refactor to use (32, 32, 3) for grayscale (reconsider JPEG defaults)
    
    sample_image = cv2.resize(sample_image, (config['sample_size'], config['sample_size']), interpolation=cv2.INTER_NEAREST)
    
    return sample_image


def write_sample_to_tfrecord(config, sample_image):
    components = 0
    if config['sample_color_space'] == 'YCbCr' or config['sample_color_space'] == 'RGB':
        components = 3
    elif config['sample_color_space'] == 'Grayscale':
        components = 1
    else:
        raise RuntimeError(f'Unexpected color space {config["sample_color_space"]}')

    return tfrecord.serialize(data_shape=(config['sample_size'], config['sample_size'], components), data=sample_image.ravel())


def amount_of_segments_in(config, image, positions_x, positions_y):
    # input parameters example:
    # positions_x: [0,1,2,3,4,5,6,7,8,...,30,31]
    # positions_y: [3,3,3,3,3,3,3,3,3,..., 3, 3]
    # zip(positions_x, positions_y) -> [(0,3), (1,3), ...]
    amount_of_segments = 1
    current_segment_accumulated_color = 0
    current_segment_amount = 0
    for x, y in zip(positions_x, positions_y):
        color = image[y, x]
        current_segment_color = current_segment_accumulated_color / current_segment_amount if current_segment_amount != 0 else color
        if abs(color - current_segment_color) < config['segments_color_threshold']:
            current_segment_accumulated_color += color
            current_segment_amount += 1
        else:
            amount_of_segments += 1
            current_segment_accumulated_color = 0
            current_segment_amount = 0
    return amount_of_segments


def collect_segments_samples(config, random_generator, image):
    if config['sample_color_space'] != 'Grayscale':
        raise RuntimeError(f'collect_segments_samples not implemented for sample color space {config["sample_color_space"]}')
    
    valid_samples_found = 0
    tfrecord_examples = []
    for _ in range(int(config['segments_amount'] * config['maximum_attempts_factor_per_image'])):
        if valid_samples_found >= config['segments_amount']:
            break
        
        sample_image = generate_random_sample(config, random_generator, image)
        
        amount_of_horizontal_segments = sum([
            amount_of_segments_in(config, sample_image, range(config['sample_size']), [y] * config['sample_size'])
            for y in range(config['sample_size'])
        ])
        amount_of_vertical_segments = sum([
            amount_of_segments_in(config, sample_image, [x] * config['sample_size'], range(config['sample_size']))
            for x in range(config['sample_size'])
        ])
        amount_of_segments = max(amount_of_horizontal_segments, amount_of_vertical_segments)
        
        if amount_of_segments > config['segments_lower_bound_amount']:
            valid_samples_found += 1
            tfrecord_examples.append(write_sample_to_tfrecord(config, sample_image))
    
    return tfrecord_examples


def collect_jpeg_samples(config, random_generator, image):
    valid_samples_found = 0
    tfrecord_examples = []
    for _ in range(int(config['jpeg_amount'] * config['maximum_attempts_factor_per_image'])):
        if valid_samples_found >= config['jpeg_amount']:
            break
        
        sample_image = generate_random_sample(config, random_generator, image)
        
        _, jpeg = cv2.imencode('.jpeg', sample_image, [int(cv2.IMWRITE_JPEG_QUALITY), config['jpeg_quality']])
        size = len(jpeg.tobytes())
        
        if size > config['jpeg_lower_bound_size']:
            valid_samples_found += 1
            tfrecord_examples.append(write_sample_to_tfrecord(config, sample_image))
    
    return tfrecord_examples


def collect_sobel_samples(config, random_generator, image):
    valid_samples_found = 0
    tfrecord_examples = []
    for _ in range(int(config['sobel_amount'] * config['maximum_attempts_factor_per_image'])):
        if valid_samples_found >= config['sobel_amount']:
            break
        
        sample_image = generate_random_sample(config, random_generator, image)
        
        image_blurred = cv2.GaussianBlur(src=sample_image, ksize=(config['sobel_blur_kernel_size'], config['sobel_blur_kernel_size']), sigmaX=0, sigmaY=0, borderType=cv2.BORDER_DEFAULT)
        grad_x = cv2.Sobel(src=image_blurred, ddepth=cv2.CV_16S, dx=1, dy=0, ksize=config['sobel_filter_kernel_size'], scale=config['sobel_scale'], delta=0, borderType=cv2.BORDER_DEFAULT)
        grad_y = cv2.Sobel(src=image_blurred, ddepth=cv2.CV_16S, dx=0, dy=1, ksize=config['sobel_filter_kernel_size'], scale=config['sobel_scale'], delta=0, borderType=cv2.BORDER_DEFAULT)
        abs_grad_x = cv2.convertScaleAbs(grad_x)
        abs_grad_y = cv2.convertScaleAbs(grad_y)
        sobel_image = cv2.addWeighted(src1=abs_grad_x, alpha=0.5, src2=abs_grad_y, beta=0.5, gamma=0)
        
        amount = np.sum(sobel_image)
        
        if amount > config['sobel_lower_bound_sum']:
            valid_samples_found += 1
            tfrecord_examples.append(write_sample_to_tfrecord(config, sample_image))
    
    return tfrecord_examples


def collect_entropy_samples(config, random_generator, image):
    valid_samples_found = 0
    tfrecord_examples = []
    for _ in range(int(config['entropy_amount'] * config['maximum_attempts_factor_per_image'])):
        if valid_samples_found >= config['entropy_amount']:
            break
        
        sample_image = generate_random_sample(config, random_generator, image)
        
        image_blurred = cv2.GaussianBlur(src=sample_image, ksize=(config['entropy_blur_kernel_size'], config['entropy_blur_kernel_size']), sigmaX=0, sigmaY=0, borderType=cv2.BORDER_DEFAULT)
        colors = image_blurred.ravel().tolist()
    
        frequencies = {}
        for c in set(colors):
            frequencies[c] = colors.count(c)
    
        entropy = 0
        for frequency in frequencies.values():
            probability = frequency / len(colors)
            information_content = np.log2(1 / probability)
            entropy += probability * information_content
        
        if entropy > config['entropy_lower_bound']:
            valid_samples_found += 1
            tfrecord_examples.append(write_sample_to_tfrecord(config, sample_image))
    
    return tfrecord_examples


@click.command()
@click.option('--image-random-seed', default=42, help='Random seed for images', show_default=True)
@click.option('--minimum-sample-size-in-image', default=16, help='Minimum sample size in image', show_default=True)
@click.option('--maximum-sample-size-in-image', default=240, help='Maximum sample size in image', show_default=True)
@click.option('--sample-size', default=32, help='Sample size', show_default=True)
@click.option('--image-color-space', type=click.Choice(['YCbCr', 'RGB', 'Grayscale'], case_sensitive=False), default='YCbCr', help='The color space of source images', show_default=True)
@click.option('--sample-color-space', type=click.Choice(['YCbCr', 'RGB', 'Grayscale'], case_sensitive=False), default='Grayscale', help='The color space of source samples', show_default=True)
@click.option('--default-gray', default=128, help='Default gray (uint8 Y component in [0,255])', show_default=True)
@click.option('--maximum-attempts-factor-per-image', default=3, help='The maximum attempts factor per method', show_default=True)
@click.option('--segments-amount', default=5, help='Amount of samples to generate with segments method', show_default=True)
@click.option('--segments-color-threshold', default=60, help='Threshold for segment clustering', show_default=True)
@click.option('--segments-lower-bound-amount', default=60, help='Required lower bound of amount of segments', show_default=True)
@click.option('--jpeg-amount', default=5, help='Amount of samples to generate with JPEG method', show_default=True)
@click.option('--jpeg-quality', default=50, help='JPEG quality', show_default=True)
@click.option('--jpeg-lower-bound-size', default=500, help='Required lower bound of JPEG compressed size', show_default=True)
@click.option('--sobel-amount', default=5, help='Amount of samples to generate with sobel method', show_default=True)
@click.option('--sobel-blur-kernel-size', default=5, help='Gaussian kernel size (odd numbers required)', show_default=True)
@click.option('--sobel-filter-kernel-size', default=3, help='Sobel filter kernel size (odd numbers required)', show_default=True)
@click.option('--sobel-scale', default=1, help='Sobel filter scale', show_default=True)
@click.option('--sobel-lower-bound-sum', default=30000, help='Required lower bound of sum of colors in edge detected image', show_default=True)
@click.option('--entropy-amount', default=5, help='Amount of samples to generate with entropy method', show_default=True)
@click.option('--entropy-blur-kernel-size', default=5, help='Gaussian kernel size (odd numbers required)', show_default=True)
@click.option('--entropy-lower-bound', default=6, help='Required lower bound of entropy', show_default=True)
@click.argument('input_annotations_file', type=click.File(mode='r'))
@click.argument('output_samples_file', type=click.File(mode='wb'))
def main(**config):
    random_generator = np.random.default_rng(config['image_random_seed'])
    
    tfrecord_examples = []
    annotations = json.load(config['input_annotations_file'])
    with click.progressbar(annotations.items()) as annotation_items:
        for image_path, circles in annotation_items:
            try:
                image = cv2.imread(image_path)
                
                convert_to_ycbcr(config, image)
                
                replace_circles_with_gray(config, image, circles)
                
                tfrecord_examples += collect_segments_samples(config, random_generator, image)
                tfrecord_examples += collect_jpeg_samples(config, random_generator, image)
                tfrecord_examples += collect_sobel_samples(config, random_generator, image)
                tfrecord_examples += collect_entropy_samples(config, random_generator, image)
            except TypeError:
                print(f'Failed to read {image_path}')
    
    random_generator.shuffle(tfrecord_examples)
    
    for example in tfrecord_examples:
        config['output_samples_file'].write(example)
