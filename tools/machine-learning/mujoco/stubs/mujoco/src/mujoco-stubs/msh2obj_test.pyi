from absl.testing import absltest
from mujoco import msh2obj as msh2obj

class MshTest(absltest.TestCase):
    def test_obj_model_matches_msh_model(self) -> None: ...
