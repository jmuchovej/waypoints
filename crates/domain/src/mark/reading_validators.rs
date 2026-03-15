//! Validation functions for [`Reading`] based on the [`FeatureKind`].
//!
//! Contains all validation logic for different [`Feature`][F] kinds.
//!
//! [F]: crate::chart::Feature

#![allow(clippy::collapsible_if)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::iter_over_hash_type)]

use super::Reading;
use crate::chart::{FeatureID, FeatureKind};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use std::str::FromStr;
use validator::{ValidateEmail, ValidateUrl};

/// Validation helper for text features
pub fn text(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::Text = feature else {
        unreachable!("validate_text called with non-Text feature")
    };

    match reading {
        Reading::Text(_) => Ok(()),
        Reading::Null => Ok(()),
        _ => Err("Expected text value".to_string()),
    }
}

/// Validation helper for number features
pub fn number(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::Number {
        min,
        max,
        step: _,
        precision: _,
        format: _,
    } = feature
    else {
        unreachable!("validate_number called with non-Number feature")
    };

    match reading {
        Reading::Number(num) => {
            if let Some(min_val) = min {
                if num < min_val {
                    return Err(format!("Number must be at least {}", min_val));
                }
            }
            if let Some(max_val) = max {
                if num > max_val {
                    return Err(format!("Number must be no more than {}", max_val));
                }
            }
            Ok(())
        }
        Reading::Null => Ok(()),
        _ => Err("Expected number value".to_string()),
    }
}

/// Validation helper for checkbox features
pub fn checkbox(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::Checkbox = feature else {
        unreachable!("validate_checkbox called with non-Checkbox feature")
    };

    match reading {
        Reading::Bool(_) => Ok(()),
        Reading::Null => Ok(()),
        _ => Err("Expected boolean value".to_string()),
    }
}

/// Helper function for date validation
fn single_date(
    dt: NaiveDateTime,
    min_date: &Option<NaiveDate>,
    max_date: &Option<NaiveDate>,
    min_time: &Option<NaiveTime>,
    max_time: &Option<NaiveTime>,
) -> Result<(), String> {
    let min_time = min_time.unwrap_or(NaiveTime::from_str("00:00").unwrap());
    let max_time = max_time.unwrap_or(NaiveTime::from_str("23:59").unwrap());
    let min_dt = min_date.unwrap_or(dt.date()).and_time(min_time);
    let max_dt = max_date.unwrap_or(dt.date()).and_time(max_time);
    datetime(dt, Some(min_dt), Some(max_dt))
}

fn date_range(
    dt_s: NaiveDateTime,
    dt_e: NaiveDateTime,
    min_date: &Option<NaiveDate>,
    max_date: &Option<NaiveDate>,
    min_time: &Option<NaiveTime>,
    max_time: &Option<NaiveTime>,
) -> Result<(), String> {
    let min_time = min_time.unwrap_or(NaiveTime::from_str("00:00").unwrap());
    let max_time = max_time.unwrap_or(NaiveTime::from_str("23:59").unwrap());

    // Validate start date
    let min_dt_s = min_date.unwrap_or(dt_s.date()).and_time(min_time);
    let max_dt_s = max_date.unwrap_or(dt_s.date()).and_time(max_time);
    datetime(dt_s, Some(min_dt_s), Some(max_dt_s))?;

    // Validate end date
    let min_dt_e = min_date.unwrap_or(dt_e.date()).and_time(min_time);
    let max_dt_e = max_date.unwrap_or(dt_e.date()).and_time(max_time);
    datetime(dt_e, Some(min_dt_e), Some(max_dt_e))?;

    // Validate start <= end
    if dt_s >= dt_e {
        return Err("Start date must be before or equal to end date".to_string());
    }

    Ok(())
}

fn datetime(
    dt: NaiveDateTime,
    min_dt: Option<NaiveDateTime>,
    max_dt: Option<NaiveDateTime>,
) -> Result<(), String> {
    if let Some(min) = min_dt {
        if dt < min {
            return Err(format!("Date must be after {}", min));
        }
    }

    if let Some(max) = max_dt {
        if dt > max {
            return Err(format!("Date must be before {}", max));
        }
    }

    Ok(())
}

/// Validation helper for date features
pub fn date(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::Date {
        min_date,
        max_date,
        min_time,
        max_time,
        timezone: _,
        granularity: _,
    } = feature
    else {
        unreachable!("validate_date called with non-Date feature")
    };

    match reading {
        Reading::Date(dt) => single_date(*dt, min_date, max_date, min_time, max_time),
        Reading::DateRange(dt_s, dt_e) => {
            date_range(*dt_s, *dt_e, min_date, max_date, min_time, max_time)
        }
        Reading::Null => Ok(()),
        _ => Err("Expected date or date range value".to_string()),
    }
}

/// Validation helper for email features
pub fn email(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::Email = feature else {
        unreachable!("validate_email called with non-Email feature")
    };

    match reading {
        Reading::Email(email) => {
            if email.validate_email() {
                Ok(())
            } else {
                Err("Invalid email format".to_string())
            }
        }
        Reading::Null => Ok(()),
        _ => Err("Expected email value".to_string()),
    }
}

