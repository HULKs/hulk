use std::path::Path;

use color_eyre::{eyre::eyre, Result};

pub trait SynContext<T> {
    fn syn_context(self, file_path: impl AsRef<Path>) -> Result<T>;
}

impl<T> SynContext<T> for syn::Result<T> {
    fn syn_context(self, file_path: impl AsRef<Path>) -> Result<T> {
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
