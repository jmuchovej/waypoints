//! Identifier newtypes. Each is distinct at the type level so a
//! Notice can never be confused with an ACL record (or a blob, a member, …)
//! no matter how similar the underlying bytes look.

use core::fmt;

use serde::{Deserialize, Serialize};

/// 32-byte identifiers rendered as hex. `Debug` shows a shortened prefix so
/// log lines stay readable; `Display` is the full hex form.
macro_rules! byte_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
            derive_more::From,
            derive_more::Into,
        )]
        pub struct $name([u8; 32]);

        impl $name {
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                for byte in self.0 {
                    write!(f, "{byte:02x}")?;
                }
                Ok(())
            }
        }

        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(f, concat!(stringify!($name), "("))?;
                for byte in &self.0[..4] {
                    write!(f, "{byte:02x}")?;
                }
                write!(f, "…)")
            }
        }
    };
}

pub(crate) use byte_id;

byte_id! {
    /// Content address of a Notice: BLAKE3 of the encoded [`SignedNotice`]
    /// bytes. Derived, never assigned.
    ///
    /// [`SignedNotice`]: crate::SignedNotice
    NoticeID
}

byte_id! {
    /// Content address of an ACL record.
    RecordID
}

byte_id! {
    /// A Territory's identity: hash of its root ACL record — thus
    /// independent of any Fleet.
    TerritoryID
}

byte_id! {
    /// BLAKE3 content hash of an uploaded file's plaintext. Marks
    /// embed this; Stations replicate blobs by per-Territory bindings on it.
    BlobHash
}

byte_id! {
    /// A member's identity: their Ed25519 identity public key. The key
    /// *is* the user ID; there is no account behind it.
    MemberID
}

impl From<&ed25519_dalek::VerifyingKey> for MemberID {
    fn from(key: &ed25519_dalek::VerifyingKey) -> Self {
        Self(key.to_bytes())
    }
}

/// A Track is identified by its root Notice.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    Serialize,
    Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
pub struct TrackID(NoticeID);

impl TrackID {
    pub const fn root_notice(&self) -> NoticeID {
        self.0
    }
}

/// One generation of a Territory's read key. Monotonically
/// increasing; bumped when a member is removed.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    Serialize,
    Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
pub struct EpochID(u64);

impl EpochID {
    /// The epoch every Territory starts in.
    pub const ZERO: Self = Self(0);

    pub const fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_full_hex_and_debug_is_shortened() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0xab;
        bytes[31] = 0x01;
        let id = NoticeID::from_bytes(bytes);
        let hex = id.to_string();
        assert_eq!(hex.len(), 64);
        assert!(hex.starts_with("ab000000"));
        assert!(hex.ends_with("01"));
        assert_eq!(format!("{id:?}"), "NoticeID(ab000000…)");
    }

    #[test]
    fn ids_are_distinct_types_but_stable_bytes() {
        let bytes = [7u8; 32];
        assert_eq!(NoticeID::from_bytes(bytes).as_bytes(), &bytes);
        assert_eq!(RecordID::from_bytes(bytes).as_bytes(), &bytes);
    }

    #[test]
    fn epoch_zero_and_next() {
        assert_eq!(EpochID::ZERO.next(), EpochID::from(1));
        assert!(EpochID::ZERO < EpochID::from(1));
    }
}
