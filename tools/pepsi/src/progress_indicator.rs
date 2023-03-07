use std::{borrow::Cow, time::Duration};

use color_eyre::{owo_colors::OwoColorize, Report, Result};
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
            default_style: ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {msg}")
                .unwrap()
                // The last char is ignored as it provides a final state
                .tick_chars("⠏⠋⠙⠹⢸⣰⣠⣄⣆⡇ "),
            error_style: ProgressStyle::with_template("{prefix:.bold.dim} {msg}").unwrap(),
            success_style: ProgressStyle::with_template("{prefix:.bold.dim} {msg}").unwrap(),
        }
    }

    pub fn task(&self, prefix: String) -> Task {
        let spinner = ProgressBar::new_spinner()
            .with_style(self.default_style.clone())
            .with_prefix(format!("[{prefix}]"));
        spinner.enable_steady_tick(Duration::from_millis(100));
        Task {
            progress: self.multi_progress.add(spinner),
            error_style: self.error_style.clone(),
            success_style: self.success_style.clone(),
        }
    }

    pub async fn map_tasks<T, F, M>(
        items: impl IntoIterator<Item = T>,
        message: impl Into<Cow<'static, str>> + Clone,
        task: impl Fn(T) -> F + Copy,
    ) where
        T: ToString + Clone,
        F: Future<Output = Result<M>>,
        M: Into<TaskMessage>,
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

pub enum TaskMessage {
    EmptyMessage,
    Message(String),
}

impl From<()> for TaskMessage {
    fn from(_: ()) -> Self {
        Self::EmptyMessage
    }
}

impl From<String> for TaskMessage {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl Task {
    pub fn enable_steady_tick(&self) {
        self.progress.enable_steady_tick(Duration::from_millis(100));
    }

    pub fn set_message(&self, message: impl Into<Cow<'static, str>>) {
        self.progress.set_message(message)
    }

    pub fn finish_with_success(&self, message: impl Into<TaskMessage>) {
        self.progress.set_style(self.success_style.clone());
        let icon = "✔".green();
        let message = match message.into() {
            TaskMessage::EmptyMessage => icon.to_string(),
            TaskMessage::Message(message) => format!("{icon}\n{message}"),
        };
        self.progress.finish_with_message(message);
    }

    pub fn finish_with_error(&self, report: Report) {
        self.progress.set_style(self.error_style.clone());
        self.progress
            .finish_with_message(format!("{}{report:?}", "✗".red()));
    }

    pub fn finish_with(&self, result: Result<impl Into<TaskMessage>>) {
        match result {
            Ok(message) => self.finish_with_success(message),
            Err(report) => self.finish_with_error(report),
        }
    }
}
