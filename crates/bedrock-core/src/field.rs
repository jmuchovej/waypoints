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
/// Every field type declares its [`FieldKind`] and knows how to round-trip
/// through a [`StoredValue`].
pub trait Field: Sized {
    /// How this type maps into storage.
    const KIND: FieldKind;

    /// Whether the storage column admits `NULL`. True only for `Option<_>`.
    const NULLABLE: bool = false;

    /// Project a value into its storage wire form. Infallible for every scalar.
    fn encode(&self) -> StoredValue;

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

            fn encode(&self) -> StoredValue {
                StoredValue::Integer(i64::from(*self))
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

            fn encode(&self) -> StoredValue {
                StoredValue::Real(f64::from(*self))
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

    fn encode(&self) -> StoredValue {
        StoredValue::Integer(i64::from(*self))
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

    fn encode(&self) -> StoredValue {
        StoredValue::Text(self.clone())
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

    fn encode(&self) -> StoredValue {
        StoredValue::Blob(self.clone())
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Blob(b) => Ok(b),
            other => Err(DecodeError::type_mismatch("Vec<u8>", &other)),
        }
    }
}

impl Field for Uuid {
    const KIND: FieldKind = FieldKind::Column(ColumnType::Uuid);

    fn encode(&self) -> StoredValue {
        StoredValue::Text(self.to_string())
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

    fn encode(&self) -> StoredValue {
        StoredValue::Text(self.format(DATE_FORMAT).to_string())
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

    fn encode(&self) -> StoredValue {
        StoredValue::Text(self.format(TIME_FORMAT).to_string())
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

    fn encode(&self) -> StoredValue {
        StoredValue::Text(self.format(TIMESTAMP_FORMAT).to_string())
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

    fn encode(&self) -> StoredValue {
        match self {
            Some(value) => value.encode(),
            None => StoredValue::Null,
        }
    }

    fn decode(value: StoredValue) -> Result<Self, DecodeError> {
        match value {
            StoredValue::Null => Ok(None),
            other => T::decode(other).map(Some),
        }
    }
}

/// Failure to reconstruct a Rust value from its stored wire form.
///
/// Encoding is currently infallible for every [`Field`] impl, so only the
/// decode direction is fallible: the wire form arrives untyped from the engine
/// and may not match what the target type expects.
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
        let decoded = T::decode(value.encode()).expect("decode");
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
        assert_eq!(Option::<i32>::None.encode(), StoredValue::Null);
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
