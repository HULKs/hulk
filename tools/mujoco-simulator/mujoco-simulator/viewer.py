from mujoco import MjData, MjModel
from mujoco.viewer import launch

model = MjModel.from_xml_path("K1/K1.xml")
# model.opt.gravity[:] = 0.0
data = MjData(model)

launch(model, data)
