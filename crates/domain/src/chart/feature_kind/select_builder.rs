//! Unified select builder that handles all select types as specializations of grouped multi-select.
use super::{FeatureKind, GlyphID};
use crate::{Color, Icon};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::str::FromStr;
use typed_builder::TypedBuilder;

/// Option for select fields
#[derive(Debug, Clone, TypedBuilder, PartialEq, Serialize, Deserialize, Type)]
#[builder(field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")))]
pub struct SelectOption {
    #[builder(default, setter(skip))]
    pub id: GlyphID,
    #[builder(
      via_mutators,
      mutators(
        pub fn with_name(mut self, name: impl Into<String>) {
            self.name = name.into();
        }
      )
    )]
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<Icon>,
    #[serde(default = "Color::default")]
    pub color: Color,
}

/// Group of options for grouped select fields
#[derive(Debug, Clone, TypedBuilder, PartialEq, Serialize, Deserialize, Type)]
#[builder(field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")))]
pub struct SelectGroup {
    #[builder(default, setter(skip))]
    pub id: GlyphID,
    #[builder(
      via_mutators,
      mutators(
        pub fn with_name(mut self, name: impl Into<String>) {
            self.name = name.into();
        }
      )
    )]
    pub name: String,

    #[builder(
      via_mutators,
      mutators(
      pub fn with_description(&self, description: impl Into<String>) {
        self.description = Some(description.into());
      }
    ))]
    pub description: Option<String>,

    #[builder(
        via_mutators,
        mutators(
            pub fn with_option(mut self, option: impl Into<SelectOption>) {
                let option: SelectOption = option.into();
                self.options.insert(option.id.clone(), option);
            }
            pub fn with_options(mut self, options: Vec<SelectOption>) {
                self.options = options.into_iter().map(|option| (option.id.clone(), option)).collect();
            }
        )
    )]
    pub options: IndexMap<GlyphID, SelectOption>,

    /// Maximum selections allowed (None = unlimited, Some(1) = single select)
    #[builder(
      via_mutators(init = None),
      mutators(
        /// Allow multiple selections (converts to multi-select)
        pub fn with_unlimited_selections(mut self) {
          self.max_selections = None; // Unlimited
        }

        /// Set maximum number of selections
        pub fn with_max_selections(mut self, max: u32) {
          self.max_selections = Some(max);
        }

        /// Allow single selection only (default)
        pub fn with_single_selection(mut self) {
          self.max_selections = Some(1);
        }
      )
    )]
    pub max_selections: Option<u32>,

    #[builder(
      via_mutators,
      mutators(
        pub fn with_default_option(mut self, option: GlyphID) {
          self.default_options.push(option);
        }

        pub fn with_default_options(mut self, options: Vec<GlyphID>) {
          self.default_options = options
        }
      )
    )]
    pub default_options: Vec<GlyphID>,
}

impl SelectOption {
    /// Give this option a deterministic id. Call before inserting it anywhere
    /// keyed by id, including a group's default.
    pub fn keyed(mut self, key: &str) -> Self {
        self.id = GlyphID::derived(key);
        self
    }
}

impl SelectGroup {
    /// Give this group a deterministic id. Call before adding it to a select.
    pub fn keyed(mut self, key: &str) -> Self {
        self.id = GlyphID::derived(key);
        self
    }
}

impl From<String> for SelectOption {
    fn from(name: String) -> Self {
        SelectOption::builder()
            .with_name(name.clone())
            .with_color(Color::sample_from_string(&name))
            .build()
    }
}

/// Unified builder for all select types (single, multi, grouped, grouped-multi)
/// All are internally represented as grouped multi-select with different constraints
#[derive(Debug, Clone, TypedBuilder, PartialEq, Serialize, Deserialize)]
#[builder(field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")))]
pub struct Select {
    /// Groups of options - single group for simple selects, multiple for grouped.
    ///
    /// `IndexMap` (not `HashMap`) is critical here: it preserves insertion
    /// order so `with_group` calls come out in the order the chart author
    /// wrote them. That order propagates through the `From<Select> for
    /// FeatureKind` conversion and on into the JSON serialization, so the
    /// frontend renders groups in the intended order (e.g. Status: Pending
    /// → In Progress → Finished).
    #[builder(
      setter(!strip_option),
      via_mutators(init = IndexMap::new()),
      mutators(
        #[mutator(requires=[default_group])]
        pub fn with_group(mut self, group: SelectGroup, is_default: Option<bool>) {
          let is_default = is_default.unwrap_or(false);
          let group_id = group.id.clone();

          if is_default || self.default_group.is_none() {
            self.default_group = Some(group_id.clone());
          }
          if self.groups.contains_key(&group_id) {
            return;
          }

          self.groups.insert(group_id, group);
        }

        // #[mutator(requires=[default_group])]
        // pub fn with_option(mut self, group_id: Option<GlyphID>, option: SelectOption, is_default: Option<bool>) {
        //   let id_to_use = if let Some(id) = group_id {
        //       id
        //   } else {
        //       if self.default_group.is_none() {
        //           self.default_group = Some(GlyphID::from_str("dflt").unwrap());
        //       }
        //       self.default_group.as_ref().unwrap().clone()
        //   };

        //   let group = self.groups.entry(id_to_use.clone()).or_insert_with(|| {
        //       SelectGroup {
        //           id: id_to_use,
        //           name: "Options".to_string(),
        //           description: None,
        //           options: Vec::new(),
        //       }
        //   });

        //   group.options.push(option);
        // }
      )
    )]
    pub groups: IndexMap<GlyphID, SelectGroup>,

    /// Maximum selections allowed (None = unlimited, Some(1) = single select)
    #[builder(
      via_mutators(init = None),
      mutators(
        /// Allow multiple selections (converts to multi-select)
        pub fn with_unlimited_selections(mut self) {
          self.max_selections = None; // Unlimited
        }

        /// Set maximum number of selections
        pub fn with_max_selections(mut self, max: u32) {
          self.max_selections = Some(max);
        }

        /// Allow single selection only (default)
        pub fn with_single_selection(mut self) {
          self.max_selections = Some(1);
        }
      )
    )]
    pub max_selections: Option<u32>,
    /// Default selected options
    #[builder(setter(!strip_option))]
    pub default_options: Vec<GlyphID>,
    /// Whether to flatten groups when there's only one group (for simple select appearance)
    #[builder(setter(!strip_option))]
    pub flatten_single_group: bool,
    #[builder(via_mutators)]
    pub default_group: Option<GlyphID>,
}

