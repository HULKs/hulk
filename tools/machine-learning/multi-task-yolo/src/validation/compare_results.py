import argparse
import json
import logging
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Literal, cast

logger = logging.getLogger(__name__)

TaskType = Literal["detect", "pose"]
TaskArg = Literal["auto", "detect", "pose"]

CONFIG_KEYS_TO_COMPARE = [
    "data",
    "split",
    "imgsz",
    "batch",
    "workers",
    "conf",
    "iou",
    "max_det",
    "half",
    "device",
]

DEFAULT_PRIMARY_METRICS: dict[TaskType, str] = {
    "detect": "metrics/mAP50-95(B)",
    "pose": "metrics/mAP50-95(P)",
}


class InvalidJsonObjectError(TypeError):
    def __init__(self, path: Path, data_type: type[Any]) -> None:
        super().__init__(f"Expected JSON object in '{path}', got {data_type!r}")


class RunDirectoryNotFoundError(FileNotFoundError):
    def __init__(self, run_dir: Path) -> None:
        super().__init__(f"Run directory not found: {run_dir}")


class RequiredFileNotFoundError(FileNotFoundError):
    def __init__(self, path: Path) -> None:
        super().__init__(f"Required file not found: {path}")


class NoNumericMetricsError(ValueError):
    def __init__(self, metrics_path: Path) -> None:
        super().__init__(f"No numeric metrics found in {metrics_path}")


class TaskInferenceError(ValueError):
    def __init__(self) -> None:
        super().__init__("Could not infer task from metrics keys")


class TaskMismatchError(ValueError):
    def __init__(
        self, baseline_task: TaskType, candidate_task: TaskType
    ) -> None:
        super().__init__(
            "Task mismatch between runs: "
            f"baseline={baseline_task}, candidate={candidate_task}"
        )


class StrictConfigMismatchError(ValueError):
    def __init__(self, mismatch_keys: list[str]) -> None:
        super().__init__(
            "Configuration mismatch in strict mode: "
            + ", ".join(sorted(mismatch_keys))
        )


@dataclass(frozen=True)
class ValidationRun:
    run_dir: Path
    metrics: dict[str, float]
    metadata: dict[str, Any]
    config: dict[str, Any]


def _load_json(path: Path) -> dict[str, Any]:
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise InvalidJsonObjectError(path, type(data))
    return data


def _load_run(run_dir: Path) -> ValidationRun:
    run_dir = run_dir.resolve()
    if not run_dir.is_dir():
        raise RunDirectoryNotFoundError(run_dir)

    metrics_path = run_dir / "metrics.json"
    metadata_path = run_dir / "metadata.json"
    config_path = run_dir / "config.json"

    for path in (metrics_path, metadata_path, config_path):
        if not path.is_file():
            raise RequiredFileNotFoundError(path)

    raw_metrics = _load_json(metrics_path)
    metrics: dict[str, float] = {}
    for k, v in raw_metrics.items():
        if isinstance(v, (int, float)):
            metrics[k] = float(v)

    if not metrics:
        raise NoNumericMetricsError(metrics_path)

    return ValidationRun(
        run_dir=run_dir,
        metrics=metrics,
        metadata=_load_json(metadata_path),
        config=_load_json(config_path),
    )


def _infer_task(metrics: dict[str, float]) -> TaskType:
    has_detect = any("(B)" in key for key in metrics)
    has_pose = any("(P)" in key for key in metrics)

    if has_detect and not has_pose:
        return "detect"
    if has_pose and not has_detect:
        return "pose"
    # Pose validation commonly includes both box (B) and pose (P) metrics,
    # so treat this case as pose.
    if has_detect and has_pose:
        return "pose"
    raise TaskInferenceError


def _task_from_metadata(metadata: dict[str, Any]) -> TaskType | None:
    raw_task = metadata.get("task")
    if not isinstance(raw_task, str):
        return None

    normalized = raw_task.strip().lower()
    if normalized in ("detect", "detection"):
        return "detect"
    if normalized == "pose":
        return "pose"
    return None


def _infer_task_from_run(run: ValidationRun) -> TaskType:
    metadata_task = _task_from_metadata(run.metadata)
    if metadata_task is not None:
        return metadata_task
    return _infer_task(run.metrics)


def _format_float(value: float | None, digits: int = 4) -> str:
    if value is None:
        return "n/a"
    return f"{value:.{digits}f}"


