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
from unittest.mock import Mock, patch

loader = importlib.machinery.SourceFileLoader("remote_tuning", str(Path(__file__).with_name("remote_ball_filter_tuning")))
spec = importlib.util.spec_from_loader(loader.name, loader)
helper = importlib.util.module_from_spec(spec)
loader.exec_module(helper)


class RemoteTuningTests(unittest.TestCase):
    def test_measured_worker_budget_allows_32_without_removing_host_headroom(self):
        resources = dict(cpus=32, load=19.3, available_bytes=40 * 1024**3)
        self.assertEqual(helper.capacity(resources, 32, full_cpu=True), 19)
        self.assertEqual(helper.capacity(resources, 32, full_cpu=True, worker_memory_mib=1024), 32)
        resources["available_bytes"] = 3 * 1024**3
        self.assertEqual(helper.capacity(resources, 32, full_cpu=True, worker_memory_mib=1024), 1)
        resources["available_bytes"] -= 1
        with self.assertRaises(ValueError):
            helper.capacity(resources, 32, full_cpu=True, worker_memory_mib=1024)
        for invalid in [0, -1, 1.5]:
            with self.assertRaisesRegex(ValueError, "positive integer"):
                helper.capacity(resources, 32, worker_memory_mib=invalid)

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


def bridge_report(loss=1.5, validation_loss=2.0):
    metrics = {key: 0.0 for key in helper.METRIC_FIELDS}
    metrics["missing_runs"] = 0
    metrics["loss"] = 3.0
    return dict(
        replay_matches_live=True, continuity_policy="guard missing time", retention_policy="fixed retention",
        objective={"version": "test-v3"}, penalty_metres=2.0, namespace="", reference_frame="field",
        reference_topic="simulation/ball_ground_truth_field", tuned_parameter_pointers=["/gain"],
        baseline_parameters={"gain": 1.0, "noise": [0.1, 0.1]},
        optimized_parameters={"gain": loss, "noise": [0.1, 0.1]}, trials=256,
        training_recordings=helper.DATA_FILES[:4], validation_recordings=helper.DATA_FILES[4:6],
        training={"baseline": metrics, "optimized": dict(metrics, loss=loss)},
        validation={"baseline": metrics, "optimized": dict(metrics, loss=validation_loss)},
    )


class BridgeTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.manifests = []
        for index in range(2):
            manifest = dict(run=f"run-{index}", host="remote-compiler", workers=1, trials=256,
                            source_sha256=str(index) * 64,
                            data_sha256={name: "a" * 64 for name in helper.DATA_FILES},
                            namespace="", reference_frame="field", reference_topic="simulation/ball_ground_truth_field")
            self.manifests.append(manifest)
            root = self.root / manifest["run"]
            root.mkdir()
            (root / "manifest.json").write_text(json.dumps(manifest))
            (root / "environment.json").write_text(json.dumps({"binary_sha256": "b" * 64}))
            (root / "worker-00.json").write_text(json.dumps({"state": "searching", "round": 2}))
            self.checkpoint(index, 1, bridge_report(1.5 - index * 0.1, 0.1 if index == 0 else 99.0))
        self.empty = helper.empty_progress("remote-compiler")

    def checkpoint(self, run, number, report, complete=True):
        path = self.root / f"run-{run}" / "results/worker-00" / f"round-{number:06d}"
        path.mkdir(parents=True)
        (path / "report.json").write_text(json.dumps(report))
        (path / "ball_filter.json5").write_text(json.dumps(report["optimized_parameters"]))
        if complete:
            (path / "complete.json").write_text(json.dumps({"training_loss": report["training"]["optimized"]["loss"]}))
        return path

    def read(self):
        return helper.read_remote_runs([item["run"] for item in self.manifests], self.root)

    def combine(self, previous=None, provenance=None):
        return helper.combine_remote_progress(self.manifests, self.read(), previous or self.empty, provenance, 123.0)

    def test_completed_only_counts_and_training_only_selection_with_different_sources(self):
        partial = self.checkpoint(0, 2, bridge_report(0.01), complete=False)
        (partial / "report.json").write_text("{partial write")
        snapshot, provenance = self.combine()
        self.assertEqual(snapshot["remote"]["completed_trials"], 512)
        self.assertEqual(snapshot["search"]["trial"], 512)
        self.assertEqual(snapshot["search"]["trials"], 0)
        self.assertEqual(snapshot["search"]["best_trial"], 1)
        self.assertIn("/run-1/", snapshot["remote"]["best_candidate"])
        self.assertEqual(snapshot["search"]["validation_best"]["loss"], 99.0)
        self.assertEqual(provenance["binary_sha256"], "b" * 64)

    def test_changed_dataset_and_binary_are_rejected(self):
        run = self.root / "run-1"
        self.manifests[1]["data_sha256"]["train-42.mcap"] = "c" * 64
        (run / "manifest.json").write_text(json.dumps(self.manifests[1]))
        with self.assertRaisesRegex(ValueError, "different recording"):
            self.combine()
        self.manifests[1]["data_sha256"]["train-42.mcap"] = "a" * 64
        (run / "manifest.json").write_text(json.dumps(self.manifests[1]))
        (run / "environment.json").write_text(json.dumps({"binary_sha256": "c" * 64}))
        with self.assertRaisesRegex(ValueError, "different filter/scoring"):
            self.combine()

    def test_equal_candidates_do_not_advance_revision_but_better_training_does(self):
        first, provenance = self.combine()
        self.checkpoint(1, 2, bridge_report(1.4, 99.0))
        second, _ = self.combine(first, provenance)
        self.assertEqual(second["search"]["best_trial"], 1)
        self.assertEqual(second["remote"]["best_candidate"], first["remote"]["best_candidate"])
        self.assertEqual(second["search"]["trial"], 768)
        self.checkpoint(0, 2, bridge_report(1.3, 100.0))
        third, _ = self.combine(second, provenance)
        self.assertEqual(third["search"]["best_trial"], 2)
        self.assertEqual(third["search"]["validation_best"]["loss"], 100.0)

    def test_timeout_keeps_last_success_age_and_parameters(self):
        previous, provenance = self.combine()
        with patch.object(helper, "ssh", side_effect=helper.subprocess.TimeoutExpired("ssh", 30)) as request:
            failed, retained = helper.poll_bridge(self.manifests, previous, provenance, now=999.0)
        self.assertEqual(failed["updated_unix_seconds"], 123.0)
        self.assertEqual(failed["search"], previous["search"])
        self.assertEqual(failed["remote"], previous["remote"])
        self.assertEqual(retained, provenance)
        self.assertTrue(failed["error"])
        self.assertEqual(request.call_args.kwargs["timeout"], 30)

    def test_ssh_error_messages_never_include_uploaded_inspection_source(self):
        previous, provenance = self.combine()
        exceptions = [
            helper.subprocess.CalledProcessError(255, "NEVER_INCLUDE inspection source", stderr="publickey denied"),
            helper.subprocess.TimeoutExpired("NEVER_INCLUDE inspection source", 30),
        ]
        for error in exceptions:
            with patch.object(helper, "ssh", side_effect=error):
                failed, _ = helper.poll_bridge(self.manifests, previous, provenance)
            self.assertNotIn("NEVER_INCLUDE", failed["error"])
            self.assertLess(len(failed["error"]), 200)
            self.assertIn("SSH to remote-compiler", failed["error"])
        with patch.object(helper, "ssh", side_effect=exceptions[0]):
            failed, _ = helper.poll_bridge(self.manifests, previous, provenance)
        self.assertIn("publickey denied", failed["error"])

    def test_parameter_comparison_uses_native_float_but_exact_integer_values(self):
        self.assertTrue(helper.parameters_equal({"gain": 0.9599999785423279}, {"gain": 0.9599999785423278}))
        self.assertFalse(helper.parameters_equal({"gain": 0.96}, {"gain": 0.97}))
        self.assertFalse(helper.parameters_equal({"secs": 16777216}, {"secs": 16777217}))
        self.assertFalse(helper.parameters_equal({"secs": 20}, {"secs": 20.0}))
        path = self.root / "run-0/results/worker-00/round-000001/ball_filter.json5"
        config = json.loads(path.read_text())
        config["gain"] += 1e-10
        path.write_text(json.dumps(config))
        self.assertEqual(self.combine()[0]["remote"]["completed_trials"], 512)

    def test_malformed_completed_round_fails_instead_of_selecting_it(self):
        path = self.root / "run-0/results/worker-00/round-000001/report.json"
        path.write_text("{partial write")
        with self.assertRaises(ValueError):
            self.read()

    def test_dead_or_reused_controller_pid_cannot_report_searching(self):
        root = self.root / "run-0"
        (root / "status.json").write_text(json.dumps({"state": "searching", "pid": 999999999}))
        result = self.read()[0]
        self.assertEqual(result["workers"][0]["status"], "controller exited")
        fake_proc = self.root / "proc"
        (fake_proc / "123").mkdir(parents=True)
        command = fake_proc / "123/cmdline"
        command.write_bytes(b"python3\0unrelated.py\0_run\0")
        self.assertFalse(helper.controller_alive(root, {"pid": 123}, fake_proc))
        command.write_bytes(b"python3\0" + str(root / "controller.py").encode() + b"\0_run\0")
        self.assertTrue(helper.controller_alive(root, {"pid": 123}, fake_proc))

    def test_initial_pending_runs_still_publish_worker_status(self):
        for manifest in self.manifests:
            root = self.root / manifest["run"]
            helper.shutil.rmtree(root / "results")
            (root / "environment.json").unlink()
        snapshot, provenance = self.combine()
        self.assertIsNone(snapshot["search"])
        self.assertIsNone(provenance)
        self.assertEqual(len(snapshot["remote"]["workers"]), 2)
        self.assertEqual(snapshot["updated_unix_seconds"], 123.0)

    def test_inspection_source_is_cached_across_later_workspace_edits(self):
        import inspect
        first = helper.bridge_inspection_command(["run-0"])
        with patch.object(inspect, "getsource", side_effect=OSError("source file changed")):
            second = helper.bridge_inspection_command(["run-1"])
        self.assertEqual(first[2], second[2])
        self.assertEqual(second[3:], ["run-1"])
        compile(second[2], "cached inspection", "exec")

    def test_read_only_probe_is_self_contained_and_does_not_rewrite_controller(self):
        arguments = helper.bridge_inspection_command(["run-0", "run-1"])
        code = compile(arguments[2], "inspection", "exec")
        namespace = {"__name__": "__test__"}
        with patch.object(helper.sys, "argv", ["-c", "run-0", "run-1"]), \
             patch.object(helper.sys, "stdout", io.StringIO()) as output, \
             patch.object(Path, "home", return_value=self.root):
            # Relocate fixture directories to match the real per-user task root.
            base = self.root / helper.TASK_BASE / "runs"
            base.mkdir(parents=True)
            for manifest in self.manifests:
                (self.root / manifest["run"]).rename(base / manifest["run"])
            exec(code, namespace)
        result = json.loads(output.getvalue())
        self.assertEqual(len(result), 2)
        self.assertEqual(result[0]["completed_trials"], 256)


