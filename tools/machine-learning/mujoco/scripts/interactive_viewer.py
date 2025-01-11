import os

import mujoco
from mujoco import viewer

os.environ["MUJOCO_GL"] = "egl"

model = mujoco.MjModel.from_xml_path("model/scene.xml")
data = mujoco.MjData(model)

viewer.launch(model, data)
