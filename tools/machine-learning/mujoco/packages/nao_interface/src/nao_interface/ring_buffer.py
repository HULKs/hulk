import numpy as np
from numpy.typing import NDArray


class RingBuffer:
    data: NDArray
    index: int = 0

    def __init__(self, size: int, initial_value: NDArray) -> None:
        self.data = np.broadcast_to(
            initial_value, (size, initial_value.shape[0])
        ).copy()

    def push(self, value: NDArray) -> None:
        self.index = (self.index + 1) % self.data.shape[0]
        self.data[self.index] = value

    def right(self) -> NDArray:
        return self.data[self.index]

    def left(self) -> NDArray:
        return self.data[(self.index + 1) % self.data.shape[0]]
