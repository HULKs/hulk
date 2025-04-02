class RingBuffer[T]:
    data: list[T]
    index: int = 0

    def __init__(self, size: int, initial_value: T) -> None:
        self.data = [initial_value] * size

    def push(self, value: T) -> None:
        self.index = (self.index + 1) % len(self.data)
        self.data[self.index] = value

    def right(self) -> T:
        return self.data[self.index]

    def left(self) -> T:
        return self.data[(self.index + 1) % len(self.data)]
