import numpy as np

from .ring_buffer import RingBuffer


def test_initialization() -> None:
    buffer = RingBuffer(1, np.array([1, 2, 3]))
    assert np.all(buffer.data == np.array([[1, 2, 3]]))


def test_buffer_initialization() -> None:
    buffer = RingBuffer(2, np.array([1, 2, 3]))
    assert np.all(buffer.data == np.array([[1, 2, 3], [1, 2, 3]]))


def test_push() -> None:
    buffer = RingBuffer(2, np.array([1, 2, 3]))
    buffer.push(np.array([4, 5, 6]))
    assert np.all(buffer.data == np.array([[1, 2, 3], [4, 5, 6]]))


def test_push_overflow() -> None:
    buffer = RingBuffer(2, np.array([1, 2, 3]))
    buffer.push(np.array([4, 5, 6]))
    buffer.push(np.array([7, 8, 9]))
    assert np.all(buffer.data == np.array([[7, 8, 9], [4, 5, 6]]))


def test_right() -> None:
    buffer = RingBuffer(2, np.array([1, 2, 3]))
    buffer.push(np.array([4, 5, 6]))
    assert np.all(buffer.right() == np.array([4, 5, 6]))


def test_left() -> None:
    buffer = RingBuffer(2, np.array([1, 2, 3]))
    buffer.push(np.array([4, 5, 6]))
    assert np.all(buffer.left() == np.array([1, 2, 3]))


def test_right_with_overflow() -> None:
    buffer = RingBuffer(2, np.array([1, 2, 3]))
    buffer.push(np.array([4, 5, 6]))
    buffer.push(np.array([7, 8, 9]))
    assert np.all(buffer.right() == np.array([7, 8, 9]))


def test_left_with_overflow() -> None:
    buffer = RingBuffer(2, np.array([1, 2, 3]))
    buffer.push(np.array([4, 5, 6]))
    buffer.push(np.array([7, 8, 9]))
    assert np.all(buffer.left() == np.array([4, 5, 6]))
