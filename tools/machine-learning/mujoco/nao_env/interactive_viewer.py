import os

import mujoco
from mujoco import viewer

os.environ["MUJOCO_GL"] = "egl"

with open("scene.xml", "r") as f:
    xml = f.read()

model = mujoco.MjModel.from_xml_string(xml)
data = mujoco.MjData(model)

# mujoco.mj_resetDataKeyframe(model, data, 2)

viewer.launch(model, data)
# viewer.launch()
