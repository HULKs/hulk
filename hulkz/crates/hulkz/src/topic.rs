//! Topic expression parsing and resolution.
//!
//! Topic expressions support ROS2-like shorthands:
//! - `/topic` -> absolute
//! - `topic` -> namespace-relative
//! - `~/topic` -> node-relative to current node
//! - `~node/topic` -> node-relative to explicit node

use crate::error::{Error, Result};

/// Canonical, resolved topic used in wire keys.
pub type Topic = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ParsedTopic {
    Absolute(String),
    Relative(String),
    PrivateCurrentNode(String),
    PrivateSpecificNode { node: String, path: String },
}

/// User-facing topic expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TopicExpression {
    raw: String,
}

impl TopicExpression {
    /// Parses and validates a topic expression.
    pub fn parse(input: &str) -> Result<Self> {
        let raw = input.trim();
        if raw.is_empty() {
            return Err(Error::InvalidTopicExpression(
                "topic expression must not be empty".to_string(),
            ));
        }

        parse_parts(raw)?;
        Ok(Self {
            raw: raw.to_string(),
        })
    }

    /// Returns the original expression string.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Resolves this expression into a canonical topic string.
    ///
    /// Resolution rules:
    /// - `/a` -> `a`
    /// - `b` -> `{namespace}/b`
    /// - `~/c` -> `{namespace}/{default_node}/c`
    /// - `~node/c` -> `{namespace}/node/c`
    pub fn resolve(&self, namespace: &str, default_node: Option<&str>) -> Result<Topic> {
        if namespace.is_empty() {
            return Err(Error::InvalidTopicExpression(
                "namespace must not be empty".to_string(),
            ));
        }

        match parse_parts(&self.raw)? {
            ParsedTopic::Absolute(path) => Ok(path),
            ParsedTopic::Relative(path) => Ok(format!("{namespace}/{path}")),
            ParsedTopic::PrivateCurrentNode(path) => {
                let node = default_node.filter(|node| !node.is_empty());
                let Some(node) = node else {
                    return Err(Error::NodeRequiredForPrivate);
                };
                Ok(format!("{namespace}/{node}/{path}"))
            }
            ParsedTopic::PrivateSpecificNode { node, path } => Ok(format!("{namespace}/{node}/{path}")),
        }
    }
}

impl From<&str> for TopicExpression {
    fn from(value: &str) -> Self {
        Self {
            raw: value.trim().to_string(),
        }
    }
}

impl From<String> for TopicExpression {
    fn from(value: String) -> Self {
        Self {
            raw: value.trim().to_string(),
        }
    }
}

fn parse_parts(input: &str) -> Result<ParsedTopic> {
    if let Some(path) = input.strip_prefix('/') {
        if path.is_empty() {
            return Err(Error::InvalidTopicExpression(
                "absolute topic path must not be empty".to_string(),
            ));
        }
        return Ok(ParsedTopic::Absolute(path.to_string()));
    }

    if let Some(rest) = input.strip_prefix('~') {
        if let Some(path) = rest.strip_prefix('/') {
            if path.is_empty() {
                return Err(Error::InvalidTopicExpression(
                    "private topic path must not be empty".to_string(),
                ));
            }
            return Ok(ParsedTopic::PrivateCurrentNode(path.to_string()));
        }

        let Some((node, path)) = rest.split_once('/') else {
            return Err(Error::InvalidTopicExpression(
                "invalid private topic syntax; use ~/path or ~node/path".to_string(),
            ));
        };
        if node.is_empty() || path.is_empty() {
            return Err(Error::InvalidTopicExpression(
                "invalid private topic syntax; use ~/path or ~node/path".to_string(),
            ));
        }
        return Ok(ParsedTopic::PrivateSpecificNode {
            node: node.to_string(),
            path: path.to_string(),
        });
    }

    Ok(ParsedTopic::Relative(input.to_string()))
}

/// Percent-encodes a topic into a single key segment.
pub fn encode_topic_segment(topic: &str) -> String {
    let mut encoded = String::with_capacity(topic.len());
    for byte in topic.as_bytes() {
        if is_unreserved(*byte) {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push(hex((*byte >> 4) & 0x0f));
            encoded.push(hex(*byte & 0x0f));
        }
    }
    encoded
}

/// Decodes a percent-encoded topic segment back into a topic.
pub fn decode_topic_segment(encoded: &str) -> Result<String> {
    let mut bytes = Vec::with_capacity(encoded.len());
    let input = encoded.as_bytes();
    let mut index = 0usize;

    while index < input.len() {
        if input[index] == b'%' {
            if index + 2 >= input.len() {
                return Err(Error::InvalidEncodedTopic {
                    encoded: encoded.to_string(),
                    reason: "truncated percent escape".to_string(),
                });
            }
            let high = from_hex(input[index + 1]).ok_or_else(|| Error::InvalidEncodedTopic {
                encoded: encoded.to_string(),
                reason: "invalid percent escape".to_string(),
            })?;
            let low = from_hex(input[index + 2]).ok_or_else(|| Error::InvalidEncodedTopic {
                encoded: encoded.to_string(),
                reason: "invalid percent escape".to_string(),
            })?;
            bytes.push((high << 4) | low);
            index += 3;
            continue;
        }
        bytes.push(input[index]);
        index += 1;
    }

    String::from_utf8(bytes).map_err(|error| Error::InvalidEncodedTopic {
        encoded: encoded.to_string(),
        reason: error.to_string(),
    })
}

fn is_unreserved(byte: u8) -> bool {
    matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~')
}

fn hex(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + (value - 10)) as char,
        _ => unreachable!("hex nibble out of range"),
    }
}

fn from_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_topic_segment, encode_topic_segment, TopicExpression};

    #[test]
    fn resolves_topic_expression_variants() {
        let absolute = TopicExpression::parse("/fleet/status").unwrap();
        let relative = TopicExpression::parse("camera/front").unwrap();
        let private_current = TopicExpression::parse("~/debug").unwrap();
        let private_node = TopicExpression::parse("~vision/debug").unwrap();

        assert_eq!(
            absolute.resolve("robot", Some("node")).unwrap(),
            "fleet/status"
        );
        assert_eq!(
            relative.resolve("robot", Some("node")).unwrap(),
            "robot/camera/front"
        );
        assert_eq!(
            private_current.resolve("robot", Some("planner")).unwrap(),
            "robot/planner/debug"
        );
        assert_eq!(
            private_node.resolve("robot", Some("planner")).unwrap(),
            "robot/vision/debug"
        );
    }

    #[test]
    fn encode_decode_topic_segment_roundtrip() {
        let topic = "robot/nav/imu ~/x";
        let encoded = encode_topic_segment(topic);
        let decoded = decode_topic_segment(&encoded).unwrap();
        assert_eq!(decoded, topic);
    }
}
