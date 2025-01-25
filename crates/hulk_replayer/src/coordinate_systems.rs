use std::{
    ops::RangeInclusive,
    time::{Duration, SystemTime},
};

use derive_more::{Add, AddAssign, Mul, Neg, Rem, Sub, SubAssign};
use eframe::egui::remap;

/// # Absolute Time Coordinate
///
/// Example: Recording Frames
///
/// - Origin: `std::time::UNIX_EPOCH``
/// - Scale: `SystemTime`/`Duration`, i.e., Seconds
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct AbsoluteTime {
    inner: SystemTime,
}

/// # Relative Time Coordinate
///
/// Example: Current Replay Position
///
/// - Origin: First Recording Frame
/// - Scale: `f32`` in Seconds
#[derive(
    Add, AddAssign, Copy, Clone, Debug, Mul, Neg, PartialEq, PartialOrd, Rem, Sub, SubAssign,
)]
pub struct RelativeTime {
    inner: f32,
}

/// # Relative Screen Coordinate
///
/// Example: Replay Position in Viewport
///
/// - Origin: `painter.clip_rect().left()`
/// - Scale: `painter.clip_rect().width()`
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct RelativeScreen {
    inner: f32,
}

/// # Absolute Screen Coordinate
///
/// Example: Cursor Position
///
/// - Origin: Left of Screen
/// - Scale: egui Logical Points
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct AbsoluteScreen {
    inner: f32,
}

/// # Range of Frames
///
/// - Start: Start of First Recording Frame
/// - End: End of Last Recording Frame
#[derive(Clone, Debug)]
pub struct FrameRange {
    inner: RangeInclusive<AbsoluteTime>,
}

/// # Range of Viewport
///
/// - Start: Relative Time at `painter.clip_rect().left()`
/// - End: Relative Time at `painter.clip_rect().right()`
#[derive(Clone, Debug)]
pub struct ViewportRange {
    inner: RangeInclusive<RelativeTime>,
}

/// # Range of Screen
///
/// - Start: `painter.clip_rect().left()`
/// - End: `painter.clip_rect().right()`
#[derive(Clone, Debug)]
pub struct ScreenRange {
    inner: RangeInclusive<AbsoluteScreen>,
}

impl AbsoluteTime {
    pub fn new(inner: SystemTime) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> SystemTime {
        self.inner
    }

    pub fn map_to_relative_time(&self, frame_range: &FrameRange) -> RelativeTime {
        // shitty if-else because std::time::Duration cannot be negative
        RelativeTime::new(if self.inner >= frame_range.start().inner {
            self.inner
                .duration_since(frame_range.start().inner())
                .expect("time ran backwards")
                .as_secs_f32()
        } else {
            -frame_range
                .start()
                .inner()
                .duration_since(self.inner)
                .expect("time ran backwards")
                .as_secs_f32()
        })
    }
}

impl RelativeTime {
    pub fn new(inner: f32) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> f32 {
        self.inner
    }

    pub fn map_to_absolute_time(&self, frame_range: &FrameRange) -> AbsoluteTime {
        // shitty if-else because std::time::Duration cannot be negative
        let duration = Duration::from_secs_f32(self.inner.abs());
        AbsoluteTime::new(if self.inner >= 0.0 {
            frame_range.start().inner() + duration
        } else {
            frame_range.start().inner() - duration
        })
    }

    pub fn map_to_relative_screen(&self, viewport_range: &ViewportRange) -> RelativeScreen {
        RelativeScreen::new(remap(
            self.inner,
            RangeInclusive::new(viewport_range.start().inner(), viewport_range.end().inner()),
            RangeInclusive::new(0.0, 1.0),
        ))
    }
}

impl RelativeScreen {
    pub fn new(inner: f32) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> f32 {
        self.inner
    }

