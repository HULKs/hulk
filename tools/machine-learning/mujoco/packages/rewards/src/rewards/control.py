import numpy as np
from nao_interface import Nao


def low_ctrl_amplitude(nao: Nao) -> float:
    return -np.square(nao.data.ctrl).sum()
