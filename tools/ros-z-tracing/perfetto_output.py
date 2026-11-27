"""Convert a ROS-Z Chrome JSON trace into a Perfetto timeline grouped by node and topic."""

from __future__ import annotations

import argparse
import heapq
import json
import math
import os
import tempfile
from collections import defaultdict
from decimal import Decimal
from pathlib import Path
from typing import Any, BinaryIO
from urllib.parse import quote

from message_trace import TERMINALS, Metadata, Slice, Trace, correlate, label
from perfetto.protos.perfetto.trace.perfetto_trace_pb2 import (
    TrackDescriptor,
    TrackEvent,
)
from perfetto.trace_builder.proto_builder import StreamingTraceProtoBuilder


def owner(meta: Metadata) -> str | None:
    return (
        next(iter(meta.names)) if meta.names and not meta.conflicting else None
    )


def focus(trace: Trace, node: str) -> Trace:
    selected = {
        ep for ep, meta in trace.endpoints.items() if owner(meta) == node
    }
    # Context, not a new edge: e.g. two queues may observe a publication
    # whose put was outside the capture. Don't recursively expand peers.
    topics = defaultdict(set)
    for s in trace.slices:
        e = s.begin
        if e.endpoint in selected and e.publication:
            topics[e.endpoint[0], e.publication].update(
                trace.endpoints[e.endpoint].topics
            )
    slices = []
    for s in trace.slices:
        e = s.begin
        if e.endpoint in selected:
            slices.append(s)
        elif e.endpoint and (key := (e.endpoint[0], e.publication)) in topics:
            meta = trace.endpoints[e.endpoint]
            if not meta.conflicting and len(meta.topics | topics[key]) <= 1:
                slices.append(s)
    visible = set(slices)
    return Trace(
        slices,
        [(a, b) for a, b in trace.links if a in visible and b in visible],
        trace.endpoints,
    )


def nanoseconds(us: Decimal | int) -> int:
    # Absolute endpoints, never start + floating-point duration. Round the
    # exact JSON decimal to nearest nanosecond (ties to even).
    numerator, denominator = us.as_integer_ratio()
    ns, remainder = divmod(numerator * 1000, denominator)
    ns += 2 * remainder > denominator or (
        2 * remainder == denominator and ns % 2
    )
    if not 0 <= ns < 2**63:
        msg = "timestamp exceeds Perfetto's signed nanosecond range"
        raise ValueError(msg)
    return ns


def annotation(event: TrackEvent, section: str, name: str, value: Any) -> None:
    """Keep scalars typed; isolate JSON values and arbitrary field names."""
    arg = event.debug_annotations.add()
    if isinstance(value, bool):
        arg.bool_value = value
    elif isinstance(value, int) and -(2**63) <= value < 2**63:
        arg.int_value = value
    elif isinstance(value, int) and 0 <= value < 2**64:
        arg.uint_value = value
    elif isinstance(value, Decimal) and math.isfinite(value):
        arg.double_value = float(value)
    elif isinstance(value, str):
        arg.string_value = value
    else:
        section += "_json"
        arg.string_value = json.dumps(
            value, separators=(",", ":"), default=float, allow_nan=False
        )
    # Trace Processor maps unescaped dots to underscores.
    arg.name = f"{section}/{quote(name, safe='').replace('.', '%2E')}"


class Tracks:
    def __init__(self, stream: BinaryIO) -> None:
        self.builder = StreamingTraceProtoBuilder(stream)
        self.descriptors = {}
        self.lanes = defaultdict(list)

    def descriptor(self, parent: int, name: str) -> int:
        key = (parent, name)
        if key not in self.descriptors:
            packet = self.builder.create_packet()
            descriptor = packet.track_descriptor
            descriptor.uuid = self.descriptors[key] = len(self.descriptors) + 1
            descriptor.name = name
            descriptor.parent_uuid = parent
            descriptor.sibling_merge_behavior = (
                TrackDescriptor.SIBLING_MERGE_BEHAVIOR_NONE
            )
            self.builder.write_packet(packet)
        return self.descriptors[key]

    def lane(
        self, node: str, topic: str, stage: str, start: int, end: int
    ) -> int:
        parent = self.descriptor(self.descriptor(0, node), topic)
        ends = self.lanes[parent, stage]
        # Separate overlapping spans and coincident flow anchors.
        index = next((i for i, t in enumerate(ends) if t < start), len(ends))
        if index == len(ends):
            ends.append(end)
        else:
            ends[index] = end
        return self.descriptor(parent, f"{stage} {index + 1:02d}")

    def packet(self, ts: int, track: int, kind: int) -> Any:
        packet = self.builder.create_packet()
        packet.timestamp = ts
        packet.trusted_packet_sequence_id = 1
        packet.track_event.track_uuid = track
        packet.track_event.type = kind
        return packet


