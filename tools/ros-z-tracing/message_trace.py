"""Correlate the explicit lifecycle observations in one finite ROS-Z capture."""

from __future__ import annotations

import json
import re
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from decimal import Decimal
from itertools import pairwise
from typing import Any

PUBLISHER = {"publish", "prepare_payload", "put_payload", "retain"}
RECEIVE = {"recv", "recv_with_metadata"}
SPANS = PUBLISHER | RECEIVE | {"enqueue_sample", "process_sample"}
TERMINALS = {"dequeue", "discarded"}
CREATIONS = {"publisher_created", "subscriber_created", "endpoint_created"}
INSTANTS = TERMINALS | CREATIONS | {"publication_id_reserved"}
# Display, not alternate (abbreviated) Display: EndpointGlobalId#i64.
GID = re.compile(r"[0-9a-f]{32}")
PUBLICATION = re.compile(r"([0-9a-f]{32})#(0|-?[1-9][0-9]{0,18})")
Json = dict[str, Any]
Endpoint = tuple[int | str | None, str, str]


def label(value: Any) -> str | None:
    """Some tracing-chrome labels are Rust debug strings; IDs are not."""
    if not isinstance(value, str):
        return None
    if value.startswith('"'):
        try:
            decoded = json.loads(value)
        except ValueError:
            return value
        if isinstance(decoded, str):
            return decoded
    return value


@dataclass(slots=True)
class Observation:
    index: int
    raw: Json
    op: str
    publication: str | None
    endpoint: Endpoint | None

    @property
    def args(self) -> Json:
        return self.raw.get("args", {})

    @property
    def ts(self) -> int | Decimal:
        return self.raw["ts"]


def identity(
    op: str, args: Json, pid: int | str | None
) -> tuple[str | None, Endpoint | None]:
    publication = args.get("publication_id")
    match = (
        PUBLICATION.fullmatch(publication)
        if isinstance(publication, str)
        else None
    )
    if not match or not -(2**63) <= int(match[2]) < 2**63:
        publication = None
    if op in CREATIONS:
        kind = (
            op.removesuffix("_created")
            if op != "endpoint_created"
            else label(args.get("endpoint_kind"))
        )
        gid = args.get("endpoint_id")
    elif op in PUBLISHER or op == "publication_id_reserved":
        kind, gid = "publisher", match[1] if publication else None
    elif op == "error":
        return None, None
    else:
        kind, gid = "subscriber", args.get("subscriber_id")
    if (
        kind in {"publisher", "subscriber"}
        and isinstance(gid, str)
        and GID.fullmatch(gid)
    ):
        return publication, (pid, kind, gid)
    return publication, None


def operation(raw: Json, index: int) -> str | None:
    category = raw.get("cat", "")
    if not isinstance(category, str) or (
        category != "ros_z::pubsub"
        and not category.startswith("ros_z::pubsub::")
    ):
        return None
    args, phase = raw.get("args", {}), raw.get("ph")
    if not isinstance(args, dict):
        msg = f"event {index}: expected an args object"
        raise TypeError(msg)
    if phase in {"i", "I"} and "error" in args:
        return "error"
    operations = (
        SPANS
        if phase in {"b", "e"}
        else INSTANTS
        if phase in {"i", "I"}
        else ()
    )
    for op in (raw.get("name"), label(args.get("operation"))):
        if isinstance(op, str) and op in operations:
            return op
    return None


def observations(data: Any) -> list[Observation]:
    events = data.get("traceEvents") if isinstance(data, dict) else data
    if not isinstance(events, list):
        msg = "expected a traceEvents object or a JSON array"
        raise TypeError(msg)
    result = []
    for index, raw in enumerate(events):
        if not isinstance(raw, dict):
            msg = f"event {index}: expected an object"
            raise TypeError(msg)
        op = operation(raw, index)
        if op is None:
            continue
        ts = raw.get("ts")
        if (
            isinstance(ts, bool)
            or not isinstance(ts, (int, Decimal))
            or not 0 <= ts < 2**63
        ):
            msg = f"event {index}: expected nonnegative decimal microseconds"
            raise ValueError(msg)
        for key in ("id", "pid"):
            value = raw.get(key)
            if value is not None and (
                isinstance(value, bool) or not isinstance(value, (str, int))
            ):
                msg = f"event {index}: unsupported {key}"
                raise ValueError(msg)
        pub, endpoint = identity(op, raw.get("args", {}), raw.get("pid"))
        result.append(Observation(index, raw, op, pub, endpoint))
    return sorted(result, key=lambda e: (e.ts, e.index))


