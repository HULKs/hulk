//! QoS profile encoding/decoding for liveliness tokens.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;
use core::fmt::Display;

/// QoS profile for native ros-z entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct QosProfile {
    pub reliability: QosReliability,
    pub durability: QosDurability,
    pub history: QosHistory,
    pub deadline: QosDuration,
    pub lifespan: QosDuration,
    pub liveliness: QosLiveliness,
    pub liveliness_lease_duration: QosDuration,
}

impl QosProfile {
    /// Encode QoS to string for liveliness token.
    /// Format: [reliability]:[durability]:[history],[depth]:[deadline]:[lifespan]:[liveliness]
    pub fn encode(&self) -> String {
        use alloc::format;
        let default_qos = Self::default();

        // Reliability - empty if default (encoded values: 1=Reliable, 2=BestEffort)
        let reliability = if self.reliability != default_qos.reliability {
            match self.reliability {
                QosReliability::Reliable => "1",
                QosReliability::BestEffort => "2",
            }
        } else {
            ""
        };

        // Durability - empty if default (encoded values: 1=TransientLocal, 2=Volatile)
        let durability = if self.durability != default_qos.durability {
            match self.durability {
                QosDurability::TransientLocal => "1",
                QosDurability::Volatile => "2",
            }
        } else {
            ""
        };

        // History format: <history_kind>,<depth>
        // Only include kind if non-default, always include depth
        let history = match self.history {
            QosHistory::KeepLast(depth) => {
                if self.history != default_qos.history {
                    format!("1,{}", depth)
                } else {
                    format!(",{}", depth)
                }
            }
            QosHistory::KeepAll => "2,".to_string(),
        };

        let deadline = encode_duration(self.deadline, default_qos.deadline);
        let lifespan = encode_duration(self.lifespan, default_qos.lifespan);
        let liveliness = encode_liveliness(
            self.liveliness,
            self.liveliness_lease_duration,
            default_qos.liveliness,
            default_qos.liveliness_lease_duration,
        );

        format!(
            "{}:{}:{}:{}:{}:{}",
            reliability, durability, history, deadline, lifespan, liveliness
        )
    }

    /// Decode QoS from liveliness token string.
    pub fn decode(s: &str) -> Result<Self, QosDecodeError> {
        let fields: alloc::vec::Vec<&str> = s.split(':').collect();
        if fields.len() != 3 && fields.len() != 6 {
            return Err(QosDecodeError::InvalidFormat);
        }

        let default_qos = Self::default();

        // Parse reliability (encoded values: 1=Reliable, 2=BestEffort)
        let reliability = match fields[0] {
            "" => default_qos.reliability,
            "1" => QosReliability::Reliable,
            "2" => QosReliability::BestEffort,
            _ => return Err(QosDecodeError::InvalidReliability),
        };

        // Parse durability (encoded values: 1=TransientLocal, 2=Volatile)
        let durability = match fields[1] {
            "" => default_qos.durability,
            "1" => QosDurability::TransientLocal,
            "2" => QosDurability::Volatile,
            _ => return Err(QosDecodeError::InvalidDurability),
        };

        // Parse history: <kind>,<depth>
        let history_parts: alloc::vec::Vec<&str> = fields[2].split(',').collect();
        if history_parts.len() != 2 {
            return Err(QosDecodeError::InvalidHistory);
        }

        let history = match history_parts[0] {
            "" | "1" => {
                // KeepLast - parse depth
                if history_parts[1].is_empty() {
                    return decode_legacy_default_history(
                        fields.len(),
                        default_qos,
                        reliability,
                        durability,
                    );
                }
                let depth = history_parts[1]
                    .parse::<usize>()
                    .map_err(|_| QosDecodeError::InvalidHistory)?;
                QosHistory::KeepLast(depth)
            }
            "2" => QosHistory::KeepAll,
            _ => return Err(QosDecodeError::InvalidHistory),
        };

        let (deadline, lifespan, liveliness, liveliness_lease_duration) = if fields.len() == 6 {
            let deadline = decode_duration(fields[3])?;
            let lifespan = decode_duration(fields[4])?;
            let (liveliness, lease) = decode_liveliness(fields[5])?;
            (deadline, lifespan, liveliness, lease)
        } else {
            (
                default_qos.deadline,
                default_qos.lifespan,
                default_qos.liveliness,
                default_qos.liveliness_lease_duration,
            )
        };

        Ok(QosProfile {
            reliability,
            durability,
            history,
            deadline,
            lifespan,
            liveliness,
            liveliness_lease_duration,
        })
    }
}

fn encode_duration(duration: QosDuration, default: QosDuration) -> String {
    use alloc::format;

    if duration == default {
        ",".to_string()
    } else {
        format!("{},{}", duration.sec, duration.nsec)
    }
}

