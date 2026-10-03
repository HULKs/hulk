"""Focused offline tests; no SSH connections or compilation."""
import importlib.machinery
import importlib.util
import io
import json
from pathlib import Path
import shlex
import tarfile
import tempfile
import unittest
from unittest.mock import patch

loader = importlib.machinery.SourceFileLoader("remote_tuning", str(Path(__file__).with_name("remote_ball_filter_tuning")))
spec = importlib.util.spec_from_loader(loader.name, loader)
helper = importlib.util.module_from_spec(spec)
loader.exec_module(helper)


class RemoteTuningTests(unittest.TestCase):
    def test_resource_guard_leaves_headroom_and_limits_workers(self):
        resources = dict(cpus=32, load=2.0, available_bytes=64 * 1024**3)
        self.assertEqual(helper.capacity(resources, 4), 4)
        self.assertEqual(helper.capacity(resources, 32), 16)
        resources["available_bytes"] = 6 * 1024**3
        self.assertEqual(helper.capacity(resources, 8), 2)
        resources["available_bytes"] = 3 * 1024**3
        with self.assertRaises(ValueError):
            helper.capacity(resources, 1)

    def test_full_cpu_removes_cpu_headroom_but_preserves_memory_limit(self):
        resources = dict(cpus=32, load=32.0, available_bytes=70 * 1024**3)
        self.assertEqual(helper.capacity(resources, 32, full_cpu=True), 32)
        self.assertEqual(helper.capacity(resources, 24, full_cpu=True), 24)
        resources["available_bytes"] = 6 * 1024**3
        self.assertEqual(helper.capacity(resources, 32, full_cpu=True), 2)

    def test_remote_arguments_are_quoted_without_shell_evaluation(self):
        arguments = ["python3", "a path/worker.py", "literal $(touch /tmp/no) `date` 'quoted'"]
        command = helper.ssh_command("remote-compiler", arguments)
        self.assertEqual(shlex.split(command[-1]), arguments)
        self.assertIn("-oBatchMode=yes", command)
        self.assertIn("-oControlMaster=auto", command)
        self.assertIn("-oControlPersist=30m", command)
        self.assertTrue(any(item.startswith("-oControlPath=") for item in command))
        for host in ["-oProxyCommand=bad", "remote;echo bad"]:
            with self.assertRaises(ValueError):
                helper.ssh_command(host, arguments)
        for run in ["../shared", "/home/shared", "name;command"]:
            with self.assertRaises(ValueError):
                helper.checked_name(run)

    def test_tmux_launch_uses_new_named_session_and_quoted_shell_payload(self):
        arguments = ["controller", ".cache/run space $(touch NO)", "ball-filter-test-abcd1234"]
        with patch.object(helper.sys, "argv", arguments), \
             patch.object(helper.subprocess, "run") as run, \
             patch.object(helper.subprocess, "check_output", return_value="123\n"), \
             patch.object(helper.sys, "stdout", io.StringIO()):
            exec(helper.TMUX_LAUNCH, {})
        launch = run.call_args_list[0].args[0]
        self.assertEqual(launch[:5], ["tmux", "new-session", "-d", "-s", arguments[2]])
        shell = shlex.split(launch[-1])
        self.assertEqual(shell[:2], ["bash", "-lc"])
        root = Path.home() / arguments[1]
        self.assertIn(shlex.quote(str(root / "controller.py")), shell[2])
        self.assertIn(shlex.quote(str(root / "controller.log")), shell[2])
        option = run.call_args_list[1].args[0]
        self.assertIn("=" + arguments[2] + ":", option)
        self.assertNotIn("kill-session", str(run.call_args_list))

    def test_workers_have_disjoint_outputs_seeds_and_explicit_warmstart(self):
        manifest = dict(trials=256, seed=7, reference_frame="field",
                        reference_topic="simulation/ball_ground_truth_field", namespace="")
        root = Path("/home/user/a run")
        a, output_a = helper.tuner_command(root, manifest, 0, 1, None)
        b, output_b = helper.tuner_command(root, manifest, 1, 1, root / "prior.json5")
        self.assertNotEqual(output_a, output_b)
        self.assertNotEqual(a[a.index("--seed") + 1], b[b.index("--seed") + 1])
        self.assertEqual(a[0], str(root / "bin/ball-filter-tuner"))
        self.assertEqual(b[-2:], ["--initial-parameters", str(root / "prior.json5")])
        self.assertEqual(len(a[a.index("--train") + 1:a.index("--validation")]), 4)

    def test_selection_never_uses_holdout_and_requires_verified_guarded_report(self):
        report = dict(replay_matches_live=True, continuity_policy="training guard",
                      training={"optimized": {"loss": 1.5}}, validation={"optimized": {"loss": 0.01}})
        self.assertEqual(helper.training_loss(report), 1.5)
        report["validation"]["optimized"]["loss"] = 999
        self.assertEqual(helper.training_loss(report), 1.5)
        report["replay_matches_live"] = False
        with self.assertRaises(ValueError):
            helper.training_loss(report)

    def test_snapshot_contains_dirty_source_and_only_explicit_recording_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, data, local = (root / name for name in ["source", "data", "local"])
            for path in [source, data, local]:
                path.mkdir()
            (source / "Cargo.toml").write_text("dirty local manifest")
            for name in helper.DATA_FILES:
                (data / name).write_text(name)
            (data / "unrelated.mcap").write_text("not this run")
            manifest = {"run": "test"}
            with patch.object(helper, "source_paths", return_value=[Path("Cargo.toml")]):
                archive = helper.snapshot(source, data, None, local, manifest)
            with tarfile.open(archive) as bundle:
                self.assertEqual(bundle.extractfile("source/Cargo.toml").read(), b"dirty local manifest")
                self.assertNotIn("data/unrelated.mcap", bundle.getnames())
                self.assertIn("controller.py", bundle.getnames())
                embedded = json.load(bundle.extractfile("manifest.json"))
                self.assertEqual(embedded["source_sha256"], manifest["source_sha256"])
            self.assertEqual(manifest["snapshot_sha256"], helper.digest(archive))

    def test_export_includes_only_successfully_completed_rounds(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            complete = root / "results/worker-00/round-000001"
            partial = root / "results/worker-00/round-000002"
            for output in [complete, partial]:
                output.mkdir(parents=True)
                (output / "report.json").write_text("{}")
                (output / "ball_filter.json5").write_text("{}")
            (complete / "complete.json").write_text("{}")
            captured = io.BytesIO()
            fake_stdout = type("Output", (), {"buffer": captured})()
            with patch.object(helper, "__file__", str(root / "controller.py")), patch.object(helper.sys, "stdout", fake_stdout):
                helper.export_results()
            with tarfile.open(fileobj=io.BytesIO(captured.getvalue())) as bundle:
                self.assertIn("results/worker-00/round-000001/report.json", bundle.getnames())
                self.assertNotIn("results/worker-00/round-000002/report.json", bundle.getnames())


if __name__ == "__main__":
    unittest.main()
