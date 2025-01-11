from nao_interface import Nao


def maximum_head_height(nao: Nao) -> float:
    head_center_id = nao.model.site_name2id("head_center")
    head_height = nao.data.site_xpos[head_center_id][2]
    return head_height