def describe(
    s: Slice, meta: Metadata, selected: str | None
) -> tuple[str, str, str, dict]:
    e = s.begin
    name = owner(meta)
    topic = label(e.args.get("topic")) or (
        next(iter(meta.topics)) if len(meta.topics) == 1 else None
    )
    role = (
        "unknown"
        if not name
        else "node"
        if not selected
        else "selected"
        if name == selected
        else "peer"
    )
    node = ("Peer " if role == "peer" else "") + (name or "Unknown node")
    zid, node_id = (
        next(iter(meta.instances)) if name and meta.instances else (None, None)
    )
    instance = f"zid {zid}, node {node_id}, " if zid else ""
    node += f" ({instance}source pid {e.raw.get('pid')!r})"
    endpoint = (
        f"{e.endpoint[1]} [{e.endpoint[2]}]"
        if e.endpoint
        else "unknown endpoint"
    )
    fields = {
        "source_index": e.index,
        "source_phase": e.raw["ph"],
        "view_role": role,
        "ownership": "conflicting"
        if meta.conflicting
        else "known"
        if name
        else "missing",
        "node_name": name,
        "node_zid": zid,
        "node_id": node_id,
        "topic": topic,
        "reason": label(e.args.get("reason")),
        "source_pid": e.raw.get("pid"),
        "source_tid": e.raw.get("tid"),
        "source_native_id": e.raw.get("id"),
        "status": s.status,
    }
    stage = (
        "Queue outcomes"
        if s.name in TERMINALS | {"Enqueue entry"}
        else "Waiting"
        if s.name.startswith("Enqueue ->")
        else s.name
    )
    return node, topic or "Unknown topic", f"{endpoint} / {stage}", fields


def annotate_slice(event: TrackEvent, s: Slice, fields: dict) -> None:
    if s.end:
        fields["end_index"] = s.end.index
        if s.end.op == s.begin.op and s.end.args != s.begin.args:
            for key, value in s.end.args.items():
                annotation(event, "end", key, value)
    for key, value in s.begin.args.items():
        annotation(event, "original", key, value)
    for key, value in fields.items():
        if value is not None:
            annotation(event, "message_trace", key, value)


def write_packets(trace: Trace, stream: BinaryIO, node: str | None) -> None:
    tracks = Tracks(stream)
    if node:
        trace = focus(trace, node)
    outgoing, incoming = defaultdict(list), defaultdict(list)
    for flow_id, (source, target) in enumerate(trace.links, 1):
        outgoing[source].append(flow_id)
        incoming[target].append(flow_id)
    # Source-first at equal timestamps, including zero-duration queue waits.
    rank = {"Put entry": 0, "Enqueue entry": 1, **dict.fromkeys(TERMINALS, 2)}
    trace.slices.sort(
        key=lambda s: (s.begin.ts, rank.get(s.name, 3), s.begin.index)
    )
    ends, unknown = [], Metadata()
    for s in trace.slices:
        e = s.begin
        start, end = nanoseconds(e.ts), nanoseconds((s.end or e).ts)
        while ends and ends[0][0] <= start:
            ts, track = heapq.heappop(ends)
            tracks.builder.write_packet(
                tracks.packet(ts, track, TrackEvent.TYPE_SLICE_END)
            )
        owner_name, topic, stage, fields = describe(
            s, trace.endpoints.get(e.endpoint, unknown), node
        )
        track = tracks.lane(owner_name, topic, stage, start, end)
        packet = tracks.packet(
            start,
            track,
            TrackEvent.TYPE_SLICE_BEGIN
            if end > start
            else TrackEvent.TYPE_INSTANT,
        )
        event = packet.track_event
        event.name = s.name
        event.categories.append("ros_z.message")
        event.flow_ids.extend(outgoing.get(s, ()))
        event.terminating_flow_ids.extend(incoming.get(s, ()))
        if s.name == "Enqueue entry" and s in incoming:
            fields["association"] = "same publication; live/replay unknown"
        annotate_slice(event, s, fields)
        tracks.builder.write_packet(packet)
        if end > start:
            heapq.heappush(ends, (end, track))
    for ts, track in sorted(ends):
        tracks.builder.write_packet(
            tracks.packet(ts, track, TrackEvent.TYPE_SLICE_END)
        )


def convert(source: Path, destination: Path, node: str | None = None) -> None:
    if os.path.lexists(destination):
        raise FileExistsError(destination)
    with source.open("rb") as stream:
        trace = correlate(json.load(stream, parse_float=Decimal))
    with tempfile.NamedTemporaryFile(
        dir=destination.parent, suffix=".pftrace"
    ) as stream:
        write_packets(trace, stream, node)
        stream.flush()
        os.link(
            stream.name, destination
        )  # Atomic, no overwrite (even aliases).


def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("input", type=Path)
    parser.add_argument(
        "--output", type=Path, required=True, help="new .pftrace file"
    )
    parser.add_argument(
        "--node",
        metavar="/NAMESPACE/NAME",
        help="exact fully qualified node name (all known instances)",
    )
    args = parser.parse_args()
    if args.node is not None and not args.node.startswith("/"):
        parser.error("--node requires a fully qualified node name")
    try:
        convert(args.input, args.output, args.node)
    except (OSError, ValueError, TypeError) as error:
        parser.exit(1, f"{parser.prog}: {error}\n")


if __name__ == "__main__":
    main()
