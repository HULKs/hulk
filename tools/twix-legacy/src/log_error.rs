use std::fmt::Display;

use log::error;

pub trait LogError {
    fn log_err(self);
}

impl<T, E> LogError for Result<T, E>
where
    E: Display,
{
    fn log_err(self) {
        if let Err(e) = self {
            error!("{e:#}");
        }
    }
}
