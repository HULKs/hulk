"""Analyze replay audit logs. Requires host numpy/scipy; never changes robot settings.

Usage: python3 audit_relocalization.py reference.json replay.json replay.log
Output is JSON. Recorded poses outside the ten checkpoints are comparators,
not ground truth. Future-image checks use raw VO, not optimized replay motion.
"""

import bisect
import collections
import json
import re
import sys

import numpy as np
from scipy.spatial.transform import Rotation, Slerp


def stamp(value):
    value = value["since_origin"]
    return value["secs"] * 10**9 + value["nanos"]


def pose(value):
    result = np.eye(4)
    result[:3, :3] = Rotation.from_quat(value["rotation"]).as_matrix()
    result[:3, 3] = value["translation"]
    return result


def interpolate(a, b, fraction):
    result = np.eye(4)
    result[:3, :3] = Slerp([0, 1], Rotation.from_matrix([a[:3, :3], b[:3, :3]]))([fraction]).as_matrix()[0]
    result[:3, 3] = a[:3, 3] * (1 - fraction) + b[:3, 3] * fraction
    return result


def rmse(frame, field_pose):
    transform = pose(frame["robot_to_camera"]) @ np.linalg.inv(field_pose)
    points = np.array([a["field_point"] for a in frame["associations"]])
    if not len(points):
        return None
    camera = points @ transform[:3, :3].T + transform[:3, 3]
    if np.any(camera[:, 2] <= 0.01):
        return None
    intrinsic = frame["camera_intrinsic"]
    pixels = camera[:, :2] / camera[:, 2:] * intrinsic["focals"] + intrinsic["optical_center"]
    detections = np.array([a["detection"] for a in frame["associations"]])
    return float(np.sqrt(np.mean(np.sum((pixels - detections)**2, axis=1))))


def error(candidate, reference):
    delta = candidate[:3, 3] - reference[:3, 3]
    yaw = Rotation.from_matrix(candidate[:3, :3]).as_euler("xyz")[2] - Rotation.from_matrix(reference[:3, :3]).as_euler("xyz")[2]
    return {"xy_m": float(np.linalg.norm(delta[:2])), "height_m": float(delta[2]),
            "yaw_deg": float(np.rad2deg(np.arctan2(np.sin(yaw), np.cos(yaw))))}


