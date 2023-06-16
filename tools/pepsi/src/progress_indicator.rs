use std::{borrow::Cow, time::Duration};

use color_eyre::{owo_colors::OwoColorize, Report, Result};
use futures_util::{stream::FuturesUnordered, Future, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct ProgressIndicator {
    multi_progress: MultiProgress,
    default_style: ProgressStyle,
    success_style: ProgressStyle,
    error_style: ProgressStyle,
}

impl ProgressIndicator {
    pub fn new() -> Self {
        Self {
            multi_progress: MultiProgress::new(),
            default_style: ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {msg}")
                .unwrap()
                // The last char is ignored as it provides a final state
                .tick_chars("⠏⠋⠙⠹⢸⣰⣠⣄⣆⡇ "),
            success_style: ProgressStyle::with_template("{prefix:.bold.dim} {msg}").unwrap(),
            error_style: ProgressStyle::with_template("{prefix:.bold.dim} {msg}").unwrap(),
        }
    }

    pub fn task(&self, prefix: String) -> Task {
        let spinner = ProgressBar::new_spinner()
            .with_style(self.default_style.clone())
            .with_prefix(format!("[{prefix}]"));
        spinner.enable_steady_tick(Duration::from_millis(100));
        Task {
            progress: self.multi_progress.add(spinner),
            success_style: self.success_style.clone(),
            error_style: self.error_style.clone(),
        }
    }

    pub async fn map_tasks<T, F, M>(
        items: impl IntoIterator<Item = T>,
        message: &'static str,
        task: impl Fn(T) -> F + Copy,
    ) where
        T: ToString,
        F: Future<Output = Result<M>>,
        M: Into<TaskOutput>,
    {
        let multi_progress = Self::new();
        items
            .into_iter()
            .map(|item| (multi_progress.task(item.to_string()), item))
            .map(|(progress, item)| {
                progress.enable_steady_tick();
                progress.set_message(message);
                async move { progress.finish_with(task(item).await) }
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;
    }
}

pub struct Task {
    progress: ProgressBar,
    success_style: ProgressStyle,
    error_style: ProgressStyle,
}

pub enum TaskOutput {
    EmptyOutput,
    Message(String),
}

impl From<()> for TaskOutput {
    fn from(_: ()) -> Self {
        Self::EmptyOutput
    }
}

impl From<String> for TaskOutput {
    fn from(value: String) -> Self {
        if value.is_empty() {
            Self::EmptyOutput
        } else {
            Self::Message(value)
        }
    }
}

impl From<&str> for TaskOutput {
    fn from(value: &str) -> Self {
        if value.is_empty() {
            Self::EmptyOutput
        } else {
            Self::Message(String::from(value))
        }
    }
}

impl Task {
    pub fn enable_steady_tick(&self) {
        self.progress.enable_steady_tick(Duration::from_millis(100));
    }

    pub fn set_message(&self, message: impl Into<Cow<'static, str>>) {
        self.progress.set_message(message)
    }

    pub fn finish_with_success(&self) {
        let icon = "✔".green();
        self.progress.set_style(self.success_style.clone());
        self.progress.finish_with_message(icon.to_string());
    }

    pub fn finish_with_error<'a>(&self, report: Report) -> TaskOutput {
        self.progress.set_style(self.error_style.clone());
        self.progress
            .finish_with_message(format!("{}{report:?}", "✗".red()));
        TaskOutput::EmptyOutput
    }

    pub fn finish_with<'a>(&self, result: Result<impl Into<TaskOutput>>) -> TaskOutput {
        match result {
            Ok(message) => {
                self.finish_with_success();
                message.into()
            }
            Err(report) => self.finish_with_error(report),
        }
    }
}
