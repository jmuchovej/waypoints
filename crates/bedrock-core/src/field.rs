use core::fmt::Display;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::value::{ColumnType, StoredValue};

/// How a Rust type maps into bedrock storage.
///
/// The two ends of the graduated-typing spectrum: a scalar that promotes to a
/// native, indexable [`Column`](FieldKind::Column), or a composite that stays a
/// freeform [`Document`](FieldKind::Document).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    /// A single-valued type stored in a native, promotable (indexable) column.
    Column(ColumnType),
    /// A composite type (struct, enum, map, sequence) stored as a `jsonb` document.
    Document,
}

/// A type that can occupy a field in a bedrock collection.
///
/// A proc macro sees tokens, not resolved types — it cannot know that `MarkId`
/// is a `Uuid` newtype and therefore a scalar column. Classification lives here
/// instead, at the type level: every field type declares its [`FieldKind`] and
/// knows how to round-trip through a [`StoredValue`]. A field whose type does
/// not implement `Field` is a compile error, so strictness falls out of the
/// trait system for free.
pub trait Field: Sized {
    /// How this type maps into storage.
    const KIND: FieldKind;

    /// Whether the storage column admits `NULL`. True only for `Option<_>`.
    const NULLABLE: bool = false;

    /// Project a value into its storage wire form.
    ///
    /// Infallible for scalars; document fields serialize through serde and may
    /// fail, which is why the signature is fallible.
    fn encode(&self) -> Result<StoredValue, EncodeError>;

    /// Reconstruct a value from its storage wire form.
    fn decode(value: StoredValue) -> Result<Self, DecodeError>;
}

// chrono <-> text formats. Encode and decode share each format string, so the
// round-trip is inverse by construction. `NaiveDateTime`'s own `Display`/`FromStr`
// disagree on the date/time separator (space vs `T`), which is exactly why we
// pin explicit formats rather than lean on them. The space-separated timestamp
// form also matches turso's engine-canonical rendering.
const DATE_FORMAT: &str = "%Y-%m-%d";
const TIME_FORMAT: &str = "%H:%M:%S%.f";
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.f";

macro_rules! impl_field_integer {
    ($($ty:ty),* $(,)?) => {$(
        impl Field for $ty {
            const KIND: FieldKind = FieldKind::Column(ColumnType::Integer);

            fn encode(&self) -> Result<StoredValue, EncodeError> {
                Ok(StoredValue::Integer(i64::from(*self)))
            }

            fn decode(value: StoredValue) -> Result<Self, DecodeError> {
                match value {
                    StoredValue::Integer(n) => <$ty>::try_from(n).map_err(|_| {
                        DecodeError::IntegerOverflow { value: n, target: stringify!($ty) }
                    }),
                    other => Err(DecodeError::type_mismatch(stringify!($ty), &other)),
                }
            }
        }
    )*};
}

// `u64`/`u128`/`i128` are deliberately omitted: they do not fit SQLite's signed
// 64-bit INTEGER. Wrap them, or store as a document.
impl_field_integer!(i8, i16, i32, i64, u8, u16, u32);

macro_rules! impl_field_real {
    ($($ty:ty),* $(,)?) => {$(
        impl Field for $ty {
            const KIND: FieldKind = FieldKind::Column(ColumnType::Real);

            fn encode(&self) -> Result<StoredValue, EncodeError> {
                Ok(StoredValue::Real(f64::from(*self)))
            }

            fn decode(value: StoredValue) -> Result<Self, DecodeError> {
                match value {
                    StoredValue::Real(n) => Ok(n as $ty),
                    other => Err(DecodeError::type_mismatch(stringify!($ty), &other)),
                }
            }
        }
    )*};
}

impl_field_real!(f32, f64);

impl Field for bool {
    const KIND: FieldKind = FieldKind::Column(ColumnType::Boolean);

    fn encode(&self) -> Result<StoredValue, EncodeError> {
        Ok(StoredValue::Integer(i64::from(*self)))
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Integer(n) => Ok(n != 0),
            other => Err(DecodeError::type_mismatch("bool", &other)),
        }
    }
}

impl Field for String {
    const KIND: FieldKind = FieldKind::Column(ColumnType::Text);

    fn encode(&self) -> Result<StoredValue, EncodeError> {
        Ok(StoredValue::Text(self.clone()))
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Text(s) => Ok(s),
            other => Err(DecodeError::type_mismatch("String", &other)),
        }
    }
}

impl Field for Vec<u8> {
    const KIND: FieldKind = FieldKind::Column(ColumnType::Blob);

    fn encode(&self) -> Result<StoredValue, EncodeError> {
        Ok(StoredValue::Blob(self.clone()))
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Blob(b) => Ok(b),
            other => Err(DecodeError::type_mismatch("Vec<u8>", &other)),
        }
    }
}

