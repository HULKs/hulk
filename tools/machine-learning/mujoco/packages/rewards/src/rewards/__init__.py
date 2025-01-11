import numpy as np
from nao_interface import Nao


def ctrl_amplitude(nao: Nao) -> float:
    return np.square(nao.data.ctrl).sum()


def impact_forces(nao: Nao) -> float:
    return np.square(nao.data.cfrc_ext).sum()


def head_height(nao: Nao) -> float:
    head_center_id = nao.model.site_name2id("head_center")
    return nao.data.site_xpos[head_center_id][2]
