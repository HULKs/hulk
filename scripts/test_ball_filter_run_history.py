"""History tests use only temporary directories and fake process/SSH results."""
import importlib.machinery
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

loader = importlib.machinery.SourceFileLoader("ball_history", str(Path(__file__).with_name("ball_filter_run_history")))
spec = importlib.util.spec_from_loader(loader.name, loader)
history = importlib.util.module_from_spec(spec)
loader.exec_module(history)


def report(training=1.0, validation=2.0):
    return dict(reference_frame="field", training_recordings=["data/train.mcap"],
                optimized_parameters={"maximum_matching_cost": .25}, replay_matches_live=True, trials=256,
                training=dict(baseline=dict(loss=10.0), optimized=dict(loss=training, missing_seconds=2.0)),
                validation=dict(baseline=dict(loss=11.0), optimized=dict(loss=validation)))


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value))


def manifest(name="example"):
    return dict(run=name, host="test-host", remote=f"{history.TASK_BASE}/runs/{name}", trials=256, workers=1)


class HistoryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.repository = self.root / "repository"
        self.logs = self.repository / "logs"
        self.logs.mkdir(parents=True)
        self.proc = self.root / "proc"
        self.proc.mkdir()

    def tearDown(self):
        self.temporary.cleanup()

    def process(self, pid, cwd, arguments):
        directory = self.proc / str(pid)
        directory.mkdir()
        (directory / "cwd").symlink_to(cwd)
        (directory / "cmdline").write_bytes(b"\0".join(str(arg).encode() for arg in arguments) + b"\0")

    def local(self, name="local-example"):
        root = self.logs / name
        write(root / "optimized/report.json", report())
        return root

    def test_best_report_is_selected_by_training_not_holdout_and_rounds_not_duplicated(self):
        root = self.local()
        write(root / "round-0001/report.json", report(.2, 9.0))
        write(root / "round-0002/report.json", report(.3, .01))
        result = history.list_runs(self.repository, proc=self.proc)
        row = result["runs"][0]
        self.assertEqual(row["metrics"]["training_optimized"]["loss"], .2)
        self.assertEqual(row["metrics"]["validation_optimized"]["loss"], 9.0)
        self.assertEqual(row["completed_trials"], 512)
        self.assertEqual(len(result["runs"]), 1)

    def test_monitor_only_and_fetched_manifests_are_not_independent_searches(self):
        write(self.logs / "monitor-only/monitor/remote-progress.json", {"search": {}})
        write(self.logs / "fetched-2020/manifest.json", manifest())
        write(self.logs / "fetched-2020/results/report.json", report())
        self.assertEqual(history.discover(self.repository), ({}, {}))

    def test_launcher_generation_manifests_are_discovered_and_duplicates_deduplicated(self):
        write(self.logs / "session/remote/manifest.json", manifest())
        write(self.logs / "session/generations/generation-0001/remote/manifest.json", manifest("new"))
        write(self.logs / "remote-ball-tuning-copy/manifest.json", manifest())
        _, remote = history.discover(self.repository)
        self.assertEqual(len(remote), 2)
        self.assertEqual(sorted(len(item["manifest_paths"]) for item in remote.values()), [1, 2])

    def test_active_local_reader_blocks_deletion_even_when_only_using_input_file(self):
        root = self.local()
        self.process(100, self.repository, ["tuner", "--recordings=" + str(root / "optimized/report.json")])
        result = history.list_runs(self.repository, proc=self.proc)
        row = result["runs"][0]
        self.assertFalse(row["can_delete"])
        with self.assertRaisesRegex(ValueError, "active"):
            history.delete_run(row["id"], self.repository, proc=self.proc)
        self.assertTrue(root.is_dir())

    def test_code_payloads_are_not_interpreted_as_file_paths(self):
        root = self.local()
        self.process(321, self.repository, ["python3", "-c", "x" * 100000, "{bad argument}" * 2000])
        self.assertFalse(history.process_activity(root, self.proc))

    def test_unreadable_desktop_cwd_does_not_hide_optimizer_unknown_status(self):
        root = self.local()
        self.process(321, self.repository, ["/usr/bin/ssh-agent"])
        with patch.object(history.os, "readlink", side_effect=PermissionError("protected cwd")):
            self.assertFalse(history.process_activity(root, self.proc))
        (self.proc / "321/cmdline").write_bytes(b"python3\0controller.py\0")
        with patch.object(history.os, "readlink", side_effect=PermissionError("protected cwd")):
            self.assertIsNone(history.process_activity(root, self.proc))

    def test_unknown_process_activity_blocks_local_deletion(self):
        root = self.local()
        with patch.object(history, "process_activity", return_value=None):
            row = history.list_runs(self.repository, proc=self.proc)["runs"][0]
            self.assertFalse(row["can_delete"])
            with self.assertRaisesRegex(ValueError, "determine"):
                history.delete_run(row["id"], self.repository, proc=self.proc)
        self.assertTrue(root.exists())

    def test_unshared_captures_move_to_trash_but_shared_capture_references_block(self):
        root = self.local()
        capture = root / "recordings/train.mcap"
        capture.parent.mkdir()
        capture.write_bytes(b"capture")
        other = self.local("other")
        data = report()
        data["training_recordings"] = [str(capture)]
        write(other / "optimized/report.json", data)
        rows = history.list_runs(self.repository, proc=self.proc)["runs"]
        row = next(row for row in rows if row["location"] == str(root))
        self.assertFalse(row["can_delete"])
        with self.assertRaisesRegex(ValueError, "referenced"):
            history.delete_run(row["id"], self.repository, proc=self.proc)
        write(other / "optimized/report.json", report())
        result = history.delete_run(row["id"], self.repository, proc=self.proc)
        self.assertTrue((Path(result["deleted"]["trash"]) / "recordings/train.mcap").is_file())

    def test_parent_launcher_blocks_nested_results_during_capture(self):
        root = self.local("session/local")
        write(root.parent / "session.json", {"pid": 123, "recordings": "elsewhere"})
        self.process(123, self.repository, ["launcher", "--output", root.parent])
        row = history.list_runs(self.repository, proc=self.proc)["runs"][0]
        self.assertFalse(row["can_delete"])
        self.assertIn("session", row["delete_reason"])

    def test_stopped_local_results_move_to_recoverable_trash_and_leave_history(self):
        root = self.local()
        row = history.list_runs(self.repository, proc=self.proc)["runs"][0]
        result = history.delete_run(row["id"], self.repository, proc=self.proc)
        self.assertFalse(root.exists())
        target = Path(result["deleted"]["trash"])
        self.assertTrue((target / "optimized/report.json").is_file())
        self.assertEqual(history.list_runs(self.repository, proc=self.proc)["runs"], [])

    def test_unlisted_id_traversal_and_symlink_are_rejected(self):
        root = self.local()
        history.list_runs(self.repository, proc=self.proc)
        for identity in ["../local-example", str(root), "local-" + "a" * 24]:
            with self.assertRaises(ValueError):
                history.delete_run(identity, self.repository, proc=self.proc)
        alias = self.logs / "alias"
        alias.symlink_to(root)
        with self.assertRaisesRegex(ValueError, "symlink"):
            history.safe_directory(alias, self.logs)
        with self.assertRaises(ValueError):
            history.safe_directory(self.logs / ".." / "elsewhere", self.logs)
        self.assertEqual(len(history.discover(self.repository)[0]), 1)

    def test_remote_ssh_failure_returns_persisted_metrics_but_disables_deletion(self):
        write(self.logs / "remote-ball-tuning-example/manifest.json", manifest())
        snapshot = dict(run="example", status="stopped", can_delete=True, delete_reason=None, date="2026-10-03",
                        metrics={"training_optimized": {"loss": .4}})
        ssh = Mock(return_value=Mock(stdout=json.dumps([snapshot])))
        first = history.list_runs(self.repository, ssh=ssh, proc=self.proc)
        self.assertEqual(ssh.call_args.kwargs["timeout"], 30)
        failed = Mock(side_effect=subprocess.TimeoutExpired("ssh", 30))
        second = history.list_runs(self.repository, ssh=failed, proc=self.proc)
        row = second["runs"][0]
        self.assertEqual(row["metrics"], first["runs"][0]["metrics"])
        self.assertTrue(row["stale"])
        self.assertFalse(row["can_delete"])
        with self.assertRaises(subprocess.TimeoutExpired):
            history.delete_run(row["id"], self.repository, ssh=failed, proc=self.proc)

    def remote(self):
        base = self.root / "runs"
        root = base / "example"
        write(root / "manifest.json", manifest())
        write(root / "status.json", dict(pid=123, state="stopped"))
        return base, root

    def test_remote_orphan_worker_blocks_trash_even_when_controller_is_dead(self):
        base, root = self.remote()
        self.process(999, self.repository, [root / "bin/ball-filter-tuner", "--output", root / "results/round-1"])
        with self.assertRaisesRegex(ValueError, "active or unknown"):
            history.remote_delete("example", base, self.proc)
        self.assertTrue(root.exists())

    def test_remote_unknown_controller_blocks_trash(self):
        base, root = self.remote()
        (root / "status.json").unlink()
        with self.assertRaisesRegex(ValueError, "active or unknown"):
            history.remote_delete("example", base, self.proc)

    def test_remote_trash_stays_in_dedicated_task_tree(self):
        base, root = self.remote()
        destination = Path(history.remote_delete("example", base, self.proc))
        self.assertEqual(destination.parent, base.parent / "trash")
        self.assertTrue((destination / "manifest.json").is_file())
        self.assertFalse(root.exists())
        for name in ["..", "../example", "/tmp/example"]:
            with self.assertRaises(ValueError):
                history.remote_delete(name, base, self.proc)

    def test_remote_best_pointer_cannot_escape_run_directory(self):
        base, root = self.remote()
        write(root / "best.json", dict(output="../../elsewhere"))
        row = history.remote_rows(["example"], base, self.proc)[0]
        self.assertFalse(row["can_delete"])
        self.assertIn("invalid remote best", row["error"])

    def test_embedded_remote_inspection_is_self_contained_valid_python(self):
        command = history.remote_command(["example"])
        self.assertEqual(command[:2], ["python3", "-c"])
        compile(command[2], "remote-inspection", "exec")


if __name__ == "__main__":
    unittest.main()