def _format_pct(value: float | None, digits: int = 2) -> str:
    if value is None:
        return "n/a"
    return f"{value:+.{digits}f}%"


def _compare_configs(
    baseline_cfg: dict[str, Any],
    candidate_cfg: dict[str, Any],
) -> tuple[list[str], dict[str, dict[str, Any]]]:
    matches: list[str] = []
    mismatches: dict[str, dict[str, Any]] = {}

    for key in CONFIG_KEYS_TO_COMPARE:
        b = baseline_cfg.get(key)
        c = candidate_cfg.get(key)
        if b == c:
            matches.append(key)
        else:
            mismatches[key] = {"baseline": b, "candidate": c}

    return matches, mismatches


def _build_metric_rows(
    baseline_metrics: dict[str, float],
    candidate_metrics: dict[str, float],
) -> tuple[list[dict[str, Any]], list[str], list[str]]:
    shared = sorted(set(baseline_metrics).intersection(candidate_metrics))
    missing_in_candidate = sorted(
        set(baseline_metrics).difference(candidate_metrics)
    )
    missing_in_baseline = sorted(
        set(candidate_metrics).difference(baseline_metrics)
    )

    rows: list[dict[str, Any]] = []
    for name in shared:
        base_value = baseline_metrics[name]
        cand_value = candidate_metrics[name]
        delta = cand_value - base_value
        delta_pct = None if base_value == 0 else (delta / base_value * 100.0)

        if abs(delta) < 1e-12:
            status = "unchanged"
        elif delta > 0:
            status = "improved"
        else:
            status = "regressed"

        rows.append(
            {
                "name": name,
                "baseline": base_value,
                "candidate": cand_value,
                "delta": delta,
                "delta_pct": delta_pct,
                "status": status,
            }
        )

    return rows, missing_in_candidate, missing_in_baseline


def _print_metric_table(rows: list[dict[str, Any]]) -> None:
    if not rows:
        print("No shared metrics to compare.")
        return

    headers = ["metric", "baseline", "candidate", "delta", "delta %", "status"]

    table_rows: list[list[str]] = []
    for row in rows:
        table_rows.append(
            [
                str(row["name"]),
                _format_float(row["baseline"]),
                _format_float(row["candidate"]),
                _format_float(row["delta"]),
                _format_pct(row["delta_pct"]),
                str(row["status"]),
            ]
        )

    widths = [len(h) for h in headers]
    for row in table_rows:
        for idx, cell in enumerate(row):
            widths[idx] = max(widths[idx], len(cell))

    def render(cols: list[str]) -> str:
        return " | ".join(c.ljust(widths[i]) for i, c in enumerate(cols))

    print(render(headers))
    print("-+-".join("-" * w for w in widths))
    for row in table_rows:
        print(render(row))


def compare_runs(
    baseline_run: ValidationRun,
    candidate_run: ValidationRun,
    task_arg: TaskArg,
    primary_metric: str | None,
    regression_threshold: float,
    *,
    strict_config: bool,
) -> dict[str, Any]:
    if task_arg == "auto":
        baseline_task = _infer_task_from_run(baseline_run)
        candidate_task = _infer_task_from_run(candidate_run)
        if baseline_task != candidate_task:
            raise TaskMismatchError(baseline_task, candidate_task)
        task: TaskType = baseline_task
    else:
        task = cast(TaskType, task_arg)

    matches, mismatches = _compare_configs(
        baseline_run.config,
        candidate_run.config,
    )

    if strict_config and mismatches:
        raise StrictConfigMismatchError(list(mismatches))

    metric_rows, missing_in_candidate, missing_in_baseline = _build_metric_rows(
        baseline_run.metrics,
        candidate_run.metrics,
    )

    primary_key = primary_metric or DEFAULT_PRIMARY_METRICS[task]
    primary_row = next(
        (r for r in metric_rows if r["name"] == primary_key), None
    )

    if primary_row is None:
        primary_result = {
            "name": primary_key,
            "status": "missing",
            "baseline": None,
            "candidate": None,
            "delta": None,
            "delta_pct": None,
            "regression_threshold": regression_threshold,
        }
    else:
        delta = float(primary_row["delta"])
        improvement_threshold = abs(regression_threshold)
        if delta < regression_threshold:
            verdict = "regressed"
        elif delta > improvement_threshold:
            verdict = "improved"
        else:
            verdict = "neutral"

        primary_result = {
            "name": primary_key,
            "status": verdict,
            "baseline": primary_row["baseline"],
            "candidate": primary_row["candidate"],
            "delta": primary_row["delta"],
            "delta_pct": primary_row["delta_pct"],
            "regression_threshold": regression_threshold,
        }

    improved = sum(1 for r in metric_rows if r["status"] == "improved")
    regressed = sum(1 for r in metric_rows if r["status"] == "regressed")
    unchanged = sum(1 for r in metric_rows if r["status"] == "unchanged")

    return {
        "timestamp": datetime.now(UTC).isoformat(),
        "baseline_dir": str(baseline_run.run_dir),
        "candidate_dir": str(candidate_run.run_dir),
        "task": task,
        "config_check": {
            "strict": strict_config,
            "keys_checked": CONFIG_KEYS_TO_COMPARE,
            "matches": matches,
            "mismatches": mismatches,
        },
        "primary_metric": primary_key,
        "primary_result": primary_result,
        "metrics": metric_rows,
        "missing": {
            "in_candidate": missing_in_candidate,
            "in_baseline": missing_in_baseline,
        },
        "summary": {
            "improved": improved,
            "regressed": regressed,
            "unchanged": unchanged,
            "shared_metrics": len(metric_rows),
        },
    }


