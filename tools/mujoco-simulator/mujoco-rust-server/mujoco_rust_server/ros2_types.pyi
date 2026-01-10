class Image:
    def __new__(
        cls, time: float, rgb: bytes, height: int, width: int
    ) -> Image: ...

class CameraInfo:
    def __new__(
        cls,
        time: float,
        height: int,
        width: int,
        focal_length_x: float,
        focal_length_y: float,
        optical_center_x: float,
        optical_center_y: float,
    ) -> CameraInfo: ...
