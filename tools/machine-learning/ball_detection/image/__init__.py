import imghdr
import struct
import typing
import cv2
import numpy as np


def size(path: str):
    # https://stackoverflow.com/a/20380514
    with open(path, 'rb') as f:
        head = f.read(24)
        if len(head) != 24:
            raise RuntimeError(
                f'Failed to get size of {path}: len(head) != 24')
        if imghdr.what(path) == 'png':
            check = struct.unpack('>i', head[4:8])[0]
            if check != 0x0d0a1a0a:
                raise RuntimeError(
                    f'Failed to get size of {path}: check != 0x0d0a1a0a')
            width, height = struct.unpack('>ii', head[16:24])
        elif imghdr.what(path) == 'gif':
            width, height = struct.unpack('<HH', head[6:10])
        elif imghdr.what(path) == 'jpeg':
            try:
                f.seek(0)  # Read 0xff next
                size = 2
                filetype = 0
                while not 0xc0 <= filetype <= 0xcf:
                    f.seek(size, 1)
                    byte = f.read(1)
                    while ord(byte) == 0xff:
                        byte = f.read(1)
                    filetype = ord(byte)
                    size = struct.unpack('>H', f.read(2))[0] - 2
                # We are at a SOFn block
                f.seek(1, 1)  # Skip `precision' byte.
                height, width = struct.unpack('>HH', f.read(4))
            except Exception:  # IGNORE:W0703
                raise RuntimeError(
                    f'Failed to get size of {path}: except Exception')
        else:
            raise RuntimeError(
                f'Failed to get size of {path}: imghdr.what(image) is not \'png\', \'gif\', nor \'jpeg\'')
        return width, height


def read(path: str):
    image = cv2.imread(path)
    if image is None:
        raise RuntimeError(f'Failed to read {path}')
    return image


def convert_color_space(image: np.ndarray, source_color_space: str, target_color_space: str):
    '''source_color_space and target_color_space in ['YCbCr', 'RGB', 'Grayscale'] or ['YCrCb', 'BGR', 'Grayscale']'''
    if source_color_space == 'YCbCr':
        if target_color_space == 'YCbCr':
            return image[:, :, [2, 0, 1]]  # convert to YCrCb
        elif target_color_space == 'RGB':
            return cv2.cvtColor(image[:, :, [2, 0, 1]], cv2.COLOR_YCrCb2BGR)
        elif target_color_space == 'Grayscale':
            return cv2.cvtColor(cv2.cvtColor(image[:, :, [2, 0, 1]], cv2.COLOR_YCrCb2BGR), cv2.COLOR_BGR2GRAY)
        else:
            raise RuntimeError(
                f'Unexpected target color space {target_color_space}')
    elif source_color_space == 'RGB':
        if target_color_space == 'YCbCr':
            return cv2.cvtColor(image, cv2.COLOR_BGR2YCrCb)
        elif target_color_space == 'RGB':
            return image
        elif target_color_space == 'Grayscale':
            return cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
        else:
            raise RuntimeError(
                f'Unexpected target color space {target_color_space}')
    elif source_color_space == 'Grayscale':
        if target_color_space == 'YCbCr':
            return cv2.cvtColor(image, cv2.COLOR_BGR2YCrCb)
        elif target_color_space == 'Grayscale' or target_color_space == 'RGB':
            return image
        else:
            raise RuntimeError(
                f'Unexpected target color space {target_color_space}')
    else:
        raise RuntimeError(
            f'Unexpected source color space {source_color_space}')


def crop(image: np.ndarray, upper_left: np.ndarray, lower_right: np.ndarray, default_color: typing.List[int]):
    '''upper_left and lower_right with shape (2, 1), default_color with 3 items'''
    upper_left = np.rint(upper_left).astype(int)
    lower_right = np.rint(lower_right).astype(int)

    image_size = np.array([[image.shape[1]],
                           [image.shape[0]]])
    border_top_left = np.maximum(0, -upper_left)
    border_bottom_right = np.maximum(0, lower_right - image_size)

    # if cropping out of image: extend image with default color
    if border_top_left[0, 0] > 0 or border_top_left[1, 0] > 0 or border_bottom_right[0, 0] > 0 or border_bottom_right[1, 0] > 0:
        upper_left += border_top_left
        lower_right += border_top_left
        image = cv2.copyMakeBorder(
            image,
            left=border_top_left[0, 0],
            top=border_top_left[1, 0],
            right=border_bottom_right[0, 0],
            bottom=border_bottom_right[1, 0],
            borderType=cv2.BORDER_CONSTANT,
            value=default_color,
        )

    # actually crop image
    return image[upper_left[1, 0]:lower_right[1, 0],
                 upper_left[0, 0]:lower_right[0, 0]]


def resize(image: np.ndarray, size: np.ndarray, interpolation=cv2.INTER_NEAREST):
    '''size with shape (2, 1)'''
    return cv2.resize(image, (size[0, 0], size[1, 0]), interpolation=interpolation)


def encode(image: np.ndarray, type: str):
    '''type in e.g. ['png', 'jpeg', ...]'''
    # type is prefixed with '.' to form a file extension
    success, buffer = cv2.imencode(f'.{type}', image)
    if not success:
        raise RuntimeError(f'Failed to encode image')
    return buffer.tobytes()
