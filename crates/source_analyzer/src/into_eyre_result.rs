use std::{fmt::Display, path::Path};

use color_eyre::{eyre::eyre, Result};
use proc_macro2::Span;
use syn::Error;

pub trait SynContext<T, E> {
    fn syn_context(self, file_path: impl AsRef<Path>) -> Result<T>;
}

impl<T> SynContext<T, Error> for syn::Result<T> {
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

pub fn new_syn_error_as_eyre_result<T>(
    span: Span,
    message: impl Display,
    file_path: impl AsRef<Path>,
) -> Result<T> {
    Err(Error::new(span, message)).syn_context(file_path)
}
