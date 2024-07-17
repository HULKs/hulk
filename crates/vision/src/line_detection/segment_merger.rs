use std::iter::Peekable;

use super::segment::Segment;

pub struct SegmentMerger<T: Iterator<Item = Segment>> {
    iterator: Peekable<T>,
    maximum_merge_gap: u16,
}

impl<T> SegmentMerger<T>
where
    T: Iterator<Item = Segment>,
{
    pub fn new(iterator: T, maximum_merge_gap: u16) -> Self {
        Self {
            iterator: iterator.peekable(),
            maximum_merge_gap,
        }
    }
}

impl<T> Iterator for SegmentMerger<T>
where
    T: Iterator<Item = Segment>,
{
    type Item = Segment;

    fn next(&mut self) -> Option<Self::Item> {
        let mut current = self.iterator.next()?;

        while let Some(next) = self.iterator.peek().copied() {
            if distance_between_segments(current, next) >= self.maximum_merge_gap {
                break;
            }

            let _ = self.iterator.next();
            current.end = next.end;
            current.end_edge_type = next.end_edge_type;
        }

        Some(current)
    }
}

fn distance_between_segments(first: Segment, second: Segment) -> u16 {
    (second.start.x() - first.end.x()) + (second.start.y() - first.end.y())
}
