import os

import numpy as np
from joblib import Parallel, delayed
from numpy.typing import NDArray
from skimage.feature import hog
from skimage.transform import resize

from image_segmenter import check_memory

# from image_segmenter.settings import MAX_N_JOBS

MAX_N_JOBS = min(os.cpu_count() // 2, 4)

class HoGFilter:
    def __init__(
        self,
        orientations: int,
        pixels_per_cell: tuple[int, int],
        cells_per_block: tuple[int, int],
    ) -> None:
        self.orientations = orientations
        self.pixels_per_cell = pixels_per_cell
        self.cells_per_block = cells_per_block

    def apply_filter(self, image: NDArray[np.uint8]) -> NDArray[np.float32]:
        return hog(
            image,
            orientations=self.orientations,
            pixels_per_cell=self.pixels_per_cell,
            cells_per_block=self.cells_per_block,
            feature_vector=False,
        )

    def _extract_and_resize(
        self,
        image: NDArray[np.uint8],
        color_channel: int,
        out_shape: tuple[int, int],
    ) -> NDArray[np.float32]:
        check_memory(min_available_gb=5)
        hog_features = self.apply_filter(image[:, :, color_channel])
        (
            blocks_per_row,
            blocks_per_col,
            cells_per_block_row,
            cells_per_block_col,
            n_orient,
        ) = hog_features.shape

        n_cells_row = blocks_per_row + cells_per_block_row - 1
        n_cells_col = blocks_per_col + cells_per_block_col - 1

        cell_feature_sums = np.zeros(
            (n_cells_row, n_cells_col, n_orient), dtype=np.float32
        )

        cell_counts = np.zeros((n_cells_row, n_cells_col, 1), dtype=np.float32)

        for row in range(blocks_per_row):
            for col in range(blocks_per_col):
                cell_feature_sums[
                    row : row + cells_per_block_row,
                    col : col + cells_per_block_col,
                    :,
                ] += hog_features[row, col]
                cell_counts[
                    row : row + cells_per_block_row,
                    col : col + cells_per_block_col,
                    :,
                ] += 1

        final_cell_features = np.divide(
            cell_feature_sums, cell_counts, where=cell_counts != 0
        )

        return resize(
            final_cell_features,
            (*out_shape, self.orientations),
            anti_aliasing=True,
            mode="reflect",
        ).astype(np.float32)

    def get_features(
        self, images: NDArray[np.uint8], color_channel: int
    ) -> NDArray[np.uint8]:
        L, M, N = images.shape[:3]
        features = Parallel(n_jobs=MAX_N_JOBS, backend="loky")(
            delayed(self._extract_and_resize)(img, color_channel, (M, N))
            for img in images
        )
        stacked = np.stack(features)
        return (stacked * 255).reshape(-1, self.orientations).astype(np.uint8)
