use std::time::SystemTime;

use factrs::linalg::Numeric;

pub fn interval_dt<T: Numeric>(start: SystemTime, end: SystemTime) -> T {
    let duration = end.duration_since(start).expect("end must be after start");
    T::from(duration.as_secs_f64())
}

pub fn tau<T: Numeric>(start: SystemTime, end: SystemTime, current: SystemTime) -> T {
    let dt = interval_dt::<T>(start, end);
    let t = interval_dt::<T>(start, current);

    t / dt
}
