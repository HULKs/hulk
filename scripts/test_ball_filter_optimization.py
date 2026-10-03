"""Launcher tests use temporary files and mock children; never start remote jobs."""
import importlib.machinery
import importlib.util
import json
from pathlib import Path
import signal
import subprocess
import tempfile
import time
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

    def test_refresh_defaults_to_five_minutes_and_can_be_disabled_or_overridden(self):
        self.assertEqual(self.arguments("remote", "unused").refresh_minutes, 5.0)
        self.assertEqual(self.arguments("remote", "unused", "--refresh-minutes", "0").refresh_minutes, 0.0)
        self.assertEqual(self.arguments("remote", "unused", "--refresh-minutes", "2.5").refresh_minutes, 2.5)

    def test_local_arguments_preserve_literal_paths_and_default_round_size(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "literal $(no shell) run"
            args = self.arguments("local", output)
            children = Mock()
            with patch.object(launcher, "log"):
                launcher.run(args, children)
            command = children.run.call_args.args[0]
            self.assertEqual(command, [launcher.REPOSITORY / "simulator", "--tune-ball-filter",
                                      output / "local", "--tuning-trials", "256", *launcher.simulator_scenario(args)])
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
                                          "--remote-tuning-output", output / "preview", "--tuning-opponents", "2", "--tuning-opponent-width", "0.44", "--tuning-walking-speed-scale", "1.0"])
            children.monitor.assert_called_once()

    def test_walking_speed_is_human_only_and_forwarded_to_local_simulator(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.arguments("local", Path(directory) / "speed", "--walking-speed-scale", "2.5")
            children = Mock()
            with patch.object(launcher, "log"):
                launcher.run(args, children)
            command = children.run.call_args.args[0]
            self.assertEqual(command[command.index("--tuning-walking-speed-scale") + 1], "2.5")

    def test_legacy_live_scenario_preserves_requested_walking_scale(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            args = self.arguments("connect", root / "new", "--walking-speed-scale", "2.0")
            scenario = root / "scenario.json"
            scenario.write_text(json.dumps(dict(count=2, width=0.44)))
            selected = launcher.read_live_scenario(args, dict(output=str(root)))
            self.assertEqual(selected["walking_speed_scale"], 2.0)
            scenario.write_text(json.dumps(dict(count=2, width=0.44, walking_speed_scale=float('nan'))))
            with self.assertRaisesRegex(ValueError, "walking speed"):
                launcher.read_live_scenario(args, dict(output=str(root)))

    def test_walking_speed_rejects_nonfinite_and_out_of_range_before_launch(self):
        for scale in ["nan", "inf", "0", "3.1"]:
            with tempfile.TemporaryDirectory() as directory:
                args = self.arguments("local", Path(directory) / "speed", "--walking-speed-scale", scale)
                children = Mock()
                with self.assertRaisesRegex(ValueError, "walking-speed-scale"):
                    launcher.run(args, children)
                children.run.assert_not_called()

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
                    (remote / "manifest.json").write_text(json.dumps(dict(run="owned", controller_protocol=1,
                        control_token="a" * 32, workers=4, host="remote-compiler")))
            children.run.side_effect = complete_stage
            with patch.object(launcher, "remote_worker_slots", side_effect=[8, 4]) as capacity, \
                 patch.object(launcher, "log"):
                launcher.run(self.arguments("remote", output, "--refresh-minutes", "0"), children)
            stages = children.run.call_args_list
            self.assertEqual(stages[0].args[0][1:], ["--capture-ball-tuning", output / "recordings", "--tuning-opponents", "2", "--tuning-opponent-width", "0.44", "--tuning-walking-speed-scale", "1.0"])
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


class RefreshGenerationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.args = launcher.parser().parse_args(["remote", "--output", str(self.root)])
        self.old_manifest = self.root / "remote/manifest.json"
        self.old_manifest.parent.mkdir()
        self.old_manifest.write_text(json.dumps(dict(run="old-owned", host="remote-compiler", workers=4,
                                                      controller_protocol=1, control_token="a" * 32)))
        monitor = self.root / "monitor"
        monitor.mkdir()
        (monitor / "bridge-state.json").write_text(json.dumps(dict(identity={"runs": ["old-owned"]}, provenance={"verified": True})))
        (monitor / "remote-progress.json").write_text(json.dumps(dict(
            updated_unix_seconds=time.time(), error=None,
            search={"best_parameters": {"hypothesis_timeout": {"secs": 20, "nanos": 0}, "gain": 1.5}},
            remote={"best_candidate": "old-owned/worker-0/round-10"})))
        preview = self.root / "preview"
        preview.mkdir()
        (preview / "scenario.json").write_text(json.dumps({"count": 3, "width": 0.7, "walking_speed_scale": 2.0}))
        self.session = dict(mode="remote", pid=123, output=str(self.root), recordings="/original/recordings",
                            manifests=[str(self.old_manifest)], generations=[dict(index=0, root=str(self.root), state="active")],
                            monitor=str(monitor), preview=str(preview), workers=4, active_generation=0)
        self.children = Mock()
        self.events = []
        self.children.close.side_effect = lambda: self.events.append("close-local")
        self.children.start.side_effect = lambda command, label: self.events.append(("local", command, label))
        self.children.run.side_effect = self.command
        status = patch.object(launcher, "remote_generation_status", return_value={"state": "ready", "stop_requested": False})
        self.remote_status = status.start()
        self.addCleanup(status.stop)

    def command(self, command, label, **kwargs):
        self.events.append(("run", command, label))
        if "--capture-ball-tuning" in command:
            self.assertFalse(any(isinstance(event, tuple) and "stop" in event[1] for event in self.events))
        if "start" in command:
            self.assertIn("--paused", command)
            self.assertIn("--initial-parameters", command)
            directory = Path(command[command.index("--local-directory") + 1])
            directory.mkdir()
            (directory / "manifest.json").write_text(json.dumps(dict(run="new-owned", host="remote-compiler", workers=4,
                                                                     controller_protocol=1, control_token="b" * 32)))

    def test_refresh_freezes_best_varies_seed_and_only_swaps_after_replacement_is_ready(self):
        with patch.object(launcher, "remote_worker_slots", return_value=4), patch.object(launcher, "log"):
            launcher.refresh_generation(self.args, self.children, self.session)
        commands = [event[1] for event in self.events if isinstance(event, tuple) and event[0] == "run"]
        capture, start, ready, stop, activate = commands
        self.assertIn("--capture-ball-parameters", capture)
        self.assertEqual(capture[capture.index("--capture-ball-seed-offset") + 1], "1000000")
        self.assertEqual(capture[capture.index("--tuning-opponents") + 1], "3")
        self.assertEqual(capture[capture.index("--tuning-opponent-width") + 1], "0.7")
        self.assertEqual(capture[capture.index("--tuning-walking-speed-scale") + 1], "2.0")
        self.assertIn("wait-ready", ready)
        self.assertEqual(stop[2:4], ["stop", self.old_manifest])
        self.assertIn("--wait", stop)
        self.assertIn("activate", activate)
        baseline = Path(capture[capture.index("--capture-ball-parameters") + 1])
        self.assertEqual(json.loads(baseline.read_text())["gain"], 1.5)
        self.assertEqual(self.session["active_generation"], 1)
        self.assertNotEqual(self.session["manifests"], [str(self.old_manifest)])
        self.assertEqual(self.session["generations"][1]["state"], "active")
        self.assertIn("generation-0001/monitor", self.session["monitor"])
        self.assertEqual(json.loads((self.root / "session.json").read_text())["manifests"], self.session["manifests"])

    def test_failed_capture_restores_old_preview_without_stopping_remote_workers(self):
        def failed_capture(command, label, **kwargs):
            self.command(command, label, **kwargs)
            raise RuntimeError("capture failed")
        self.children.run.side_effect = failed_capture
        with patch.object(launcher, "log"), self.assertRaisesRegex(RuntimeError, "capture failed"):
            launcher.refresh_generation(self.args, self.children, self.session)
        self.assertEqual(self.session["manifests"], [str(self.old_manifest)])
        self.assertEqual(self.session["generations"][1]["state"], "capture_failed")
        commands = [event[1] for event in self.events if isinstance(event, tuple)]
        self.assertFalse(any("stop" in command or "start" in command for command in commands))
        self.assertTrue(any("--remote-ball-tuning" in command for command in commands))

    def test_failed_upload_preserves_old_search_and_completed_new_capture(self):
        def failed_upload(command, label, **kwargs):
            self.command(command, label, **kwargs)
            if "start" in command:
                raise RuntimeError("upload failed")
        self.children.run.side_effect = failed_upload
        with patch.object(launcher, "log"), self.assertRaisesRegex(RuntimeError, "upload failed"):
            launcher.refresh_generation(self.args, self.children, self.session)
        self.assertEqual(self.session["generations"][1]["state"], "prepare_failed")
        self.assertEqual(self.session["manifests"], [str(self.old_manifest)])
        self.assertFalse(any(isinstance(event, tuple) and "stop" in event[1] for event in self.events))
        self.assertTrue((self.root / "generations/generation-0001/baseline.json5").exists())

    def test_stale_snapshot_defers_before_interrupting_live_preview(self):
        path = Path(self.session["monitor"]) / "remote-progress.json"
        value = json.loads(path.read_text())
        value["updated_unix_seconds"] -= 16
        path.write_text(json.dumps(value))
        with self.assertRaisesRegex(ValueError, "fresh"):
            launcher.refresh_generation(self.args, self.children, self.session)
        self.children.close.assert_not_called()
        self.children.run.assert_not_called()

    def test_activation_failure_retries_prepared_generation_without_recapturing(self):
        original = self.command
        def fail_activate(command, label, **kwargs):
            original(command, label, **kwargs)
            if "activate" in command:
                raise RuntimeError("temporary SSH failure")
        self.children.run.side_effect = fail_activate
        with patch.object(launcher, "remote_worker_slots", return_value=4), patch.object(launcher, "log"):
            with self.assertRaisesRegex(RuntimeError, "temporary SSH failure"):
                launcher.refresh_generation(self.args, self.children, self.session)
            self.assertEqual(self.session["generations"][1]["state"], "old_stopped")
            self.events.clear()
            self.children.run.side_effect = original
            launcher.refresh_generation(self.args, self.children, self.session)
        commands = [event[1] for event in self.events if isinstance(event, tuple) and event[0] == "run"]
        self.assertEqual(len(commands), 1)
        self.assertIn("activate", commands[0])
        self.assertEqual(len(self.session["generations"]), 2)

    def test_lost_activation_ack_adopts_already_running_owned_workers_without_capacity_probe(self):
        def lose_ack(command, label, **kwargs):
            self.command(command, label, **kwargs)
            if "activate" in command:
                raise RuntimeError("activation acknowledgement lost")
        self.children.run.side_effect = lose_ack
        with patch.object(launcher, "remote_worker_slots", return_value=4) as capacity, patch.object(launcher, "log"):
            with self.assertRaisesRegex(RuntimeError, "acknowledgement"):
                launcher.refresh_generation(self.args, self.children, self.session)
            self.remote_status.return_value = {"state": "searching", "stop_requested": False}
            self.children.run.reset_mock()
            capacity.reset_mock()
            launcher.refresh_generation(self.args, self.children, self.session)
            capacity.assert_not_called()
            self.children.run.assert_not_called()
        self.assertEqual(self.session["active_generation"], 1)

    def test_supervisor_does_not_restart_preview_when_stale_result_defers_refresh(self):
        self.children.monitor.side_effect = [None, launcher.StopRequested(signal.SIGINT)]
        with patch.object(launcher, "refresh_generation", side_effect=ValueError("stale result")), \
             patch.object(launcher, "log"), self.assertRaises(launcher.StopRequested):
            launcher.supervise_remote(self.args, self.children, self.session)
        self.children.close.assert_not_called()
        self.children.start.assert_not_called()

    def test_resume_adopts_only_protocol_owned_run_and_starts_no_remote_job(self):
        output = self.root / "adopted"
        args = launcher.parser().parse_args(["remote", "--output", str(output), "--resume-manifest",
                                            str(self.old_manifest), "--refresh-minutes", "0"])
        with patch.object(launcher, "remote_worker_slots") as capacity, patch.object(launcher, "log"):
            launcher.run(args, self.children)
        capacity.assert_not_called()
        self.children.run.assert_not_called()
        session = json.loads((output / "session.json").read_text())
        self.assertEqual(session["manifests"], [str(self.old_manifest)])
        self.assertEqual(session["workers"], 4)


if __name__ == "__main__":
    unittest.main()