@dataclass(eq=False, slots=True)
class Slice:
    begin: Observation
    name: str
    end: Observation | None = None
    status: str | None = None


def call_slices(events: list[Observation]) -> list[Slice]:
    """The async ID is a shared root, not a unique function invocation."""
    slices, groups = [], defaultdict(list)
    for e in events:
        if e.op not in SPANS:
            slices.append(Slice(e, e.op))
        elif e.raw.get("id") is None or e.raw.get("pid") is None:
            slices.append(Slice(e, e.op, status="missing async id/pid"))
        elif any(
            k in e.args and not isinstance(e.args[k], str)
            for k in ("publication_id", "subscriber_id")
        ):
            slices.append(Slice(e, e.op, status="invalid identity field"))
        else:
            # Do not pair different raw IDs, even if both failed validation.
            key = (
                e.raw["pid"],
                e.raw["cat"],
                e.raw["id"],
                e.op,
                e.raw.get("name"),
                e.args.get("publication_id"),
                e.args.get("subscriber_id"),
            )
            groups[key].append(e)
    for group in groups.values():
        run, depth = [], 0
        for e in group:
            run.append(e)
            depth += 1 if e.raw["ph"] == "b" else -1
            if depth <= 0:
                if len(run) == 2:
                    slices.append(Slice(run[0], e.op, e))
                else:
                    slices.extend(
                        Slice(x, x.op, status="unpaired/overlapping call")
                        for x in run
                    )
                run, depth = [], 0
        # A lone begin is an entry observation, never a completed call.
        status = (
            "unclosed call" if len(run) == 1 else "unpaired/overlapping call"
        )
        slices.extend(Slice(x, x.op, status=status) for x in run)
    return slices


@dataclass(slots=True)
class Metadata:
    topics: set[str] = field(default_factory=set)
    names: set[str] = field(default_factory=set)
    instances: set[tuple[str, str]] = field(default_factory=set)
    creations: int = 0

    @property
    def conflicting(self) -> bool:
        return self.creations > 1 or any(
            len(s) > 1 for s in (self.topics, self.names, self.instances)
        )


def endpoint_metadata(events: list[Observation]) -> dict[Endpoint, Metadata]:
    catalog = defaultdict(Metadata)
    for e in events:
        if not e.endpoint:
            continue
        meta = catalog[e.endpoint]
        if topic := label(e.args.get("topic")):
            meta.topics.add(topic)
        meta.creations += e.op in CREATIONS
        name, ns = (
            label(e.args.get("node_name")),
            label(e.args.get("node_namespace")),
        )
        if name and ns is not None:
            meta.names.add(
                "/" + "/".join(p for p in (ns.strip("/"), name) if p)
            )
        zid, node_id = e.args.get("node_zid"), e.args.get("node_id")
        if (
            isinstance(zid, str)
            and zid
            and isinstance(node_id, (str, int))
            and not isinstance(node_id, bool)
        ):
            meta.instances.add((zid, str(node_id)))
    return dict(catalog)


@dataclass(slots=True)
class Trace:
    slices: list[Slice]
    links: list[tuple[Slice, Slice]]
    endpoints: dict[Endpoint, Metadata]


