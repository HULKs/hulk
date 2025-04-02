import os

import mujoco
from mujoco import viewer
from nao_interface import Nao
from nao_interface.poses import PENALIZED_POSE


os.environ["MUJOCO_GL"] = "egl"

model = mujoco.MjModel.from_xml_path("model/scene.xml")
data = mujoco.MjData(model)

nao = Nao(model, data)
nao.reset(PENALIZED_POSE)

viewer.launch(model, data)
