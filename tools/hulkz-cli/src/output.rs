//! Output formatting for CLI commands.

use serde::Serialize;

/// Output format for CLI commands.
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    /// Human-readable output
    Human,
    /// JSON output for machine consumption
    Json,
}

impl OutputFormat {
    /// Prints a list of items with a header (human) or as JSON array.
    pub fn print_list<T: Serialize + std::fmt::Display>(
        &self,
        header: &str,
        namespace: &str,
        items: &[T],
    ) {
        match self {
            OutputFormat::Human => {
                println!("{} (namespace: {})", header, namespace);
                if items.is_empty() {
                    println!("  (none)");
                } else {
                    for item in items {
                        println!("  {}", item);
                    }
                }
            }
            OutputFormat::Json => {
                if let Ok(json) = serde_json::to_string(items) {
                    println!("{}", json);
                }
            }
        }
    }

    /// Prints a single event/message line.
    pub fn print_event<T: Serialize>(&self, event_type: &str, data: &T) {
        match self {
            OutputFormat::Human => {
                if let Ok(json) = serde_json::to_string(data) {
                    println!("[{}] {}", event_type, json);
                }
            }
            OutputFormat::Json => {
                #[derive(Serialize)]
                struct EventWrapper<'a, T> {
                    event: &'a str,
                    data: &'a T,
                }
                let wrapper = EventWrapper {
                    event: event_type,
                    data,
                };
                if let Ok(json) = serde_json::to_string(&wrapper) {
                    println!("{}", json);
                }
            }
        }
    }
}