/// Fixed-size byte arrays are BLOB columns with an enforced length —
/// content hashes and public keys as native binary primary keys or
/// indexed columns, no text encoding detour. Decoding a blob of any other
/// length is a type mismatch, not a truncation.
impl<const N: usize> Field for [u8; N] {
    const KIND: FieldKind = FieldKind::Column(ColumnType::Blob);

    fn encode(&self) -> Result<StoredValue, EncodeError> {
        Ok(StoredValue::Blob(self.to_vec()))
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Blob(b) if b.len() == N => {
                let mut out = [0u8; N];
                out.copy_from_slice(&b);
                Ok(out)
            }
            other => Err(DecodeError::type_mismatch(
                core::any::type_name::<Self>(),
                &other,
            )),
        }
    }
}

/// Arbitrary JSON is a document — a genuinely schemaless field (e.g. a
/// metadata bag), stored as `jsonb` without a Rust type to shape it. Unlike a
/// derived [`Document`](FieldKind::Document), it needs no serde round-trip: the
/// value already *is* the wire form.
impl Field for serde_json::Value {
    const KIND: FieldKind = FieldKind::Document;

    fn encode(&self) -> Result<StoredValue, EncodeError> {
        Ok(StoredValue::Json(self.clone()))
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Json(v) => Ok(v),
            other => Err(DecodeError::type_mismatch("serde_json::Value", &other)),
        }
    }
}

/// An `IndexMap` is a document — round-tripped through serde as `jsonb`. A
/// generic impl so a typed, order-preserving map (e.g. a Chart's `features`)
/// can be a collection field directly, with no newtype wrapper.
impl<K, V, S> Field for indexmap::IndexMap<K, V, S>
where
    indexmap::IndexMap<K, V, S>: serde::Serialize + serde::de::DeserializeOwned,
{
    const KIND: FieldKind = FieldKind::Document;

    fn encode(&self) -> Result<StoredValue, EncodeError> {
        serde_json::to_value(self)
            .map(StoredValue::Json)
            .map_err(|e| EncodeError::Serialize {
                target: "IndexMap",
                message: e.to_string(),
            })
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Json(v) => serde_json::from_value(v).map_err(|e| DecodeError::Document {
                target: "IndexMap",
                message: e.to_string(),
            }),
            other => Err(DecodeError::type_mismatch("IndexMap", &other)),
        }
    }
}

impl Field for Uuid {
    const KIND: FieldKind = FieldKind::Column(ColumnType::Uuid);

    fn encode(&self) -> Result<StoredValue, EncodeError> {
        Ok(StoredValue::Text(self.to_string()))
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Text(s) => {
                Uuid::parse_str(&s).map_err(|e| DecodeError::parse("Uuid", s, e))
            }
            other => Err(DecodeError::type_mismatch("Uuid", &other)),
        }
    }
}

impl Field for NaiveDate {
    const KIND: FieldKind = FieldKind::Column(ColumnType::Date);

    fn encode(&self) -> Result<StoredValue, EncodeError> {
        Ok(StoredValue::Text(self.format(DATE_FORMAT).to_string()))
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Text(s) => NaiveDate::parse_from_str(&s, DATE_FORMAT)
                .map_err(|e| DecodeError::parse("NaiveDate", s, e)),
            other => Err(DecodeError::type_mismatch("NaiveDate", &other)),
        }
    }
}

impl Field for NaiveTime {
    const KIND: FieldKind = FieldKind::Column(ColumnType::Time);

    fn encode(&self) -> Result<StoredValue, EncodeError> {
        Ok(StoredValue::Text(self.format(TIME_FORMAT).to_string()))
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Text(s) => NaiveTime::parse_from_str(&s, TIME_FORMAT)
                .map_err(|e| DecodeError::parse("NaiveTime", s, e)),
            other => Err(DecodeError::type_mismatch("NaiveTime", &other)),
        }
    }
}

impl Field for NaiveDateTime {
    const KIND: FieldKind = FieldKind::Column(ColumnType::Timestamp);

    fn encode(&self) -> Result<StoredValue, EncodeError> {
        Ok(StoredValue::Text(self.format(TIMESTAMP_FORMAT).to_string()))
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Text(s) => NaiveDateTime::parse_from_str(&s, TIMESTAMP_FORMAT)
                .map_err(|e| DecodeError::parse("NaiveDateTime", s, e)),
            other => Err(DecodeError::type_mismatch("NaiveDateTime", &other)),
        }
    }
}

/// `Option<T>` delegates to `T` and makes the column nullable. An absent value
/// stores as `NULL` and reassembles as `None`.
impl<T: Field> Field for Option<T> {
    const KIND: FieldKind = T::KIND;
    const NULLABLE: bool = true;

    fn encode(&self) -> Result<StoredValue, EncodeError> {
        match self {
            Some(value) => value.encode(),
            None => Ok(StoredValue::Null),
        }
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Null => Ok(None),
            other => T::decode(other).map(Some),
        }
    }
}

