from absl.testing import absltest
from mujoco import minimize as minimize

class MinimizeTest(absltest.TestCase):
    def test_basic(self) -> None: ...
    def test_start_at_minimum(self) -> None: ...
    def test_jac_callback(self) -> None: ...
    def test_max_iter(self) -> None: ...
    def test_bounds(self) -> None: ...
    def test_bad_bounds(self) -> None: ...
    def test_iter_callback(self) -> None: ...
    def test_norm(self) -> None: ...
