"""Launcher tests use temporary files and mock children; never start remote jobs."""
import importlib.machinery
import importlib.util
import json
from pathlib import Path
import signal
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

loader = importlib.machinery.SourceFileLoader(
    "ball_filter_launcher", str(Path(__file__).with_name("ball_filter_optimization"))
)
spec = importlib.util.spec_from_loader(loader.name, loader)
launcher = importlib.util.module_from_spec(spec)
loader.exec_module(launcher)


class BallFilterLauncherTests(unittest.TestCase):
    def arguments(self, mode, output, *extra):
        return launcher.parser().parse_args([mode, "--output", str(output), *map(str, extra)])

    def test_local_arguments_preserve_literal_paths_and_default_round_size(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "literal $(no shell) run"
            args = self.arguments("local", output)
            children = Mock()
            with patch.object(launcher, "log"):
                launcher.run(args, children)
            command = children.run.call_args.args[0]
            self.assertEqual(command, [launcher.REPOSITORY / "simulator", "--tune-ball-filter",
                                      output / "local", "--tuning-trials", "256"])
            children.start.assert_not_called()
            children.monitor.assert_not_called()

    def test_existing_output_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            marker = output / "keep.txt"
            marker.write_text("existing run")
            children = Mock()
            with self.assertRaisesRegex(ValueError, "output already exists"):
                launcher.run(self.arguments("local", output), children)
            self.assertEqual(marker.read_text(), "existing run")
            children.run.assert_not_called()

    def test_connect_only_bridges_supplied_manifests_and_previews(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifests = [root / "one.json", root / "two with spaces.json"]
            for manifest in manifests:
                manifest.write_text("{}")
            output = root / "monitor-session"
            children = Mock()
            with patch.object(launcher, "remote_worker_slots") as capacity, patch.object(launcher, "log"):
                launcher.run(self.arguments("connect", output,
                    "--manifest", manifests[0], "--manifest", manifests[1]), children)
            capacity.assert_not_called()
            children.run.assert_not_called()
            bridge = children.start.call_args_list[0].args[0]
            preview = children.start.call_args_list[1].args[0]
            self.assertEqual(bridge[2:], ["bridge", *manifests, "--output", output / "monitor"])
            self.assertEqual(preview[1:], ["--remote-ball-tuning", output / "monitor/remote-progress.json",
                                          "--remote-tuning-output", output / "preview"])
            children.monitor.assert_called_once()

    def test_capacity_rejects_another_32_workers_and_bounds_auth_wait(self):
        helper = Mock()
        helper.ssh.return_value.stdout = json.dumps(dict(cpus=32, active_tuners=32,
                                                        available_bytes=100 * 1024**3, load=32))
        with patch.object(launcher, "remote_helper", return_value=helper):
            with self.assertRaisesRegex(ValueError, "Use connect"):
                launcher.remote_worker_slots("remote-compiler", 32)
        self.assertEqual(helper.ssh.call_args.kwargs["timeout"], 30)
        helper.capacity.assert_not_called()

    def test_capacity_accounts_for_active_workers_and_available_memory(self):
        helper = Mock()
        helper.ssh.return_value.stdout = json.dumps(dict(cpus=32, active_tuners=24,
                                                        available_bytes=100 * 1024**3, load=24))
        helper.capacity.return_value = 32
        with patch.object(launcher, "remote_helper", return_value=helper), patch.object(launcher, "log"):
            self.assertEqual(launcher.remote_worker_slots("remote-compiler", 32), 8)
            helper.capacity.return_value = 3
            self.assertEqual(launcher.remote_worker_slots("remote-compiler", 32), 3)

    def test_capacity_auth_timeout_explains_no_search_was_submitted(self):
        helper = Mock()
        helper.ssh.side_effect = subprocess.TimeoutExpired("ssh", 30)
        with patch.object(launcher, "remote_helper", return_value=helper):
            with self.assertRaisesRegex(ValueError, "no new search was submitted"):
                launcher.remote_worker_slots("remote-compiler", 32)

    def test_no_capture_or_directory_created_when_remote_capacity_is_full(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "new"
            children = Mock()
            with patch.object(launcher, "remote_worker_slots", side_effect=ValueError("Use connect")):
                with self.assertRaisesRegex(ValueError, "Use connect"):
                    launcher.run(self.arguments("remote", output), children)
            self.assertFalse(output.exists())
            children.run.assert_not_called()
            children.start.assert_not_called()

    def test_remote_capture_then_start_then_bridge_without_local_search(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "new"
            children = Mock()
            def complete_stage(command, _label, **_kwargs):
                if "start" in command:
                    remote = output / "remote"
                    remote.mkdir()
                    (remote / "manifest.json").write_text("{}")
            children.run.side_effect = complete_stage
            with patch.object(launcher, "remote_worker_slots", side_effect=[8, 4]) as capacity, \
                 patch.object(launcher, "log"):
                launcher.run(self.arguments("remote", output), children)
            stages = children.run.call_args_list
            self.assertEqual(stages[0].args[0][1:], ["--capture-ball-tuning", output / "recordings"])
            remote = stages[1].args[0]
            self.assertEqual(remote[remote.index("--workers") + 1], "4")
            self.assertIn("--full-cpu", remote)
            self.assertEqual(stages[1].kwargs["timeout"], 1800)
            self.assertEqual(capacity.call_args_list[1].args, ("remote-compiler", 8))
            self.assertNotIn("--tune-ball-filter", str(children.mock_calls))

    def test_connect_requires_manifests_and_rejects_capture_arguments(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "new"
            with self.assertRaisesRegex(ValueError, "requires at least one"):
                launcher.validate(self.arguments("connect", output))
            manifest = Path(directory) / "manifest.json"
            manifest.write_text("{}")
            with self.assertRaisesRegex(ValueError, "do not pass --recordings"):
                launcher.validate(self.arguments("connect", output, "--manifest", manifest,
                                                  "--recordings", directory))

    def test_concurrent_panel_launches_share_one_lock(self):
        with tempfile.TemporaryDirectory() as directory, \
             patch.object(launcher.Path, "home", return_value=Path(directory)), \
             patch.object(launcher, "ensure_router_available"):
            with launcher.launcher_lock():
                with self.assertRaisesRegex(ValueError, "Another ball-filter launcher"):
                    with launcher.launcher_lock():
                        self.fail("second launcher acquired the lock")
            with launcher.launcher_lock():
                pass

    def test_cleanup_targets_only_owned_local_groups_and_reaps_them(self):
        children = launcher.Children()
        processes = [Mock(pid=71001), Mock(pid=71002)]
        children.running = [(process, "local child") for process in processes]
        live = {process.pid for process in processes}
        def kill_group(pid, signum):
            self.assertIn(pid, {71001, 71002})
            if pid not in live:
                raise ProcessLookupError()
            if signum == signal.SIGINT:
                live.remove(pid)
        with patch.object(launcher.os, "killpg", side_effect=kill_group) as kill:
            children.close()
        for process in processes:
            process.wait.assert_called_once_with(timeout=1)
        self.assertEqual([call.args for call in kill.call_args_list if call.args[1] == signal.SIGINT],
                         [(71001, signal.SIGINT), (71002, signal.SIGINT)])

    def test_child_failure_is_actionable_and_never_calls_remote_stop(self):
        children = launcher.Children()
        child = Mock()
        child.poll.return_value = 2
        child.returncode = 2
        children.running = [(child, "Remote progress bridge")]
        with self.assertRaisesRegex(RuntimeError, "Remote workers remain running"):
            children.monitor()

    def test_startup_timeout_is_actionable(self):
        children = launcher.Children()
        child = Mock()
        child.wait.side_effect = subprocess.TimeoutExpired("ssh", 300)
        with patch.object(children, "start", return_value=child):
            with self.assertRaisesRegex(RuntimeError, "check SSH authentication"):
                children.run(["placeholder"], "Remote start", timeout=300)


if __name__ == "__main__":
    unittest.main()