def put_origin(calls: list[Slice]) -> Observation | None:
    """Never choose among colliding publications, including partial calls."""
    by_op = {s.name: s for s in calls}
    if len(by_op) != len(calls) or any(
        s.status not in (None, "unclosed call") for s in calls
    ):
        return None
    put = by_op.get("put_payload")
    if not put:
        return None
    # Every observed stage must be consistent with this single invocation.
    order = [
        by_op[n]
        for n in (
            "publication_id_reserved",
            "prepare_payload",
            "retain",
            "put_payload",
        )
        if n in by_op
    ]
    if any((a.end or a.begin).ts > b.begin.ts for a, b in pairwise(order)):
        return None
    publish = by_op.get("publish")
    if publish and (
        any(
            s.begin.raw.get("id") != publish.begin.raw.get("id")
            or s.begin.ts < publish.begin.ts
            or (publish.end and (s.end or s.begin).ts > publish.end.ts)
            for s in order
            if s.name in PUBLISHER
        )
        or (
            "publication_id_reserved" in by_op
            and by_op["publication_id_reserved"].begin.ts > publish.begin.ts
        )
    ):
        return None
    return put.begin


def queue_status(group: list[Slice]) -> str | None:
    counts = Counter(s.name for s in group)
    if (
        any(s.status not in (None, "unclosed call") for s in group)
        or max(counts.values()) > 1
        or sum(counts[n] for n in TERMINALS) > 1
    ):
        return "ambiguous or incomplete calls/outcomes"
    by_op = {s.name: s.begin for s in group}
    terminal = by_op.get("discarded") or by_op.get("dequeue")
    if "enqueue_sample" not in by_op:
        return "missing enqueue"
    processing = by_op.get("process_sample")
    if not terminal:
        return "missing dequeue before processing" if processing else None
    if processing and (
        terminal.op == "discarded" or processing.ts < terminal.ts
    ):
        return "processing conflicts with outcome"
    if terminal.ts < by_op["enqueue_sample"].ts:
        return "terminal precedes enqueue"
    return None


def candidates(
    slices: list[Slice], endpoints: dict[Endpoint, Metadata]
) -> tuple[dict, dict]:
    publishers, queues = defaultdict(list), defaultdict(list)
    for s in slices:
        e = s.begin
        if e.op not in PUBLISHER | TERMINALS | {
            "publication_id_reserved",
            "enqueue_sample",
            "process_sample",
        }:
            continue
        if not e.publication or not e.endpoint or e.endpoint[0] is None:
            s.status = s.status or "missing explicit identity"
        elif endpoints[e.endpoint].conflicting:
            s.status = s.status or "conflicting endpoint metadata"
        elif e.endpoint[1] == "publisher":
            publishers[e.endpoint[0], e.publication].append(s)
        else:
            queues[e.endpoint, e.publication].append(s)
    return publishers, queues


def correlate(data: Any) -> Trace:
    events = observations(data)
    endpoints = endpoint_metadata(events)
    slices = call_slices(events)
    publishers, queues = candidates(slices, endpoints)
    origins = {}
    for key, calls in publishers.items():
        if origin := put_origin(calls):
            marker = Slice(origin, "Put entry")
            slices.append(marker)
            origins[key] = marker
        else:
            for s in calls:
                s.status = s.status or "no unique consistent put"

    links = []
    for (endpoint, publication), group in queues.items():
        if status := queue_status(group):
            for s in group:
                s.status = s.status or status
            continue
        enqueue = next(s.begin for s in group if s.name == "enqueue_sample")
        terminal = next((s for s in group if s.name in TERMINALS), None)
        entry = Slice(enqueue, "Enqueue entry")
        slices.append(entry)
        if terminal:
            slices.append(
                Slice(enqueue, f"Enqueue -> {terminal.name}", terminal.begin)
            )
            links.append((entry, terminal))
        else:
            entry.status = "no observed terminal"
        origin = origins.get((endpoint[0], publication))
        if (
            origin
            and origin.begin.ts <= enqueue.ts
            and len(
                endpoints[origin.begin.endpoint].topics
                | endpoints[endpoint].topics
            )
            <= 1
        ):
            links.append((origin, entry))
    return Trace(slices, links, endpoints)
