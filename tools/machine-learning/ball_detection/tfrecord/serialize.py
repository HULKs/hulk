'''https://www.tensorflow.org/tutorials/load_data/tfrecord#tfrecords_format_details'''

import crc32c
import struct
import typing
from . import example_pb2


def masked_crc32c(data: bytes):
    checksum = crc32c.crc32(data)
    return ((((checksum >> 15) & 0xffffffff) | ((checksum << 17) & 0xffffffff)) + 0xa282ead8) & 0xffffffff


def serialize(is_positive=False, circle: typing.Tuple[float, float, float] = (0, 0, 0), data_shape: typing.Tuple[int, int, int] = (0, 0, 0), data: typing.Iterable[int] = []):
    example = example_pb2.Example()

    # isPositive: [isPositive]
    example.features.feature['isPositive'].int64_list.value.append(
        1 if is_positive else 0)

    # circle: [x, y, radius]
    example.features.feature['circle'].float_list.value.append(circle[0])
    example.features.feature['circle'].float_list.value.append(circle[1])
    example.features.feature['circle'].float_list.value.append(circle[2])

    # dataShape: [height, width, components]
    example.features.feature['dataShape'].int64_list.value.append(
        data_shape[0])
    example.features.feature['dataShape'].int64_list.value.append(
        data_shape[1])
    example.features.feature['dataShape'].int64_list.value.append(
        data_shape[2])

    # data: cropped image as [pixel data]
    example.features.feature['data'].int64_list.value[:] = data

    # serialize example and generate TFRecord metadata
    data_bytes = bytes(example.SerializeToString())
    masked_crc32_of_data = masked_crc32c(data_bytes)
    masked_crc32_of_data_bytes = struct.pack('<L', masked_crc32_of_data)
    length = len(data_bytes)
    length_bytes = struct.pack('<Q', length)
    masked_crc32_of_length = masked_crc32c(length_bytes)
    masked_crc32_of_length_bytes = struct.pack('<L', masked_crc32_of_length)

    # construct TFRecord
    return length_bytes + masked_crc32_of_length_bytes + data_bytes + masked_crc32_of_data_bytes
