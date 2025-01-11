import numpy as np
from nao_interface import Nao
from numpy.typing import NDArray


def ctrl_amplitude(nao: Nao) -> float:
    return np.square(nao.data.ctrl).sum()


def impact_forces(nao: Nao) -> float:
    return np.square(nao.data.cfrc_ext).sum()


def head_height(nao: Nao) -> float:
    return nao.data.site("head_center").xpos[2]


def head_z_error(nao: Nao, target: float) -> float:
    return np.square(head_height(nao) - target)


def action_rate(nao: Nao, last_ctrl: NDArray[np.floating]) -> float:
    return np.mean(np.square(nao.data.ctrl - last_ctrl))


def head_xy_error(nao: Nao, target: NDArray[np.floating]) -> float:
    head_center_xy = nao.data.site("head_center").xpos[:2]
    return np.mean(np.square(head_center_xy - target))