    pub fn map_to_relative_time(&self, viewport_range: &ViewportRange) -> RelativeTime {
        RelativeTime::new(remap(
            self.inner,
            RangeInclusive::new(0.0, 1.0),
            RangeInclusive::new(viewport_range.start().inner(), viewport_range.end().inner()),
        ))
    }

    pub fn scale_to_relative_time(&self, viewport_range: &ViewportRange) -> RelativeTime {
        RelativeTime::new(
            self.inner * (viewport_range.end().inner() - viewport_range.start().inner()),
        )
    }

    pub fn map_to_absolute_screen(&self, screen_range: &ScreenRange) -> AbsoluteScreen {
        AbsoluteScreen::new(remap(
            self.inner,
            RangeInclusive::new(0.0, 1.0),
            RangeInclusive::new(screen_range.start().inner(), screen_range.end().inner()),
        ))
    }
}

impl AbsoluteScreen {
    pub fn new(inner: f32) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> f32 {
        self.inner
    }

    pub fn map_to_relative_screen(&self, screen_range: &ScreenRange) -> RelativeScreen {
        RelativeScreen::new(remap(
            self.inner,
            RangeInclusive::new(screen_range.start().inner(), screen_range.end().inner()),
            RangeInclusive::new(0.0, 1.0),
        ))
    }

    pub fn scale_to_relative_screen(&self, screen_range: &ScreenRange) -> RelativeScreen {
        RelativeScreen::new(
            self.inner / (screen_range.end().inner() - screen_range.start().inner()),
        )
    }
}

impl FrameRange {
    pub fn new(start: AbsoluteTime, end: AbsoluteTime) -> Self {
        Self {
            inner: RangeInclusive::new(start, end),
        }
    }

    pub fn start(&self) -> AbsoluteTime {
        *self.inner.start()
    }

    pub fn end(&self) -> AbsoluteTime {
        *self.inner.end()
    }
}

impl ViewportRange {
    pub fn new(start: RelativeTime, end: RelativeTime) -> Self {
        Self {
            inner: RangeInclusive::new(start, end),
        }
    }

    pub fn from_frame_range(frame_range: &FrameRange) -> Self {
        const MINIMUM_WIDTH: f32 = 0.001;

        Self::new(
            RelativeTime::new(0.0),
            RelativeTime::new(
                frame_range
                    .end()
                    .inner()
                    .duration_since(frame_range.start().inner())
                    .expect("time ran backwards")
                    .as_secs_f32()
                    .max(MINIMUM_WIDTH),
            ),
        )
    }

    pub fn start(&self) -> RelativeTime {
        *self.inner.start()
    }

    pub fn end(&self) -> RelativeTime {
        *self.inner.end()
    }
}

impl ScreenRange {
    pub fn new(start: AbsoluteScreen, end: AbsoluteScreen) -> Self {
        Self {
            inner: RangeInclusive::new(start, end),
        }
    }

    pub fn start(&self) -> AbsoluteScreen {
        *self.inner.start()
    }

