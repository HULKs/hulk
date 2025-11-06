class RGBDSensors:
    def __new__(
        cls, time: float, rgb: bytes, depth: bytes, height: int, width: int
    ) -> RGBDSensors: ...