impl Default for Select {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl From<Select> for FeatureKind {
    fn from(s: Select) -> Self {
        match (s.groups.len(), s.max_selections, s.flatten_single_group) {
            // No groups, single selection = Empty Select
            (0, Some(1), _) => FeatureKind::Select {
                options: IndexMap::new(),
                default_option: s.default_options.first().cloned(),
            },
            // No groups, multiple selection = Empty Multi Select
            (0, max_sel, _) if max_sel != Some(1) => FeatureKind::MultiSelect {
                options: IndexMap::new(),
                max_selections: max_sel,
                default_options: s.default_options,
            },
            // Single group, single selection, flatten = Simple Select
            (1, Some(1), true) => {
                let group = s.groups.into_values().next().unwrap();
                FeatureKind::Select {
                    options: group.options,
                    default_option: s.default_options.first().cloned(),
                }
            }
            // Single group, multiple selection (None or Some(n>1)), flatten = Multi Select
            (1, max_sel, true) if max_sel != Some(1) => {
                let group = s.groups.into_values().next().unwrap();
                FeatureKind::MultiSelect {
                    options: group.options,
                    max_selections: max_sel,
                    default_options: s.default_options,
                }
            }
            // Multiple groups, single selection = Grouped Select
            (_, Some(1), _) => FeatureKind::GroupedSelect {
                groups: s.groups.into_iter().collect(),
                default_option: s.default_options.first().cloned(),
            },
            // Multiple groups, multiple selection = Grouped Multi Select
            (_, max_sel, _) => {
                // Convert default_options to HashMap format for GroupedMultiSelect
                let default_map = IndexMap::new(); // Would need more complex logic to map options to groups
                FeatureKind::GroupedMultiSelect {
                    groups: s.groups.into_iter().collect(),
                    max_selections: max_sel,
                    default_options: default_map,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use FeatureKind::{
        GroupedMultiSelect as FGroupedMultiSelect, GroupedSelect as FGroupedSelect,
        MultiSelect as FMultiSelect, Select as FSelect,
    };

    #[test]
    fn test_simple_select() {
        let select: FeatureKind = {
            let options = SelectGroup::builder()
                .with_option(SelectOption::builder().with_name("Option 1").build())
                .with_option(SelectOption::builder().with_name("Option 2").build())
                .build();
            Select::builder()
                .with_group(options, true.into())
                .with_single_selection()
                .with_flatten_single_group(true)
                .build()
        }
        .into();

        if let FSelect { options, .. } = select {
            assert_eq!(options.len(), 2);
            assert_eq!(options[0].name, "Option 1");
        } else {
            panic!("Expected Select variant, got {select:?}");
        }
    }

    #[test]
    fn test_multi_select() {
        let multi_select = {
            let options: SelectGroup = SelectGroup::builder()
                .with_option("Option 1".to_string())
                .with_option("Option 2".to_string())
                .build();
            Select::builder()
                .with_group(options, true.into())
                .with_max_selections(3)
                .with_flatten_single_group(true)
                .build()
        }
        .into();

        if let FMultiSelect {
            options,
            max_selections,
            ..
        } = multi_select
        {
            assert_eq!(options.len(), 2);
            assert_eq!(max_selections, Some(3));
        } else {
            panic!("Expected MultiSelect variant");
        }
    }

    #[test]
    fn test_grouped_select() {
        let grouped_select = {
            let group1 = {
                let md_opt = SelectOption::from("Medium".to_string());
                let id = md_opt.id.clone();
                SelectGroup::builder()
                    .with_name("Priority")
                    .with_option("High".to_string())
                    .with_option(md_opt)
                    .with_option("Low".to_string())
                    .with_default_option(id)
                    .build()
            };

            let group2 = SelectGroup::builder()
                .with_name("Status")
                .with_option("Active".to_string())
                .with_option("Inactive".to_string())
                .build();

            Select::builder()
                .with_group(group1, None)
                .with_group(group2, None)
                .with_single_selection()
                .build()
        }
        .into();

        if let FGroupedSelect { groups, .. } = grouped_select {
            assert_eq!(groups.len(), 2);
        } else {
            panic!("Expected GroupedSelect variant, got {grouped_select:?}");
        }
    }

    #[test]
    fn test_grouped_multi_select_with_chained_option() {
        let grouped_multi_select: FeatureKind = {
            let group1 = SelectGroup::builder()
                .with_name("Colors")
                .with_option("Red".to_string())
                .with_option("Blue".to_string())
                .build();

            let group2 = SelectGroup::builder()
                .with_name("Sizes")
                .with_option("Small".to_string())
                .with_option("Large".to_string())
                .build();

            Select::builder()
                .with_group(group1, None)
                .with_group(group2, None)
                .with_unlimited_selections()
                .build()
        }
        .into();

        if let FGroupedMultiSelect { groups, .. } = grouped_multi_select {
            assert_eq!(groups.len(), 2);
        } else {
            panic!("Expected GroupedMultiSelect variant");
        }
    }
}
