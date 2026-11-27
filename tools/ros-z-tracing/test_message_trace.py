import unittest

from message_trace import correlate


PUBLICATION = "a" * 32 + "#0"
FAST, SLOW = "b" * 32, "c" * 32


def event(name, phase, ts, subscriber):
    return {
        "cat": "ros_z::pubsub::subscriber",
        "pid": 1,
        "tid": 0,
        "id": 1,
        "name": name,
        "ph": phase,
        "ts": ts,
        "args": {
            "topic": "/test",
            "publication_id": PUBLICATION,
            "subscriber_id": subscriber,
        },
    }


def enqueue(ts, subscriber):
    return [
        event("enqueue_sample", "b", ts, subscriber),
        event("enqueue_sample", "e", ts + 1, subscriber),
    ]


class CorrelationTests(unittest.TestCase):
    def test_fanout_links_each_outcome_to_its_own_enqueue(self):
        trace = correlate(
            [
                *enqueue(1, FAST),
                *enqueue(3, SLOW),
                event("discarded", "i", 5, SLOW),
                event("dequeue", "i", 6, FAST),
            ]
        )
        self.assertCountEqual(
            [(source.begin.ts, target.name) for source, target in trace.links],
            [(1, "dequeue"), (3, "discarded")],
        )

    def test_ambiguous_dequeue_is_visible_but_unlinked(self):
        trace = correlate(
            [
                *enqueue(1, FAST),
                *enqueue(3, FAST),
                event("dequeue", "i", 5, FAST),
            ]
        )
        self.assertEqual(
            [s.begin.ts for s in trace.slices if s.name == "dequeue"], [5]
        )
        self.assertFalse(trace.links)
