from concurrent.futures import ThreadPoolExecutor

import numpy as np
from numba import njit, prange, set_num_threads
from numpy.typing import NDArray

from image_segmenter import check_memory

set_num_threads(2)

@njit(parallel=True)
def sample_neighbors(
    image: NDArray[np.uint8], dx: NDArray, dy: NDArray
) -> NDArray[np.uint8]:
    M, N = image.shape
    n_points = len(dx)
    feature_map = np.zeros((M, N, n_points), dtype=np.uint8)

    for i in prange(n_points):
        for y in range(M):
            for x in range(N):
                y_sample = min(max(round(y + dy[i]), 0), M - 1)
                x_sample = min(max(round(x + dx[i]), 0), N - 1)
                feature_map[y, x, i] = image[y_sample, x_sample]

    return feature_map


class NeighboringPixels:
    def __init__(self, radius: int, orientations: int) -> None:
        self.radius = radius
        self.orientations = orientations
        angles = np.linspace(0, 2 * np.pi, self.orientations, endpoint=False)
        self.dx = self.radius * np.cos(angles)
        self.dy = self.radius * np.sin(angles)

    def apply_filter(self, image: NDArray[np.uint8]) -> NDArray[np.uint8]:
        check_memory(min_available_gb=5)
        return sample_neighbors(image, self.dx, self.dy)

    def get_features(
        self, images: NDArray[np.uint8], color_channel: int
    ) -> NDArray[np.uint8]:
        with ThreadPoolExecutor(max_workers=8) as executor:
            results = list(
                executor.map(
                    lambda img: self.apply_filter(img[:, :, color_channel]),
                    images,
                )
            )

        feature_vector = np.stack(results)
        return feature_vector.reshape(-1, self.orientations).astype(np.uint8)