/// Failure to project a Rust value into its stored wire form.
///
/// Scalar encoding is infallible; only document fields — which serialize through
/// serde — can fail here.
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    /// A document field could not be serialized to its stored form.
    #[error("could not serialize {target} to a document: {message}")]
    Serialize {
        target: &'static str,
        message: String,
    },
}

/// Failure to reconstruct a Rust value from its stored wire form.
///
/// The wire form arrives untyped from the engine and may not match what the
/// target type expects.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// The stored value's variant did not match the target type.
    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },

    /// A stored integer did not fit the target integer type.
    #[error("integer {value} out of range for {target}")]
    IntegerOverflow { value: i64, target: &'static str },

    /// A stored text value could not be parsed into the target type.
    #[error("could not parse {target} from {input:?}: {message}")]
    Parse {
        target: &'static str,
        input: String,
        message: String,
    },

    /// A stored document could not be deserialized into the target type.
    #[error("could not deserialize {target} from its stored document: {message}")]
    Document {
        target: &'static str,
        message: String,
    },

    /// A row carried the wrong number of columns for the target collection.
    #[error("collection {collection} expected {expected} columns, found {found}")]
    ColumnCount {
        collection: &'static str,
        expected: usize,
        found: usize,
    },
}

impl DecodeError {
    pub(crate) fn type_mismatch(expected: &'static str, found: &StoredValue) -> Self {
        Self::TypeMismatch {
            expected,
            found: found.type_name(),
        }
    }

    pub(crate) fn parse(
        target: &'static str,
        input: impl Into<String>,
        message: impl Display,
    ) -> Self {
        Self::Parse {
            target,
            input: input.into(),
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime};
    use uuid::Uuid;

    use super::*;

    fn roundtrip<T>(value: T)
    where
        T: Field + Clone + PartialEq + core::fmt::Debug,
    {
        let encoded = value.encode().expect("encode");
        let decoded = T::decode(encoded).expect("decode");
        assert_eq!(value, decoded);
    }

    #[test]
    fn scalars_roundtrip() {
        roundtrip(true);
        roundtrip(false);
        roundtrip(-42i32);
        roundtrip(7u32);
        roundtrip(i64::MIN);
        roundtrip(3.5f64);
        roundtrip(1.25f32);
        roundtrip(String::from("hello"));
        roundtrip(vec![1u8, 2, 3]);
    }

    #[test]
    fn uuid_is_text_column() {
        assert_eq!(Uuid::KIND, FieldKind::Column(ColumnType::Uuid));
        roundtrip(Uuid::from_u128(0x0102_0304_0506_0708_090a_0b0c_0d0e_0f10));
    }

    #[test]
    fn chrono_roundtrips_including_fractional_seconds() {
        roundtrip(NaiveDate::from_ymd_opt(2026, 7, 10).unwrap());
        roundtrip(NaiveTime::from_hms_opt(12, 34, 56).unwrap());
        roundtrip(NaiveTime::from_hms_milli_opt(12, 34, 56, 500).unwrap());
        let stamp = NaiveDate::from_ymd_opt(2026, 7, 10)
            .unwrap()
            .and_hms_milli_opt(12, 34, 56, 500)
            .unwrap();
        assert_eq!(
            NaiveDateTime::KIND,
            FieldKind::Column(ColumnType::Timestamp)
        );
        roundtrip(stamp);
    }

    #[test]
    fn option_is_nullable_and_delegates_kind() {
        const { assert!(<Option<i32>>::NULLABLE) };
        assert_eq!(<Option<i32>>::KIND, i32::KIND);

        roundtrip(Some(5i32));
        roundtrip(Option::<i32>::None);
        assert_eq!(
            Option::<i32>::None.encode().expect("encode"),
            StoredValue::Null
        );
    }

    #[test]
    fn json_value_is_a_document_and_roundtrips() {
        assert_eq!(<serde_json::Value>::KIND, FieldKind::Document);

        // A nested, heterogeneous blob — the schemaless (`extra`) case.
        let value = serde_json::json!({ "a": 1, "b": [true, null, "x"], "c": { "d": 2.5 } });
        let encoded = value.encode().expect("encode");
        assert!(matches!(encoded, StoredValue::Json(_)));
        assert_eq!(<serde_json::Value>::decode(encoded).expect("decode"), value);

        // Wrapped in `Option`, it's a nullable document column.
        const { assert!(<Option<serde_json::Value>>::NULLABLE) };
        assert_eq!(<Option<serde_json::Value>>::KIND, FieldKind::Document);
    }

    #[test]
    fn integer_overflow_is_reported() {
        let too_big = StoredValue::Integer(i64::from(i32::MAX) + 1);
        let err = i32::decode(too_big).unwrap_err();
        assert!(matches!(
            err,
            DecodeError::IntegerOverflow { target: "i32", .. }
        ));
    }

    #[test]
    fn type_mismatch_is_reported() {
        let err = i32::decode(StoredValue::Text("nope".into())).unwrap_err();
        assert!(matches!(
            err,
            DecodeError::TypeMismatch {
                expected: "i32",
                found: "text"
            }
        ));
    }
}
