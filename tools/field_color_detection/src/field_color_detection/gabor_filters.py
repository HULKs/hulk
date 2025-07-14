import cv2
import numpy as np
from numpy.typing import NDArray

from .data import check_memory


class GaborFilter:
    def __init__(
        self,
        ksize: int,
        sigma: int,
        lambd: float,
        gamma: float,
        phi: float,
        theta: float = 0,
        orientations: int = 1,
    ) -> None:
        self.ksize = ksize
        self.sigma = sigma
        self.lambd = lambd
        self.gamma = gamma
        self.phi = phi
        self.theta = theta
        self.orientations = orientations

    def get_kernels(self) -> NDArray[np.float32]:
        angles = (
            [self.theta]
            if self.orientations == 1
            else np.linspace(0, np.pi, self.orientations, endpoint=False)
        )
        return np.array(
            [
                cv2.getGaborKernel(
                    (self.ksize, self.ksize),
                    self.sigma,
                    theta,
                    self.lambd,
                    self.gamma,
                    self.phi,
                    ktype=cv2.CV_32F,
                )
                for theta in angles
            ]
        )

    def apply_filter(
        self, image: NDArray[np.uint8], kernels: NDArray[np.float32]
    ) -> NDArray[np.uint8]:
        check_memory(min_available_gb=5)
        M, N = image.shape[:2]
        feature_maps = np.zeros((M, N, self.orientations), dtype=np.uint8)

        for i, kernel in enumerate(kernels):
            filtered_image = cv2.filter2D(image, cv2.CV_32F, kernel)
            min_val = filtered_image.min()
            max_val = filtered_image.max()
            if max_val > min_val:
                feature_maps[..., i] = (
                    (filtered_image - min_val) / (max_val - min_val) * 255
                ).astype(np.uint8)

        return feature_maps

    def get_features(
        self, images: NDArray[np.uint8], color_channel: int
    ) -> NDArray[np.uint8]:
        L, M, N = images.shape[:3]
        kernels = self.get_kernels()
        texture_feature_vector = np.zeros(
            (L, M, N, self.orientations), dtype=np.uint8
        )
        for i, image in enumerate(images):
            texture_feature_vector[i] = self.apply_filter(
                image[:, :, color_channel], kernels
            )

        return texture_feature_vector.reshape((-1, self.orientations)).astype(
            np.uint8
        )
