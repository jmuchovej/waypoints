use domain::Json;
use schemars::{JsonSchema, generate::SchemaSettings};

/// The Draft-07 identifier a generated options schema declares.
pub const DRAFT_07: &str = "http://json-schema.org/draft-07/schema#";

/// The options schema for a configuration type: JSON Schema Draft-07,
/// generated. Draft-07 because that is what transload validates with — a
/// 2020-12 document would pass the meta-schema check while its newer
/// keywords went silently unread.
pub fn options<C: JsonSchema>() -> Json {
    let schema = SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<C>();
    serde_json::to_value(schema).expect("a schema is a JSON value")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct Config {
        title_from: TitleFrom,
        #[serde(default)]
        fail_on_entry: Option<String>,
    }

    #[derive(Deserialize, JsonSchema)]
    #[serde(rename_all = "lowercase")]
    #[allow(dead_code)]
    enum TitleFrom {
        Heading,
        Filename,
    }

    #[test]
    fn the_schema_declares_draft_07_and_names_the_fields() {
        let schema = options::<Config>();
        assert_eq!(schema["$schema"], DRAFT_07);
        let properties = schema["properties"].as_object().unwrap();
        let mut names: Vec<_> = properties.keys().cloned().collect();
        names.sort();
        assert_eq!(names, ["fail_on_entry", "title_from"]);
        assert_eq!(schema["required"], serde_json::json!(["title_from"]));
    }
}
