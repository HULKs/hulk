import mujoco
from mujoco.viewer import launch

model = mujoco.MjModel.from_xml_path("model/K1.xml")
data = mujoco.MjData(model)
model.opt.gravity[:] = 0.0

launch(model, data)
