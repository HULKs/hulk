use std::{fmt::Display, io, path::PathBuf};

use proc_macro2::Span;
use quote::ToTokens;
use thiserror::Error;
use threadbound::ThreadBound;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to perform IO on `{path}`")]
    Io { source: io::Error, path: PathBuf },
    #[error("failed to parse Rust at {path}:{source}")]
    RustParse { source: ParseError, path: PathBuf },
    #[error("failed to read node `{node}` at {path}:{source}")]
    Node {
        source: ParseError,
        node: String,
        path: PathBuf,
    },
    #[error("invalid module path")]
    InvalidModulePath,
    #[error("`{node}` requires output `{output}`, but it is never produced")]
    MissingOutput { node: String, output: String },
    #[error("failed to sort nodes, circular dependency detected")]
    CircularDependency,
}

#[derive(Debug, Error)]
#[error("{}:{}, {message}",
        span.get_ref().cloned().unwrap_or_else(Span::call_site).start().line,
        span.get_ref().cloned().unwrap_or_else(Span::call_site).start().column,
    )]
pub struct ParseError {
    span: ThreadBound<Span>,
    message: String,
}

impl ParseError {
    pub fn new_spanned(tokens: impl ToTokens, message: impl Display) -> Self {
        let span = tokens
            .into_token_stream()
            .into_iter()
            .next()
            .map_or_else(Span::call_site, |token| token.span());
        Self {
            span: ThreadBound::new(span),
            message: message.to_string(),
        }
    }
}

impl From<syn::Error> for ParseError {
    fn from(value: syn::Error) -> Self {
        Self {
            span: ThreadBound::new(value.span()),
            message: value.to_string(),
        }
    }
}