def _print_cli_summary(report: dict[str, Any]) -> None:
    print("Comparison")
    print(f"- Baseline: {report['baseline_dir']}")
    print(f"- Candidate: {report['candidate_dir']}")
    print(f"- Task: {report['task']}")

    cfg = report["config_check"]
    print(
        "- Config mismatches: "
        f"{len(cfg['mismatches'])} / {len(cfg['keys_checked'])}"
    )
    if cfg["mismatches"]:
        print("- Mismatch keys: " + ", ".join(sorted(cfg["mismatches"])))

    print()
    _print_metric_table(report["metrics"])
    print()

    primary = report["primary_result"]
    print(
        "Primary metric"
        f" ({primary['name']}): {primary['status']}"
        f" | delta={_format_float(primary['delta'])}"
        f" | delta%={_format_pct(primary['delta_pct'])}"
    )

    summary = report["summary"]
    print(
        "Summary: "
        f"improved={summary['improved']}, "
        f"regressed={summary['regressed']}, "
        f"unchanged={summary['unchanged']}, "
        f"shared={summary['shared_metrics']}"
    )

    missing = report["missing"]
    if missing["in_candidate"]:
        print("Missing in candidate: " + ", ".join(missing["in_candidate"]))
    if missing["in_baseline"]:
        print("Missing in baseline: " + ", ".join(missing["in_baseline"]))


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compare two validation result directories",
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        required=True,
        help="Path to baseline run directory containing metrics.json",
    )
    parser.add_argument(
        "--candidate",
        type=Path,
        required=True,
        help="Path to candidate run directory containing metrics.json",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help=(
            "Path to output comparison JSON "
            "(default: <candidate>/comparison.json)"
        ),
    )
    parser.add_argument(
        "--task",
        choices=("auto", "detect", "pose"),
        default="auto",
        help="Task type to compare. Use auto to infer from metric keys",
    )
    parser.add_argument(
        "--strict-config",
        action="store_true",
        help="Fail comparison if selected config keys do not match",
    )
    parser.add_argument(
        "--primary-metric",
        default=None,
        help="Primary metric key used for verdict",
    )
    parser.add_argument(
        "--regression-threshold",
        type=float,
        default=-0.01,
        help="Absolute delta threshold for regression on primary metric",
    )
    args = parser.parse_args()

    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s | %(levelname)s | %(message)s",
    )

    baseline_run = _load_run(args.baseline)
    candidate_run = _load_run(args.candidate)

    report = compare_runs(
        baseline_run=baseline_run,
        candidate_run=candidate_run,
        task_arg=cast(TaskArg, args.task),
        primary_metric=args.primary_metric,
        regression_threshold=args.regression_threshold,
        strict_config=args.strict_config,
    )

    _print_cli_summary(report)

    output_path = args.output or (args.candidate / "comparison.json")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)
    logger.info("Saved comparison report to %s", output_path)


if __name__ == "__main__":
    main()