    pub fn end(&self) -> AbsoluteScreen {
        *self.inner.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_frame_range() -> FrameRange {
        FrameRange::new(
            AbsoluteTime::new(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
            AbsoluteTime::new(SystemTime::UNIX_EPOCH + Duration::from_secs(3)),
        )
    }

    fn create_viewport_range() -> ViewportRange {
        ViewportRange::new(RelativeTime::new(1.0), RelativeTime::new(3.0))
    }

    fn create_screen_range() -> ScreenRange {
        ScreenRange::new(AbsoluteScreen::new(100.0), AbsoluteScreen::new(300.0))
    }

    fn create_absolute_time_relative_time_cases() -> Vec<(AbsoluteTime, RelativeTime)> {
        vec![
            (
                AbsoluteTime::new(SystemTime::UNIX_EPOCH + Duration::from_secs(0)),
                RelativeTime::new(-1.0),
            ),
            (
                AbsoluteTime::new(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
                RelativeTime::new(0.0),
            ),
            (
                AbsoluteTime::new(SystemTime::UNIX_EPOCH + Duration::from_secs(2)),
                RelativeTime::new(1.0),
            ),
            (
                AbsoluteTime::new(SystemTime::UNIX_EPOCH + Duration::from_secs(3)),
                RelativeTime::new(2.0),
            ),
            (
                AbsoluteTime::new(SystemTime::UNIX_EPOCH + Duration::from_secs(4)),
                RelativeTime::new(3.0),
            ),
        ]
    }

    fn create_relative_time_relative_screen_cases() -> Vec<(RelativeTime, RelativeScreen)> {
        vec![
            (RelativeTime::new(-1.0), RelativeScreen::new(-1.0)),
            (RelativeTime::new(0.0), RelativeScreen::new(-0.5)),
            (RelativeTime::new(1.0), RelativeScreen::new(0.0)),
            (RelativeTime::new(2.0), RelativeScreen::new(0.5)),
            (RelativeTime::new(3.0), RelativeScreen::new(1.0)),
            (RelativeTime::new(4.0), RelativeScreen::new(1.5)),
        ]
    }

    fn create_relative_screen_absolute_screen_cases() -> Vec<(RelativeScreen, AbsoluteScreen)> {
        vec![
            (RelativeScreen::new(-1.0), AbsoluteScreen::new(-100.0)),
            (RelativeScreen::new(-0.5), AbsoluteScreen::new(0.0)),
            (RelativeScreen::new(0.0), AbsoluteScreen::new(100.0)),
            (RelativeScreen::new(0.5), AbsoluteScreen::new(200.0)),
            (RelativeScreen::new(1.0), AbsoluteScreen::new(300.0)),
            (RelativeScreen::new(1.5), AbsoluteScreen::new(400.0)),
        ]
    }

    #[test]
    fn absolute_time_to_relative_time() {
        let range = create_frame_range();
        for (absolute_time, relative_time) in create_absolute_time_relative_time_cases() {
            assert_eq!(
                absolute_time.map_to_relative_time(&range),
                relative_time,
                "absolute_time = {absolute_time:?}"
            );
        }
    }

    #[test]
    fn relative_time_to_absolute_time() {
        let range = create_frame_range();
        for (absolute_time, relative_time) in create_absolute_time_relative_time_cases() {
            assert_eq!(
                relative_time.map_to_absolute_time(&range),
                absolute_time,
                "relative_time = {relative_time:?}"
            );
        }
    }

    #[test]
    fn relative_time_to_relative_screen() {
        let range = create_viewport_range();
        for (relative_time, relative_screen) in create_relative_time_relative_screen_cases() {
            assert_eq!(
                relative_time.map_to_relative_screen(&range),
                relative_screen,
                "relative_time = {relative_time:?}"
            );
        }
    }

    #[test]
    fn relative_screen_to_relative_time() {
        let range = create_viewport_range();
        for (relative_time, relative_screen) in create_relative_time_relative_screen_cases() {
            assert_eq!(
                relative_screen.map_to_relative_time(&range),
                relative_time,
                "relative_screen = {relative_screen:?}"
            );
        }
    }

    #[test]
    fn relative_screen_to_absolute_screen() {
        let range = create_screen_range();
        for (relative_screen, absolute_screen) in create_relative_screen_absolute_screen_cases() {
            assert_eq!(
                relative_screen.map_to_absolute_screen(&range),
                absolute_screen,
                "relative_screen = {relative_screen:?}"
            );
        }
    }

    #[test]
    fn absolute_screen_to_relative_screen() {
        let range = create_screen_range();
        for (relative_screen, absolute_screen) in create_relative_screen_absolute_screen_cases() {
            assert_eq!(
                absolute_screen.map_to_relative_screen(&range),
                relative_screen,
                "absolute_screen = {absolute_screen:?}"
            );
        }
    }
}
