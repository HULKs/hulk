use std::{borrow::Cow, fmt::Display, time::Duration};

use color_eyre::Result;
use futures_util::{stream::FuturesUnordered, Future, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct ProgressIndicator {
    multi_progress: MultiProgress,
    default_style: ProgressStyle,
    error_style: ProgressStyle,
    success_style: ProgressStyle,
}

impl ProgressIndicator {
    pub fn new() -> Self {
        Self {
            multi_progress: MultiProgress::new(),
            default_style: ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
            error_style: ProgressStyle::with_template("{prefix:.bold.dim} {wide_msg:.red}")
                .unwrap(),
            success_style: ProgressStyle::with_template("{prefix:.bold.dim} {wide_msg:.green}")
                .unwrap(),
        }
    }

    pub fn task(&self, prefix: String) -> Task {
        let spinner = ProgressBar::new_spinner()
            .with_style(self.default_style.clone())
            .with_prefix(prefix);
        Task {
            progress: self.multi_progress.add(spinner),
            error_style: self.error_style.clone(),
            success_style: self.success_style.clone(),
        }
    }

    pub async fn map_tasks<T, F>(
        items: impl IntoIterator<Item = T>,
        message: impl Into<Cow<'static, str>> + Clone,
        task: impl Fn(T) -> F + Copy,
    ) where
        T: ToString + Clone,
        F: Future<Output = Result<()>>,
    {
        let multi_progress = Self::new();
        items
            .into_iter()
            .map(|item| (multi_progress.task(item.to_string()), item))
            .map(|(progress, item)| {
                progress.enable_steady_tick();
                progress.set_message(message.clone());
                async move { progress.finish_with(task(item).await) }
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;
    }
}

pub struct Task {
    progress: ProgressBar,
    error_style: ProgressStyle,
    success_style: ProgressStyle,
}

impl Task {
    pub fn enable_steady_tick(&self) {
        self.progress.enable_steady_tick(Duration::from_millis(100));
    }

    pub fn set_message(&self, message: impl Into<Cow<'static, str>>) {
        self.progress.set_message(message)
    }

    pub fn finish_with_success(&self, message: impl Display) {
        self.progress.set_style(self.success_style.clone());
        self.progress.finish_with_message(format!("✓ {message}"));
    }

    pub fn finish_with_error(&self, message: impl Display) {
        self.progress.set_style(self.error_style.clone());
        self.progress.finish_with_message(format!("✗ {message}"));
    }

    pub fn finish_with<T>(&self, result: Result<T, impl Display>) {
        match result {
            Ok(_) => self.finish_with_success("Done"),
            Err(message) => self.finish_with_error(message),
        }
    }
}
