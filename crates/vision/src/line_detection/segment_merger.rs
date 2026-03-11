use std::iter::Peekable;

use types::image_segments::GenericSegment;

pub struct SegmentMerger<T: Iterator<Item = GenericSegment>> {
    iterator: Peekable<T>,
    maximum_merge_gap: u16,
}

impl<T> SegmentMerger<T>
where
    T: Iterator<Item = GenericSegment>,
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
    T: Iterator<Item = GenericSegment>,
{
    type Item = GenericSegment;

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

fn distance_between_segments(first: GenericSegment, second: GenericSegment) -> u16 {
    (second.start.x() - first.end.x()) + (second.start.y() - first.end.y())
}
