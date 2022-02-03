import copy
import imghdr
import io
import json
import struct
import threading
import click
import crc32c
import cv2
import flask
import viewer.feature_pb2
import viewer.example_pb2
import numpy as np
import image as img

app = flask.Flask(__name__, static_folder='static',
                  static_url_path='/viewer/static')


k_mask_delta = 0xa282ead8


def masked_crc32c(data: bytes):
    checksum = crc32c.crc32(data)
    return ((((checksum >> 15) & 0xffffffff) | ((checksum << 17) & 0xffffffff)) + k_mask_delta) & 0xffffffff


@app.route('/')
def root():
    return app.send_static_file('index.html')


@app.route('/config.json')
def config_json():
    return flask.jsonify({
        'gridImageSize': app.config['arguments']['grid_image_size'],
    })


@app.route('/samples.json')
def samples_json():
    return flask.jsonify(len(app.config['sample_offsets']))


@app.route('/sample/<int:index>')
def sample(index):
    if index < 0:
        return flask.make_response('index cannot be negative', 404)
    if index >= len(app.config['sample_offsets']):
        return flask.make_response('index cannot be larger than amount of images', 404)

    sample_offset = app.config['sample_offsets'][index]

    with app.config['input_samples_file_lock']:
        input_samples_file = app.config['arguments']['input_samples_file']
        input_samples_file.seek(sample_offset[0])
        data_bytes = input_samples_file.read(sample_offset[1])
        computed_masked_crc32_of_data = masked_crc32c(data_bytes)
        masked_crc32_of_data_bytes = input_samples_file.read(4)
        masked_crc32_of_data = struct.unpack(
            '<L', masked_crc32_of_data_bytes)[0]

    if masked_crc32_of_data != computed_masked_crc32_of_data:
        raise RuntimeError(
            f'CRC integrity check failed for data in input samples file at {sample_offset[0]}')

    example = viewer.example_pb2.Example()
    example.ParseFromString(data_bytes)

    data_shape_feature = example.features.feature['dataShape']
    assert data_shape_feature.HasField('int64_list')
    data_shape = data_shape_feature.int64_list.value
    data_feature = example.features.feature['data']
    assert data_feature.HasField('int64_list')
    data = np.reshape(
        np.array(data_feature.int64_list.value, dtype=np.uint8), tuple(data_shape))

    image = np.repeat(data, repeats=3, axis=2)

    if 'scale' in flask.request.args:
        try:
            scale_width = int(
                flask.request.args['width']) if 'width' in flask.request.args else None
            scale_height = int(
                flask.request.args['height']) if 'height' in flask.request.args else None
        except ValueError:
            return flask.make_response('width or height malformed', 404)

        if scale_width is None and scale_height is None:
            return flask.make_response('width and height missing', 404)

        if scale_width is None:
            scale_width = int(scale_height / image.shape[0] * image.shape[1])
        if scale_height is None:
            scale_height = int(scale_width / image.shape[1] * image.shape[0])

        size = np.array([[scale_width],
                         [scale_height]])

        image = img.resize(image, size)

    response = flask.make_response(img.encode(image, 'png'))
    response.headers['Content-Type'] = 'image/png'
    return response


@app.route('/annotation/<int:index>')
def annotation(index):
    if index < 0:
        return flask.make_response('index cannot be negative', 404)
    if index >= len(app.config['sample_offsets']):
        return flask.make_response('index cannot be larger than amount of images', 404)

    sample_offset = app.config['sample_offsets'][index]

    with app.config['input_samples_file_lock']:
        input_samples_file = app.config['arguments']['input_samples_file']
        input_samples_file.seek(sample_offset[0])
        data_bytes = input_samples_file.read(sample_offset[1])
        computed_masked_crc32_of_data = masked_crc32c(data_bytes)
        masked_crc32_of_data_bytes = input_samples_file.read(4)
        masked_crc32_of_data = struct.unpack(
            '<L', masked_crc32_of_data_bytes)[0]

    if masked_crc32_of_data != computed_masked_crc32_of_data:
        raise RuntimeError(
            f'CRC integrity check failed for data in input samples file at {sample_offset[0]}')

    example = viewer.example_pb2.Example()
    example.ParseFromString(data_bytes)

    is_positive_feature = example.features.feature['isPositive']
    assert is_positive_feature.HasField('int64_list')
    is_positive = is_positive_feature.int64_list.value[0]
    circle_feature = example.features.feature['circle']
    assert circle_feature.HasField('float_list')
    circle = circle_feature.float_list.value
    data_shape_feature = example.features.feature['dataShape']
    assert data_shape_feature.HasField('int64_list')
    data_shape = data_shape_feature.int64_list.value

    return flask.jsonify({
        'isPositive': is_positive,
        'circle': {
            'centerX': circle[0],
            'centerY': circle[1],
            'radius': circle[2],
        },
        'dataShape': {
            'width': data_shape[1],
            'height': data_shape[0],
        }
    })


@click.command()
@click.option('--debug', is_flag=True, help='Run server in debug/development mode which enables hot reloading of the application', show_default=True)
@click.option('--host', default='localhost', help='Hostname to listen on, set this to \'0.0.0.0\' to have the server available externally as well', show_default=True)
@click.option('--port', default=5000, help='Port of the webserver')
@click.option('--grid-image-size', default=200, help='Size of scaled images in grid', show_default=True)
@click.option('--color-space', type=click.Choice(['YCbCr', 'RGB', 'Grayscale'], case_sensitive=False), default='YCbCr', help='Color space of raw images', show_default=True)
@click.option('--default-gray', default=128, help='Default gray (uint8 Y component in [0,255])', show_default=True)
@click.argument('input_samples_file', type=click.File('rb'))
def server(*args, **kwargs):
    app.config['input_samples_file_lock'] = threading.Lock()
    app.config['arguments'] = kwargs

    input_samples_file = app.config['arguments']['input_samples_file']

    # get file size
    input_samples_file.seek(0, io.SEEK_END)
    size = input_samples_file.tell()
    input_samples_file.seek(0, io.SEEK_SET)

    position = 0
    offsets = []
    while position < size:
        input_samples_file.seek(position)

        # read and validate length
        length_bytes = input_samples_file.read(8)
        length = struct.unpack('<Q', length_bytes)[0]
        computed_masked_crc32_of_length = masked_crc32c(length_bytes)
        masked_crc32_of_length_bytes = input_samples_file.read(4)
        masked_crc32_of_length = struct.unpack(
            '<L', masked_crc32_of_length_bytes)[0]

        if masked_crc32_of_length != computed_masked_crc32_of_length:
            raise RuntimeError(
                f'CRC integrity check failed for length in input samples file at {position}')

        # validate next position (last position must equal size)
        if position + length + 16 > size:
            raise RuntimeError(f'Data truncated in input samples file')

        # store position
        offsets.append((position + 12, length))

        # update position
        #   length + CRC(length) + data[length] + CRC(data[length])
        #   8      + 4           + length       + 4                 = length + 16
        position += length + 16

    app.config['sample_offsets'] = offsets

    app.run(debug=app.config['arguments']['debug'], host=app.config['arguments']
            ['host'], port=app.config['arguments']['port'])