def main(reference_path, replay_path, log_path):
    reference = json.load(open(reference_path))
    replay = json.load(open(replay_path))
    origin = reference["origin_ns"]
    references = sorted(reference["references"], key=lambda r: stamp(r["time"]))
    reference_times = [stamp(r["time"]) for r in references]
    frames = sorted(reference["frames"], key=lambda r: stamp(r["time"]))
    frame_times = [stamp(r["time"]) for r in frames]
    proposed = {stamp(f["time"]): f["inner"] for f, admitted in replay["audit_frames"] if admitted}

    def reference_at(t, epoch):
        i = bisect.bisect_left(reference_times, t)
        if i == len(references) or (i == 0 and reference_times[i] != t):
            return None
        a = references[i] if reference_times[i] == t else references[i-1]
        b = references[i]
        if a["epoch"] != epoch or b["epoch"] != epoch or a["generation"] != b["generation"]:
            return None
        if not a["robot_to_field"] or not b["robot_to_field"]:
            return None
        ta, tb = stamp(a["time"]), stamp(b["time"])
        if tb-ta > 200_000_000:
            return None
        return interpolate(pose(a["robot_to_field"]["pose"]), pose(b["robot_to_field"]["pose"]), (t-ta)/(tb-ta) if tb != ta else 0)

    # Integrate only contiguous raw body-frame VO edges; never bridge a VO reset/gap.
    vo = {}
    chain = -1
    last_time = last_epoch = None
    current = np.eye(4)
    for edge in sorted(reference["odometry"], key=lambda r: stamp(r["time"])):
        previous, t = stamp(edge["previous_time"]), stamp(edge["time"])
        if t <= previous:
            continue
        if previous != last_time or edge["epoch"] != last_epoch:
            chain += 1
            current = np.eye(4)
            vo[previous] = (chain, current.copy())
        current = current @ pose(edge["transform"])
        vo[t] = (chain, current.copy())
        last_time, last_epoch = t, edge["epoch"]
    vo_times = sorted(vo)

    def vo_at(t):
        i = bisect.bisect_left(vo_times, t)
        if i == len(vo_times) or (i == 0 and vo_times[i] != t):
            return None
        ta = vo_times[i] if vo_times[i] == t else vo_times[i-1]
        tb = vo_times[i]
        ca, a = vo[ta]
        cb, b = vo[tb]
        if ca != cb or tb-ta > 100_000_000:
            return None
        return ca, interpolate(a, b, (t-ta)/(tb-ta) if tb != ta else 0)

    attempts = {}
    trial_batch = []
    last_stage = None
    reasons = collections.Counter()
    for line in open(log_path):
        line = re.sub(r"\x1b\[[0-9;]*m", "", line)
        if "rolling back inconsistent" in line:
            reason = re.search(r"error=(.*?) visual_frames=", line)
            if reason:
                text = reason[1].split(":")[0]
                reasons[text] += 1
                for trial_time in trial_batch:
                    attempts[trial_time]["reason"] = text
        if "visual correction audit" not in line:
            continue
        match = re.search(r"time=Time \{ secs: (\d+), nanos: (\d+) \} stage=\"(\w+)\" source=(\w+) features=(\d+) rms_px=(?:Some\(([^)]+)\)|None) pose=Some\(Isometry \{ rotation: \[([^]]+)\], translation: \[([^]]+)\]", line)
        if not match:
            continue
        sec, ns, stage, source, count, rms, quat, translation = match.groups()
        t = int(sec)*10**9 + int(ns)
        record = attempts.setdefault(t, {"time": (t-origin)/1e9, "source": source, "features": int(count), "accepted": False})
        sample = {"rmse_px": float(rms) if rms else None, "pose": pose({"rotation": json.loads("["+quat+"]"), "translation": json.loads("["+translation+"]")})}
        if stage == "before":
            record.setdefault("before", sample)
        elif stage == "trial":
            record["trial"] = sample
            if last_stage != "trial":
                trial_batch = []
            trial_batch.append(t)
        elif stage == "accepted":
            record["accepted"] = True
            record["trial"] = sample
        last_stage = stage

    rows = []
    for t, record in sorted(attempts.items()):
        if "before" not in record or "trial" not in record or t not in proposed:
            continue
        frame = proposed[t]
        record["epoch"] = frame["epoch"]
        record["generation"] = frame["generation"]
        before, after = record["before"]["pose"], record["trial"]["pose"]
        revision = float(np.linalg.norm(after[:3, 3]-before[:3, 3]))
        rotation = float(np.rad2deg(Rotation.from_matrix(before[:3, :3].T @ after[:3, :3]).magnitude()))
        # Keep all attempts for counts; detailed comparisons target meaningful corrections.
        record["revision_m"], record["revision_deg"] = revision, rotation
        if revision < 0.25 and rotation < 5:
            continue
        ref = reference_at(t, frame["epoch"])
        if ref is not None:
            record["reference_before"] = error(before, ref)
            record["reference_after"] = error(after, ref)
            record["reference_rmse_same_features_px"] = rmse(frame, ref)
        nearby = bisect.bisect_left(frame_times, t)
        choices = [i for i in (nearby-1, nearby) if 0 <= i < len(frames)]
        if choices:
            i = min(choices, key=lambda i: abs(frame_times[i]-t))
            if abs(frame_times[i]-t) <= 30_000_000:
                original = frames[i]["inner"]["associations"]
                matches = []
                for association in frame["associations"]:
                    candidates = [o for o in original if np.linalg.norm(np.array(o["detection"])-association["detection"]) < 2]
                    if candidates:
                        matches.append(any(np.linalg.norm(np.array(o["field_point"])-association["field_point"]) < 0.01 for o in candidates))
                record["recorded_correspondence_agreement"] = {"agree": sum(matches), "compared": len(matches)}
        start_vo = vo_at(t)
        future = []
        if start_vo is not None:
            used = set()
            for horizon in [0.15, 0.3, 0.5]:
                target = t + int(horizon*1e9)
                i = bisect.bisect_left(frame_times, target)
                choices = [j for j in (i-1, i) if 0 <= j < len(frames)]
                if not choices:
                    continue
                i = min(choices, key=lambda j: abs(frame_times[j]-target))
                ft = frame_times[i]
                if i in used or abs(ft-target) > 80_000_000 or ft <= t or frames[i]["inner"]["epoch"] != frame["epoch"]:
                    continue
                used.add(i)
                end_vo = vo_at(ft)
                if end_vo is None or end_vo[0] != start_vo[0]:
                    continue
                delta = np.linalg.inv(start_vo[1]) @ end_vo[1]
                future_frame = frames[i]["inner"]
                future.append({"dt_s": (ft-t)/1e9, "features": len(future_frame["associations"]),
                               "before_rmse_px": rmse(future_frame, before @ delta),
                               "after_rmse_px": rmse(future_frame, after @ delta)})
        record["future_raw_vo_checks"] = future
        for stage in ("before", "trial"):
            record[stage]["pose"] = {"translation": record[stage]["pose"][:3, 3].tolist(),
                                     "rotation": Rotation.from_matrix(record[stage]["pose"][:3, :3]).as_quat().tolist()}
        rows.append(record)

    summary = {"passes": sum(bool(c.get("pass")) for c in replay["checkpoints"]),
               "checkpoints": replay["checkpoints"], "rejection_reasons": reasons,
               "accepted_frames": sum(r["accepted"] for r in attempts.values()),
               "large_accepted_corrections": sum(r["accepted"] for r in rows),
               "large_rejected_corrections": sum(not r["accepted"] for r in rows)}
    low_fit_rejected = [r for r in rows if not r["accepted"] and r["features"] >= 3 and r["trial"]["rmse_px"] is not None and r["trial"]["rmse_px"] <= 10]
    summary["low_fit_large_rejected"] = len(low_fit_rejected)
    rich = [r for r in rows if r["features"] >= 3 and r["generation"] > 0]
    for accepted, label in [(True, "accepted"), (False, "rejected")]:
        selected = [r for r in rich if r["accepted"] == accepted]
        checks = [r for r in selected if len(r["future_raw_vo_checks"]) >= 2]
        supported = [r for r in checks if all(c["after_rmse_px"] is not None and c["after_rmse_px"] <= 10
                     and (c["before_rmse_px"] is None or c["after_rmse_px"] < c["before_rmse_px"])
                     for c in r["future_raw_vo_checks"])]
        summary[label+"_rich_corrections"] = len(selected)
        summary[label+"_with_two_future_checks"] = len(checks)
        summary[label+"_supported_by_future_checks"] = len(supported)
    # Source-time fresh-output coverage, not wall-time accuracy or delivery availability.
    bounds = {}
    for entry in replay["lifecycle"]:
        epoch = entry["epoch"]
        bounds.setdefault(epoch, entry["time"])
    ordered_epochs = sorted(bounds, key=bounds.get)
    estimates = replay["audit_estimates"]
    last = max((stamp(e["time"])-origin)/1e9 for e, _ in estimates)
    coverage = {}
    for require_tracking, label in [(False, "field"), (True, "tracking")]:
        total = covered = 0.0
        for index, epoch in enumerate(ordered_epochs):
            lo = bounds[epoch]
            hi = bounds[ordered_epochs[index+1]] if index+1 < len(ordered_epochs) else last
            total += hi-lo
            right = lo
            for estimate, state in estimates:
                if estimate["epoch"] != epoch or not estimate["robot_to_field"] or (require_tracking and state != "Tracking"):
                    continue
                t = (stamp(estimate["time"])-origin)/1e9
                start, end = max(lo, t), min(hi, t+0.1)
                if end > max(start, right):
                    covered += end-max(start, right)
                right = max(right, end)
        coverage[label] = {"fresh_coverage_s": covered, "evaluated_s": total, "fraction": covered/total}
    summary["source_time_coverage_100ms"] = coverage
    losses = []
    lifecycle = replay["lifecycle"]
    for i, status in enumerate(lifecycle):
        if status["state"] != "LostTrack":
            continue
        next_status = lifecycle[i+1] if i+1 < len(lifecycle) else None
        end = next_status["time"] if next_status else last
        recovered = next_status is not None and next_status["epoch"] == status["epoch"] and next_status["state"] == "Tracking"
        losses.append({"from_s": status["time"], "duration_s": end-status["time"], "recovered": recovered})
    summary["lost_track_intervals"] = losses
    summary["lost_track_total_s"] = sum(loss["duration_s"] for loss in losses)
    print(json.dumps({"summary": summary, "corrections": rows}, indent=2, allow_nan=False))


def self_check():
    identity = {"translation": [0, 0, 0], "rotation": [0, 0, 0, 1]}
    frame = {"robot_to_camera": identity, "camera_intrinsic": {"focals": [100, 200], "optical_center": [320, 240]},
             "associations": [{"field_point": [1, 2, 10], "detection": [333, 284]}]}
    assert rmse(frame, np.eye(4)) == 5
    assert np.allclose(interpolate(np.eye(4), np.eye(4), 0.5), np.eye(4))
    half_turn = np.diag([-1., -1., 1., 1.])
    frame["associations"][0]["field_point"] = [-1, -2, 10]
    assert rmse(frame, half_turn) == 5  # Symmetric correspondences preserve the residual.
    frame["associations"][0]["field_point"][2] = -10
    assert rmse(frame, np.eye(4)) is None


if __name__ == "__main__":
    self_check()
    main(*sys.argv[1:])
