import mujoco
import numpy as np
from numpy.typing import NDArray


def _calculate_initial_velocity(
    start: NDArray[np.floating],
    target: NDArray[np.floating],
    acceleration: NDArray[np.floating],
    time_to_reach: float,
) -> NDArray[np.floating]:
    displacement = target - start
    initial_velocity = (
        displacement - 0.5 * acceleration * time_to_reach**2
    ) / time_to_reach
    return initial_velocity


def _random_start_above_zero(
    target: NDArray[np.floating],
    radius: float,
    throwable_radius: float,
) -> NDArray[np.floating]:
    height = target[2]
    if height <= 0:
        raise ValueError(
            "cannot generate random start if target is not above floor"
        )
    min_elevation = np.atan2(throwable_radius - height, radius)
    max_elevation = np.deg2rad(60)
    elevation = np.random.uniform(min_elevation, max_elevation)
    azimuth = np.random.uniform(0, 2 * np.pi)

    return target + radius * np.array(
        [
            np.sin(elevation) * np.cos(azimuth),
            np.sin(elevation) * np.sin(azimuth),
            np.cos(elevation),
        ]
    )


class ThrowableObject:
    def __init__(
        self,
        model: mujoco.MjModel,
        data: mujoco.MjData,
        plane_body: str,
        throwable_body: str,
    ):
        self.model = model
        self.data = data
        self.ground_index = mujoco.mj_name2id(
            model, mujoco.mjtObj.mjOBJ_BODY, plane_body
        )
        self.throwable_index = mujoco.mj_name2id(
            model, mujoco.mjtObj.mjOBJ_BODY, throwable_body
        )

    def throw_has_ended(self) -> bool:
        """
        Check if the throw has ended by detecting contact between the throwable object and the ground.
        Returns:
            bool: True if the throw has ended (i.e., the throwable object has made contact with the ground), False otherwise.
        """
        geoms = (
            self.model.body_geomadr[self.throwable_index],
            self.model.body_geomadr[self.ground_index],
        )
        for i in range(self.data.ncon):
            contact = self.data.contact[i]
            if (contact.geom1, contact.geom2) == geoms or (
                contact.geom2,
                contact.geom1,
            ) == geoms:
                return True
        return False

    def random_throw(
        self,
        target: NDArray[np.floating],
        *,
        time_to_reach: float,
        distance: float,
    ):
        throwable_radius = self.model.geom_rbound[
            self.model.body_geomadr[self.throwable_index]
        ]
        qpos_index = self.model.jnt_qposadr[
            self.model.body_jntadr[self.throwable_index]
        ]
        throwable_qpos = self.data.qpos[qpos_index : qpos_index + 7]

        qvel_index = self.model.jnt_dofadr[
            self.model.body_jntadr[self.throwable_index]
        ]
        throwable_qvel = self.data.qvel[qvel_index : qvel_index + 6]

        start = _random_start_above_zero(target, distance, throwable_radius)
        throwable_qpos[:3] = start

        initial_velocity = _calculate_initial_velocity(
            start, target, self.model.opt.gravity, time_to_reach
        )
        throwable_qvel[:3] = initial_velocity