fn encode_liveliness(
    liveliness: QosLiveliness,
    lease: QosDuration,
    default_liveliness: QosLiveliness,
    default_lease: QosDuration,
) -> String {
    use alloc::format;

    if liveliness == default_liveliness && lease == default_lease {
        ",,".to_string()
    } else {
        let kind = match liveliness {
            QosLiveliness::Automatic => "1",
            QosLiveliness::ManualByNode => "2",
            QosLiveliness::ManualByTopic => "3",
        };
        format!("{},{},{}", kind, lease.sec, lease.nsec)
    }
}

fn decode_duration(s: &str) -> Result<QosDuration, QosDecodeError> {
    let parts: alloc::vec::Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err(QosDecodeError::InvalidDuration);
    }
    if parts[0].is_empty() && parts[1].is_empty() {
        return Ok(QosDuration::default());
    }
    if parts[0].is_empty() || parts[1].is_empty() {
        return Err(QosDecodeError::InvalidDuration);
    }

    Ok(QosDuration {
        sec: parts[0]
            .parse()
            .map_err(|_| QosDecodeError::InvalidDuration)?,
        nsec: parts[1]
            .parse()
            .map_err(|_| QosDecodeError::InvalidDuration)?,
    })
}

fn decode_liveliness(s: &str) -> Result<(QosLiveliness, QosDuration), QosDecodeError> {
    let parts: alloc::vec::Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        return Err(QosDecodeError::InvalidLiveliness);
    }
    if parts[0].is_empty() && parts[1].is_empty() && parts[2].is_empty() {
        return Ok((QosLiveliness::default(), QosDuration::default()));
    }
    if parts[0].is_empty() || parts[1].is_empty() || parts[2].is_empty() {
        return Err(QosDecodeError::InvalidLiveliness);
    }

    let liveliness = match parts[0] {
        "1" => QosLiveliness::Automatic,
        "2" => QosLiveliness::ManualByNode,
        "3" => QosLiveliness::ManualByTopic,
        _ => return Err(QosDecodeError::InvalidLiveliness),
    };
    let lease = QosDuration {
        sec: parts[1]
            .parse()
            .map_err(|_| QosDecodeError::InvalidLiveliness)?,
        nsec: parts[2]
            .parse()
            .map_err(|_| QosDecodeError::InvalidLiveliness)?,
    };

    Ok((liveliness, lease))
}

fn decode_legacy_default_history(
    field_count: usize,
    default_qos: QosProfile,
    reliability: QosReliability,
    durability: QosDurability,
) -> Result<QosProfile, QosDecodeError> {
    if field_count == 3 {
        Ok(QosProfile {
            reliability,
            durability,
            history: default_qos.history,
            deadline: default_qos.deadline,
            lifespan: default_qos.lifespan,
            liveliness: default_qos.liveliness,
            liveliness_lease_duration: default_qos.liveliness_lease_duration,
        })
    } else {
        Err(QosDecodeError::InvalidHistory)
    }
}

/// QoS reliability policy.
///
/// Native ros-z default: Reliable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum QosReliability {
    BestEffort = 0,
    #[default]
    Reliable = 1,
}

/// QoS durability policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum QosDurability {
    #[default]
    Volatile = 0,
    TransientLocal = 1,
}

/// Represents a QoS duration in seconds and nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QosDuration {
    pub sec: u64,
    pub nsec: u64,
}

impl QosDuration {
    pub const INFINITE: QosDuration = QosDuration {
        sec: 9223372036,
        nsec: 854775807,
    };
}

impl Default for QosDuration {
    fn default() -> Self {
        Self::INFINITE
    }
}

/// QoS liveliness policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum QosLiveliness {
    #[default]
    Automatic = 0,
    ManualByNode = 1,
    ManualByTopic = 2,
}

/// QoS history policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QosHistory {
    KeepLast(usize),
    KeepAll,
}

impl Default for QosHistory {
    fn default() -> Self {
        QosHistory::KeepLast(10)
    }
}

impl QosHistory {
    pub fn from_depth(depth: usize) -> Self {
        QosHistory::KeepLast(depth)
    }

    pub fn depth(&self) -> usize {
        match self {
            QosHistory::KeepLast(d) => *d,
            QosHistory::KeepAll => 0,
        }
    }
}

/// QoS decode errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosDecodeError {
    InvalidFormat,
    InvalidReliability,
    InvalidDurability,
    InvalidHistory,
    InvalidDuration,
    InvalidLiveliness,
}

impl Display for QosDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            QosDecodeError::InvalidFormat => write!(f, "Invalid QoS format"),
            QosDecodeError::InvalidReliability => write!(f, "Invalid reliability value"),
            QosDecodeError::InvalidDurability => write!(f, "Invalid durability value"),
            QosDecodeError::InvalidHistory => write!(f, "Invalid history value"),
            QosDecodeError::InvalidDuration => write!(f, "Invalid duration value"),
            QosDecodeError::InvalidLiveliness => write!(f, "Invalid liveliness value"),
        }
    }
}