/// Validation helper for phone features
pub fn phone(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::Phone = feature else {
        unreachable!("validate_phone called with non-Phone feature")
    };

    match reading {
        Reading::Phone(_, _) => {
            // TODO implement phone-number validation
            Ok(())
        }
        Reading::Null => Ok(()),
        _ => Err("Expected a phone number!".to_string()),
    }
}

/// Validation helper for URL features
pub fn url(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::URL = feature else {
        unreachable!("validate_url called with non-URL feature")
    };

    match reading {
        Reading::URL(url) => {
            if url.validate_url() {
                Ok(())
            } else {
                Err("Invalid URL format".to_string())
            }
        }
        Reading::Null => Ok(()),
        _ => Err("Expected URL value".to_string()),
    }
}

/// Validation helper for reference features
pub fn reference(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::Reference {
        allowed_types: _,
        max_references,
        display_field: _,
        display_mode: _,
        filter_criteria: _,
    } = feature
    else {
        unreachable!("validate_reference called with non-Reference feature")
    };

    match reading {
        Reading::References(refs) => {
            if let Some(max) = max_references {
                if refs.len() > *max as usize {
                    return Err(format!("Too many references (max: {})", max));
                }
            }
            Ok(())
        }
        Reading::Null => Ok(()),
        _ => Err("Expected reference value".to_string()),
    }
}

/// Validation helper for select features
pub fn select(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::Select {
        options,
        default_option: _,
    } = feature
    else {
        unreachable!("validate_select called with non-Select feature")
    };

    match reading {
        Reading::Select(selected) => match options.get(selected) {
            Some(_) => Ok(()),
            None => Err(format!("Invalid option: {}", selected)),
        },
        Reading::Null => Ok(()),
        _ => Err("Expected single selection".to_string()),
    }
}

/// Validation helper for multi-select features
pub fn multi_select(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::MultiSelect {
        options,
        max_selections,
        default_options: _,
    } = feature
    else {
        unreachable!("validate_multi_select called with non-MultiSelect feature")
    };

    match reading {
        Reading::MultiSelect(selected) => {
            if let Some(max) = max_selections {
                if selected.len() > *max as usize {
                    return Err(format!("Too many selections (max: {})", max));
                }
            }
            for sel in selected {
                if options.get(sel).is_none() {
                    return Err(format!("Invalid option: {}", sel));
                }
            }
            Ok(())
        }
        Reading::Null => Ok(()),
        _ => Err("Expected multiple selections".to_string()),
    }
}

/// Validation helper for grouped select features
pub fn grouped_select(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::GroupedSelect {
        groups,
        default_option: _,
    } = feature
    else {
        unreachable!("validate_grouped_select called with non-GroupedSelect feature")
    };

    match reading {
        Reading::Select(selected) => {
            for (_, group) in groups.iter() {
                if group.options.get(selected).is_some() {
                    return Ok(());
                }
            }
            Err(format!("Invalid option: {}", selected))
        }
        Reading::Null => Ok(()),
        _ => Err("Expected grouped selection".to_string()),
    }
}

/// Validation helper for grouped multi-select features
pub fn grouped_multi_select(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::GroupedMultiSelect {
        groups,
        max_selections,
        default_options: _,
    } = feature
    else {
        unreachable!("validate_grouped_multi_select called with non-GroupedMultiSelect feature")
    };

    match reading {
        Reading::MultiSelect(selected) => {
            if let Some(max) = max_selections {
                if selected.len() > *max as usize {
                    return Err(format!("Too many selections (max: {})", max));
                }
            }
            let mut found = 0;
            for sel in selected {
                for (_, group) in groups.iter() {
                    if group.options.get(sel).is_some() {
                        found += 1;
                        break;
                    }
                }
            }
            if found != selected.len() {
                return Err(format!("Invalid options: {:?}", selected));
            }
            Ok(())
        }
        Reading::GroupedSelect(grouped_map) => {
            for (group_id, options) in grouped_map {
                if let Some(group) = groups.get(group_id) {
                    for option_id in options {
                        if !group.options.values().any(|opt| &opt.id == option_id) {
                            return Err(format!(
                                "Invalid option {} in group {}",
                                option_id, group_id
                            ));
                        }
                    }
                } else {
                    return Err(format!("Invalid group: {}", group_id));
                }
            }
            Ok(())
        }
        Reading::Null => Ok(()),
        _ => Err("Expected grouped multiple selections".to_string()),
    }
}

/// Validation helper for formula features
pub fn formula(reading: &Reading, feature: &FeatureKind) -> Result<(), String> {
    let FeatureKind::Formula = feature else {
        unreachable!("validate_formula called with non-Formula feature")
    };

    match reading {
        Reading::Formula(_) => Ok(()),
        Reading::Null => Ok(()),
        // Formulas can evaluate to any type
        _ => Ok(()),
    }
}
