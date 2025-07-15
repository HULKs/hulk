import numpy as np
from joblib import Parallel, delayed
from numpy.typing import NDArray
from skimage.feature import local_binary_pattern

from .data import check_memory
from .settings import MAX_N_JOBS


class LBPFilter:
    def __init__(self, radius: int) -> None:
        self.radius = radius
        self.n_points = 8  # * radius

    def apply_filter(self, image: NDArray[np.uint8]) -> NDArray[np.uint8]:
        check_memory(min_available_gb=5)
        return local_binary_pattern(
            image, self.n_points, self.radius, method="uniform"
        ).astype(np.uint8)

    def get_features(
        self, images: NDArray[np.uint8], color_channel: int
    ) -> NDArray[np.uint8]:
        feature_vector = Parallel(n_jobs=MAX_N_JOBS, backend="threading")(
            delayed(self.apply_filter)(img[:, :, color_channel])
            for img in images
        )
        return np.array(feature_vector).reshape(-1, 1).astype(np.uint8)
