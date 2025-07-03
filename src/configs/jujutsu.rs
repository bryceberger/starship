use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(
    feature = "config-schema",
    derive(schemars::JsonSchema),
    schemars(deny_unknown_fields)
)]
#[serde(default)]
pub struct JujutsuCommitConfig<'a> {
    pub change_id_length: usize,
    pub format: &'a str,
    pub description_empty: &'a str,
    pub style_prefix: &'a str,
    pub style_rest: &'a str,
    pub style_description: &'a str,
    pub style_description_empty: &'a str,
}

impl Default for JujutsuCommitConfig<'_> {
    fn default() -> Self {
        Self {
            change_id_length: 8,
            format:
                "[$prefix]($style_prefix)[$rest]($style_rest) [$description]($style_description)",
            description_empty: "(no description)",
            style_prefix: "bold purple",
            style_rest: "bright-black",
            style_description: "",
            style_description_empty: "green",
        }
    }
}
