use std::path::Path;

use color_eyre::{eyre::eyre, Result};
use syn::Error;

pub trait SynContext<T, E> {
    fn syn_context<P>(self, file_path: P) -> Result<T>
    where
        P: AsRef<Path>;
}

impl<T> SynContext<T, Error> for syn::Result<T> {
    fn syn_context<P>(self, file_path: P) -> Result<T>
    where
        P: AsRef<Path>,
    {
        self.map_err(|error| {
            let start = error.span().start();
            eyre!(
                "{error} at {}:{}:{}",
                file_path.as_ref().display(),
                start.line,
                start.column
            )
        })
    }
}
