mod priority;
mod runtime;
mod selection;
mod team;
mod validation;

use std::{boxed::Box, future::Future, pin::Pin, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::Context;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(runtime::run(ctx))
}

#[cfg(test)]
mod tests;
