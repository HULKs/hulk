from nao_interface import Nao


def head_height(nao: Nao) -> float:
    head_center_id = nao.model.site_name2id("head_center")
    return nao.data.site_xpos[head_center_id][2]
