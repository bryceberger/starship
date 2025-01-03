use serde::{Deserialize, Serialize};

static DEFAULT_TEMPLATE: &str = r#"
separate(" ",
  change_id.shortest(6),
  branches.map(|x| if(
    x.name().substr(0, 20).starts_with(x.name()),
    x.name().substr(0, 20),
    x.name().substr(0, 19) ++ "…")
  ).join(" "),
  if(
    description.first_line().substr(0, 24).starts_with(description.first_line()),
    description.first_line().substr(0, 24),
    description.first_line().substr(0, 23) ++ "…"
  ),
  if(conflict, "conflict"),
  if(divergent, "divergent"),
  if(hidden, "hidden"),
)"#;

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(
    feature = "config-schema",
    derive(schemars::JsonSchema),
    schemars(deny_unknown_fields)
)]
#[serde(default)]
pub struct JujutsuConfig<'a> {
    pub format: &'a str,
    pub symbol: &'a str,
    pub template: &'a str,
    pub disabled: bool,
    pub detect_folders: Vec<&'a str>,
}

impl Default for JujutsuConfig<'_> {
    fn default() -> Self {
        Self {
            format: "$symbol $commit_info ",
            symbol: "jj",
            template: DEFAULT_TEMPLATE,
            disabled: false,
            detect_folders: vec![".jj"],
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(
    feature = "config-schema",
    derive(schemars::JsonSchema),
    schemars(deny_unknown_fields)
)]
#[serde(default)]
pub struct JujutsuDiffConfig<'a> {
    pub added_style: &'a str,
    pub deleted_style: &'a str,
    pub only_nonzero_diffs: bool,
    pub format: &'a str,
    pub disabled: bool,
    pub detect_folders: Vec<&'a str>,
}

impl Default for JujutsuDiffConfig<'_> {
    fn default() -> Self {
        Self {
            added_style: "bold green",
            deleted_style: "bold red",
            format: "([+$added]($added_style) )([-$deleted]($deleted_style) )",
            disabled: false,
            detect_folders: vec![".jj"],
            only_nonzero_diffs: true,
        }
    }
}
