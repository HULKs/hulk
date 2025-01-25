from absl.testing import absltest

class MemoryLeakTest(absltest.TestCase):
    def test_deepcopy_mjdata_leak(self) -> None: ...
