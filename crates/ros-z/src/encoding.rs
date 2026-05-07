use std::fmt;

/// Encoding marker for typed ros-z messages.
///
/// This is intentionally a single-variant enum. CDR is the native wire
/// encoding; future codecs should live in explicit adapter crates rather than
/// implicit MIME-style negotiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Encoding {
    #[default]
    Cdr,
}

impl Encoding {
    pub const fn cdr() -> Self {
        Encoding::Cdr
    }

    pub fn mime_type(&self) -> &'static str {
        "application/cdr"
    }

    pub fn to_zenoh_encoding(&self) -> zenoh::bytes::Encoding {
        zenoh::bytes::Encoding::from(self.mime_type())
    }
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CDR")
    }
}