class CooperativeControllerTests(unittest.TestCase):
    def test_controls_refuse_legacy_and_wrong_ownership_without_touching_stop_file(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            root = home / helper.TASK_BASE / "runs/owned"
            root.mkdir(parents=True)
            manifest = dict(run="owned", host="remote-compiler", controller_protocol=1, control_token="a" * 32)
            path = root / "manifest.json"
            path.write_text(json.dumps(manifest))
            arguments = ["-c", str(root.relative_to(home)), "owned", "b" * 32, "stop"]
            with patch.object(Path, "home", return_value=home), patch.object(helper.sys, "argv", arguments):
                with self.assertRaisesRegex(ValueError, "ownership/protocol"):
                    exec(helper.CONTROL, {})
            self.assertFalse((root / "stop.requested").exists())
            arguments[3] = "a" * 32
            with patch.object(Path, "home", return_value=home), patch.object(helper.sys, "argv", arguments), \
                 patch.object(helper.sys, "stdout", io.StringIO()):
                exec(helper.CONTROL, {})
            self.assertTrue((root / "stop.requested").exists())
            manifest.pop("controller_protocol")
            path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, "monitor-only"):
                helper.controlled_manifest(path)

    def run_fixture(self, *, stop_while_paused):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        home = Path(temporary.name)
        root = home / "run"
        (root / "source").mkdir(parents=True)
        cache = home / helper.TASK_BASE
        (cache / "target/release").mkdir(parents=True)
        (cache / "target/release/ball-filter-tuner").write_text("fixture binary")
        manifest = dict(run="owned", workers=1, build_jobs=1, toolchain="test", tmux_session="owned",
                        start_paused=True, initial=False, seed=7, trials=2,
                        reference_frame="field", reference_topic="truth", namespace="")
        (root / "manifest.json").write_text(json.dumps(manifest))
        events = []
        def sleep(_seconds):
            self.assertEqual(json.loads((root / "status.json").read_text())["state"], "ready")
            self.assertNotIn("tuner", events)
            (root / ("stop.requested" if stop_while_paused else "activate.requested")).touch()
            events.append("stop-paused" if stop_while_paused else "activate")
        def run(command, **kwargs):
            if command[0] == "cargo":
                events.append("build")
                return Mock(returncode=0)
            self.assertIn("activate", events)
            events.append("tuner")
            output = Path(command[command.index("--output") + 1])
            output.mkdir()
            (output / "report.json").write_text(json.dumps(dict(replay_matches_live=True,
                continuity_policy="baseline", training={"optimized": {"loss": 1.0}})))
            (output / "ball_filter.json5").write_text("{}")
            # Request stop during the round. Its successful result must still be
            # committed before the worker exits, with no second round launched.
            (root / "stop.requested").touch()
            return Mock(returncode=0)
        with patch.object(helper, "__file__", str(root / "controller.py")), \
             patch.object(Path, "home", return_value=home), \
             patch.object(helper.subprocess, "run", side_effect=run), \
             patch.object(helper.subprocess, "check_output", return_value="rustc test"), \
             patch.object(helper.time, "sleep", side_effect=sleep), \
             patch.object(helper.sys, "stdout", io.StringIO()):
            helper.run_remote()
        return root, events

    def test_paused_build_starts_no_workers_until_activated_then_finishes_current_round(self):
        root, events = self.run_fixture(stop_while_paused=False)
        self.assertEqual(events, ["build", "activate", "tuner"])
        self.assertEqual(json.loads((root / "status.json").read_text())["state"], "stopped")
        self.assertEqual(json.loads((root / "worker-00.json").read_text())["state"], "stopped")
        self.assertTrue((root / "results/worker-00/round-000001/complete.json").is_file())
        self.assertFalse((root / "results/worker-00/round-000002").exists())
        self.assertEqual(json.loads((root / "best.json").read_text())["round"], 1)

    def test_paused_generation_can_stop_without_ever_spawning_a_tuner(self):
        root, events = self.run_fixture(stop_while_paused=True)
        self.assertEqual(events, ["build", "stop-paused"])
        self.assertEqual(json.loads((root / "status.json").read_text())["state"], "stopped")
        self.assertFalse((root / "results").exists())


if __name__ == "__main__":
    unittest.main()
