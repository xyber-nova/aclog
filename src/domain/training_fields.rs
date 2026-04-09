use std::collections::HashMap;

use crate::domain::record::TrainingFields;

pub const NOTE_FIELD: &str = "Note";
pub const MISTAKES_FIELD: &str = "Mistakes";
pub const INSIGHT_FIELD: &str = "Insight";
pub const CONFIDENCE_FIELD: &str = "Confidence";
pub const SOURCE_KIND_FIELD: &str = "Source-Kind";
pub const TIME_SPENT_FIELD: &str = "Time-Spent";

const NOTE_FIELD_ALIASES: &[&str] = &[NOTE_FIELD, "笔记"];
const MISTAKES_FIELD_ALIASES: &[&str] = &[MISTAKES_FIELD, "卡点"];
const INSIGHT_FIELD_ALIASES: &[&str] = &[INSIGHT_FIELD, "收获"];
const CONFIDENCE_FIELD_ALIASES: &[&str] = &[CONFIDENCE_FIELD, "熟练度"];
const SOURCE_KIND_FIELD_ALIASES: &[&str] = &[SOURCE_KIND_FIELD, "完成方式"];
const TIME_SPENT_FIELD_ALIASES: &[&str] = &[TIME_SPENT_FIELD, "训练耗时"];

pub fn normalize_optional_training_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-")
        .map(ToString::to_string)
}

pub fn parse_training_fields(fields: &HashMap<String, String>) -> TrainingFields {
    TrainingFields {
        note: normalize_optional_training_text(first_field_value(fields, NOTE_FIELD_ALIASES)),
        mistakes: normalize_optional_training_text(first_field_value(
            fields,
            MISTAKES_FIELD_ALIASES,
        )),
        insight: normalize_optional_training_text(first_field_value(fields, INSIGHT_FIELD_ALIASES)),
        confidence: normalize_optional_training_text(first_field_value(
            fields,
            CONFIDENCE_FIELD_ALIASES,
        )),
        source_kind: normalize_optional_training_text(first_field_value(
            fields,
            SOURCE_KIND_FIELD_ALIASES,
        )),
        time_spent: normalize_optional_training_text(first_field_value(
            fields,
            TIME_SPENT_FIELD_ALIASES,
        )),
    }
}

pub fn format_training_fields(training: &TrainingFields) -> Vec<(&'static str, &str)> {
    let mut formatted = Vec::new();
    if let Some(value) = training.note.as_deref() {
        formatted.push((NOTE_FIELD, value));
    }
    if let Some(value) = training.mistakes.as_deref() {
        formatted.push((MISTAKES_FIELD, value));
    }
    if let Some(value) = training.insight.as_deref() {
        formatted.push((INSIGHT_FIELD, value));
    }
    if let Some(value) = training.confidence.as_deref() {
        formatted.push((CONFIDENCE_FIELD, value));
    }
    if let Some(value) = training.source_kind.as_deref() {
        formatted.push((SOURCE_KIND_FIELD, value));
    }
    if let Some(value) = training.time_spent.as_deref() {
        formatted.push((TIME_SPENT_FIELD, value));
    }
    formatted
}

fn first_field_value<'a>(fields: &'a HashMap<String, String>, aliases: &[&str]) -> Option<&'a str> {
    aliases
        .iter()
        .find_map(|alias| fields.get(*alias).map(String::as_str))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        NOTE_FIELD, format_training_fields, normalize_optional_training_text, parse_training_fields,
    };
    use crate::domain::record::TrainingFields;

    #[test]
    fn normalize_optional_training_text_treats_blank_values_as_absent() {
        assert_eq!(normalize_optional_training_text(Some("  ")), None);
        assert_eq!(normalize_optional_training_text(Some("-")), None);
        assert_eq!(
            normalize_optional_training_text(Some(" needs review ")),
            Some("needs review".to_string())
        );
    }

    #[test]
    fn parse_and_format_training_fields_use_shared_field_names() {
        let mut fields = HashMap::new();
        fields.insert(NOTE_FIELD.to_string(), "remember this".to_string());

        let parsed = parse_training_fields(&fields);
        assert_eq!(
            parsed,
            TrainingFields {
                note: Some("remember this".to_string()),
                ..TrainingFields::default()
            }
        );
        assert_eq!(
            format_training_fields(&parsed),
            vec![(NOTE_FIELD, "remember this")]
        );
    }
}
