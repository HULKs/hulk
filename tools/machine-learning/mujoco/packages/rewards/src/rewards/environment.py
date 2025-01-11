import numpy as np
from nao_interface import Nao


def impact_forces(nao: Nao) -> float:
    return -np.square(nao.data.cfrc_ext).sum()
