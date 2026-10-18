//! Independently authored action catalog and ordered action-group execution.
//! References: https://docs.mp3tag.de/actions/ and its individual action pages.
//! Native effects are explicit: a caller must execute them or return an error.
use crate::batch_errors::{ErrorCause, IndexedCause};
use crate::{
    script::{evaluate, evaluate_filename_with_context, Action, EvalContext},
    tags::Fields,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const LIMIT: usize = 1_048_576;
/// Separates several values of one field in a Format Value result.
const MULTI_VALUE_SEPARATOR: &str = "\\\\";
pub const ACTION_CATALOG: [(&str, &str); 18] = [
    ("adjust_cover", "Adjust Cover"),
    ("case_conversion", "Case Conversion"),
    ("convert_codepage", "Convert Codepage"),
    ("export", "Export"),
    ("export_cover", "Export Cover to File"),
    ("format_value", "Format Value"),
    ("generate_playlists", "Generate Playlists"),
    ("guess_values", "Guess Values"),
    ("import_cover", "Import Cover from File"),
    ("import_text_file", "Import Text File"),
    ("merge_duplicate_fields", "Merge Duplicate Fields"),
    ("remove_duplicate_fields", "Remove Duplicate Fields"),
    ("remove_fields", "Remove Fields"),
    ("remove_fields_except", "Remove Fields Except"),
    ("replace", "Replace"),
    ("regex_replace", "Replace with Regular Expression"),
    ("set_cover_properties", "Set Cover Properties"),
    ("split_field", "Split Field by Separator"),
];
/// Additional numeric IDs corroborated by original `.mta` reports on the
/// official forum: /t/testing-mp3tag-behaviour/42469/22. Only types 1 and 2
/// have settings verified on original action files; the settings of the
/// others follow the same layout (`F` and numbered settings in dialog order)
/// and are recorded in docs/verification/script-export.md.
pub const MTA_TYPE_REGISTRY: [(u32, &str); 15] = [
    (1, "Case Conversion"),
    (2, "Replace"),
    (4, "Replace with Regular Expression"),
    (5, "Format Value"),
    (6, "Remove Duplicate Fields"),
    (7, "Guess Values"),
    (8, "Merge Duplicate Fields"),
    (9, "Remove Fields"),
    (10, "Remove Fields Except"),
    (11, "Convert Codepage"),
    (12, "Import Cover from File"),
    (13, "Export Cover to File"),
    (14, "Import Text File"),
    (15, "Export"),
    (16, "Split Field by Separator"),
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaseMode {
    Lower,
    Mixed,
    Sentence,
    Upper,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverImageFormat {
    Original,
    Jpeg,
    Png,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverSizeMode {
    Maximum,
    Minimum,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverScaleMode {
    Proportional,
    Square,
    CropSquare,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverResize {
    pub pixels: u32,
    pub size_mode: CoverSizeMode,
    pub scale_mode: CoverScaleMode,
}

/// One variant for every action type listed in the official documentation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionSpec {
    AdjustCover {
        format: CoverImageFormat,
        quality: u8,
        resize: Option<CoverResize>,
    },
    CaseConversion {
        field: String,
        mode: CaseMode,
        word_separators: String,
    },
    ConvertCodepage {
        field: String,
        codepage: String,
    },
    Export {
        template: String,
        filename_template: String,
        append: bool,
        partition_template: Option<String>,
    },
    ExportCover {
        filename_template: String,
        picture_type: Option<String>,
        export_duplicates: bool,
    },
    FormatValue {
        field: String,
        template: String,
    },
    GeneratePlaylists {
        filename_template: String,
        partition_template: String,
    },
    GuessValues {
        source: String,
        pattern: String,
    },
    ImportCover {
        filename_template: String,
        /// "Import cover as" type; `None` imports a front cover.
        #[serde(default)]
        picture_type: Option<String>,
        /// "Delete existing covers" before embedding the imported image.
        #[serde(default)]
        delete_existing: bool,
    },
    ImportTextFile {
        field: String,
        filename_template: String,
    },
    MergeDuplicateFields {
        field: String,
        separator: String,
    },
    RemoveDuplicateFields {
        field: String,
        only_same_content: bool,
    },
    RemoveFields {
        fields: Vec<String>,
    },
    RemoveFieldsExcept {
        fields: Vec<String>,
    },
    Replace {
        field: String,
        from: String,
        to: String,
        whole_word: bool,
        case_sensitive: bool,
    },
    RegexReplace {
        field: String,
        pattern: String,
        replacement: String,
        case_sensitive: bool,
    },
    SetCoverProperties {
        picture_type: Option<String>,
        description: Option<String>,
        only_picture_type: Option<String>,
    },
    SplitField {
        field: String,
        separator: String,
    },
    /// Preserve unresolved imported settings rather than silently skipping them.
    UnresolvedMta {
        type_id: String,
        properties: BTreeMap<String, String>,
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionGroup {
    pub name: String,
    pub enabled: bool,
    pub actions: Vec<ActionSpec>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectionPreset {
    pub name: String,
    pub enabled_groups: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionLibrary {
    pub groups: Vec<ActionGroup>,
    pub presets: Vec<SelectionPreset>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionState {
    pub fields: Fields,
    pub context: Fields,
}
impl ActionState {
    pub fn evaluation_fields(&self) -> Fields {
        let mut fields = self.context.clone();
        fields.extend(self.fields.clone());
        fields
    }
}
/// Where a new file or folder name came from, which decides how the host
/// turns it into a path.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NameOrigin {
    /// The current name changed by Case Conversion, Replace and similar
    /// actions. `\` and `/` separate folders.
    #[default]
    Edited,
    /// A format string evaluated in filename mode (Format Value, Tag → Tag):
    /// literal separators create folders, tag values had theirs replaced, and
    /// a name already ending in the file's extension does not get it twice.
    Template,
    /// Text captured by Guess Values, used like a tag value: separators and
    /// invalid characters are replaced, and the extension is not doubled.
    Value,
}

/// How an action wants a format string evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateMode {
    Text,
    /// Filename mode, as Convert → Tag - Filename evaluates.
    Filename,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionEffect {
    RenameFile {
        filename: String,
        includes_extension: bool,
        #[serde(default)]
        origin: NameOrigin,
    },
    RenameDirectory {
        directory: String,
        #[serde(default)]
        origin: NameOrigin,
    },
    RemoveCovers,
    ImportCover {
        filename: String,
        picture_type: Option<String>,
        delete_existing: bool,
    },
    ExportCover {
        filename: String,
        picture_type: Option<String>,
        export_duplicates: bool,
    },
    AdjustCover {
        format: CoverImageFormat,
        quality: u8,
        resize: Option<CoverResize>,
    },
    SetCoverProperties {
        picture_type: Option<String>,
        description: Option<String>,
        only_picture_type: Option<String>,
    },
    ImportTextFile {
        field: String,
        filename: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BatchActionEffect {
    Export {
        template: String,
        filename_template: String,
        append: bool,
        partition_template: Option<String>,
    },
    GeneratePlaylists {
        filename_template: String,
        partition_template: String,
    },
}
/// The native adapter supplies file, image, text and set-of-files execution.
/// Imports and renames must update state before the next action is evaluated.
pub trait ActionHost {
    /// Override with actual application EvalContext (columns, paths, locale,
    /// Windows ANSI codepage), rebuilding tag context after each action.
    fn evaluate(&mut self, template: &str, state: &ActionState) -> Result<String, String> {
        evaluate(template, &state.evaluation_fields())
    }
    /// Evaluate a `_FILENAME`/`_DIRECTORY` target in filename mode.
    fn evaluate_filename(&mut self, template: &str, state: &ActionState) -> Result<String, String> {
        evaluate_filename_with_context(template, &EvalContext::new(&state.evaluation_fields()))
    }
    fn execute(&mut self, effect: &ActionEffect, state: &mut ActionState) -> Result<(), String>;
    fn execute_batch(
        &mut self,
        effect: &BatchActionEffect,
        states: &mut [ActionState],
    ) -> Result<(), String>;
}

fn checked(value: String) -> Result<String, String> {
    if value.len() > LIMIT {
        Err("Action output limit exceeded".into())
    } else {
        Ok(value)
    }
}
fn actual_key(fields: &Fields, name: &str) -> String {
    fields
        .keys()
        .find(|key| key.eq_ignore_ascii_case(name))
        .cloned()
        .unwrap_or_else(|| name.to_uppercase())
}
fn is_cover(name: &str) -> bool {
    name.eq_ignore_ascii_case("COVER") || name.eq_ignore_ascii_case("PICTURE")
}
fn metadata_keys(fields: &Fields, selector: &str) -> Result<Vec<String>, String> {
    if selector.trim().is_empty() {
        return Err("Action field must not be empty".into());
    }
    if selector.eq_ignore_ascii_case("_TAG") || selector.eq_ignore_ascii_case("_ALL") {
        return Ok(fields.keys().cloned().collect());
    }
    if selector.starts_with('_') {
        return Err(format!(
            "Field selector {selector} requires a filename, directory or field-name action"
        ));
    }
    Ok(vec![actual_key(fields, selector)])
}
/// The values a Format Value result stores. `\\` separates several values,
/// as in the Tag Panel; each is trimmed and empty ones are dropped.
fn format_values(result: &str) -> Vec<String> {
    if result.contains(MULTI_VALUE_SEPARATOR) {
        result
            .split(MULTI_VALUE_SEPARATOR)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect()
    } else if result.is_empty() {
        vec![]
    } else {
        vec![result.to_owned()]
    }
}
fn put(fields: &mut Fields, key: String, values: Vec<String>) {
    if values.is_empty() {
        fields.remove(&key);
    } else {
        fields.insert(key, values);
    }
}
fn case_value(value: &str, mode: CaseMode, separators: &str) -> String {
    match mode {
        CaseMode::Lower => value.to_lowercase(),
        CaseMode::Upper => value.to_uppercase(),
        CaseMode::Sentence => {
            let mut chars = value.chars();
            chars
                .next()
                .map(|first| {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                })
                .unwrap_or_default()
        }
        CaseMode::Mixed => {
            let mut start = true;
            let mut out = String::new();
            for c in value.chars() {
                if start {
                    out.extend(c.to_uppercase())
                } else {
                    out.extend(c.to_lowercase())
                }
                start = c.is_whitespace() || separators.contains(c);
            }
            out
        }
    }
}
#[cfg(test)]
fn replace_values(
    values: &[String],
    from: &str,
    to: &str,
    whole_word: bool,
    case_sensitive: bool,
) -> Result<Vec<String>, String> {
    replace_values_diagnostic(values, from, to, whole_word, case_sensitive)
        .map_err(|error| error.to_string())
}

fn replace_values_diagnostic(
    values: &[String],
    from: &str,
    to: &str,
    whole_word: bool,
    case_sensitive: bool,
) -> Result<Vec<String>, ErrorCause> {
    if from.is_empty() {
        return Err(ErrorCause::owned("Replacement search must not be empty"));
    }
    let re = regex::RegexBuilder::new(&regex::escape(from))
        .case_insensitive(!case_sensitive)
        .size_limit(8 * LIMIT)
        .build()
        .map_err(ErrorCause::external)?;
    let boundary = |c: char| c.is_whitespace() || ".,;:!?".contains(c);
    values
        .iter()
        .map(|value| {
            let mut out = String::new();
            let mut end = 0;
            for found in re.find_iter(value) {
                let left = value[..found.start()].chars().next_back();
                let right = value[found.end()..].chars().next();
                if whole_word && (!left.is_none_or(boundary) || !right.is_none_or(boundary)) {
                    continue;
                }
                out.push_str(&value[end..found.start()]);
                out.push_str(to);
                end = found.end();
            }
            out.push_str(&value[end..]);
            checked(out).map_err(ErrorCause::owned_text)
        })
        .collect()
}
fn regex_values(
    values: &[String],
    pattern: &str,
    replacement: &str,
    case_sensitive: bool,
) -> Result<Vec<String>, String> {
    regex_values_diagnostic(values, pattern, replacement, case_sensitive)
        .map_err(|error| error.to_string())
}

fn regex_values_diagnostic(
    values: &[String],
    pattern: &str,
    replacement: &str,
    case_sensitive: bool,
) -> Result<Vec<String>, ErrorCause> {
    let expression =
        regex_compat::Regex::new(pattern, case_sensitive).map_err(ErrorCause::external)?;
    values
        .iter()
        .map(|value| {
            expression
                .replace(value, replacement)
                .map_err(ErrorCause::external)
                .and_then(|value| checked(value).map_err(ErrorCause::owned_text))
        })
        .collect()
}
fn transform_diagnostic<F>(
    state: &mut ActionState,
    selector: &str,
    filename_extension: bool,
    mut operation: F,
) -> Result<Vec<ActionEffect>, ErrorCause>
where
    F: FnMut(&[String]) -> Result<Vec<String>, ErrorCause>,
{
    let mut effects = Vec::new();
    let mut fields = state.fields.clone();
    if selector.eq_ignore_ascii_case("_FIELDNAME") {
        // The displayed tag (`_TAG_READ`) is the one whose fields are edited;
        // `_TAG` lists every tag in the file.
        let format = state
            .context
            .get("_TAG_READ")
            .or_else(|| state.context.get("_TAG"))
            .and_then(|v| v.first())
            .map(String::as_str)
            .unwrap_or("");
        if !crate::tags::keeps_field_names(format) {
            return Err(ErrorCause::owned(
                "Field-name actions require an APEv2 or VorbisComment tag",
            ));
        }
        let mut renamed = Fields::new();
        for (key, values) in &state.fields {
            let names = operation(std::slice::from_ref(key))?;
            if names.len() != 1 || names[0].is_empty() {
                return Err(ErrorCause::owned(
                    "Field-name transformation must produce one nonempty name",
                ));
            }
            let name = &names[0];
            if renamed.keys().any(|key| key.eq_ignore_ascii_case(name)) {
                return Err(ErrorCause::owned_text(format!(
                    "Field-name transformation collision: {name}"
                )));
            }
            renamed.insert(name.clone(), values.clone());
        }
        fields = renamed;
    } else {
        let includes_tags = selector.eq_ignore_ascii_case("_TAG")
            || selector.eq_ignore_ascii_case("_ALL")
            || !selector.starts_with('_');
        if includes_tags {
            for key in metadata_keys(&fields, selector).map_err(ErrorCause::owned_text)? {
                if let Some(values) = fields.get(&key) {
                    let values = operation(values)?;
                    put(&mut fields, key, values);
                }
            }
        }
        if selector.eq_ignore_ascii_case("_FILENAME") || selector.eq_ignore_ascii_case("_ALL") {
            let source = if filename_extension {
                "_FILENAME_EXT"
            } else {
                "_FILENAME"
            };
            let values = state.context.get(source).ok_or_else(|| {
                ErrorCause::owned_text(format!("Missing {source} action context"))
            })?;
            let value = operation(values)?;
            if value.len() != 1 || value[0].is_empty() {
                return Err(ErrorCause::owned(
                    "Filename transformation must produce one nonempty name",
                ));
            }
            effects.push(ActionEffect::RenameFile {
                filename: value[0].clone(),
                includes_extension: filename_extension,
                origin: NameOrigin::Edited,
            });
        } else if selector.eq_ignore_ascii_case("_DIRECTORY") {
            let values = state
                .context
                .get("_DIRECTORY")
                .ok_or_else(|| ErrorCause::owned("Missing _DIRECTORY action context"))?;
            let value = operation(values)?;
            if value.len() != 1 || value[0].is_empty() {
                return Err(ErrorCause::owned(
                    "Directory transformation must produce one nonempty name",
                ));
            }
            effects.push(ActionEffect::RenameDirectory {
                directory: value[0].clone(),
                origin: NameOrigin::Edited,
            });
        } else if selector.starts_with('_')
            && !selector.eq_ignore_ascii_case("_TAG")
            && !selector.eq_ignore_ascii_case("_FIELDNAME")
        {
            return Err(ErrorCause::owned_text(format!(
                "Unsupported action selector {selector}"
            )));
        }
    }
    state.fields = fields;
    Ok(effects)
}

/// Inspect the same field placeholders used by Guess Values without source data.
pub fn guess_value_targets(pattern: &str) -> Result<Vec<String>, String> {
    let (_, names) = guessing_pattern(pattern)?;
    Ok(names.into_iter().filter(|name| name != "DUMMY").collect())
}

fn guessing_pattern(pattern: &str) -> Result<(regex::Regex, Vec<String>), String> {
    guessing_pattern_diagnostic(pattern).map_err(|error| error.to_string())
}

fn guessing_pattern_diagnostic(pattern: &str) -> Result<(regex::Regex, Vec<String>), ErrorCause> {
    if pattern.len() > LIMIT {
        return Err(ErrorCause::owned("Guess Values input limit exceeded"));
    }
    let mut expression = String::from("\\A");
    let mut names = Vec::new();
    let mut rest = pattern;
    let mut prior_capture = false;
    while let Some(start) = rest.find('%') {
        let literal = &rest[..start];
        if prior_capture && literal.is_empty() {
            return Err(ErrorCause::owned(
                "Adjacent guessing placeholders are ambiguous",
            ));
        }
        expression.push_str(&regex::escape(literal));
        rest = &rest[start + 1..];
        let end = rest
            .find('%')
            .ok_or_else(|| ErrorCause::owned("Unterminated guessing placeholder"))?;
        let name = &rest[..end];
        if name.is_empty() {
            return Err(ErrorCause::owned("Empty guessing placeholder"));
        }
        names.push(name.to_uppercase());
        expression.push_str("(.*?)");
        rest = &rest[end + 1..];
        prior_capture = true;
    }
    if names.is_empty() {
        return Err(ErrorCause::owned(
            "Guessing pattern needs at least one field placeholder",
        ));
    }
    expression.push_str(&regex::escape(rest));
    expression.push_str("\\z");
    let expression = regex::RegexBuilder::new(&expression)
        .dot_matches_new_line(true)
        .size_limit(8 * LIMIT)
        .build()
        .map_err(ErrorCause::external)?;
    Ok((expression, names))
}

/// Match an entire source string without filename extension or path processing.
pub fn guess_values(source: &str, pattern: &str) -> Result<Fields, String> {
    try_guess_values(source, pattern)?
        .ok_or_else(|| "Source does not completely match the guessing pattern".into())
}

/// Like [`guess_values`], with a source that does not match the pattern as
/// `None`: the action then leaves the file unchanged, like Mp3tag.
pub fn try_guess_values(source: &str, pattern: &str) -> Result<Option<Fields>, String> {
    try_guess_values_diagnostic(source, pattern).map_err(|error| error.to_string())
}

fn try_guess_values_diagnostic(source: &str, pattern: &str) -> Result<Option<Fields>, ErrorCause> {
    if source.len() > LIMIT {
        return Err(ErrorCause::owned("Guess Values input limit exceeded"));
    }
    let (expression, names) = guessing_pattern_diagnostic(pattern)?;
    let Some(captures) = expression.captures(source) else {
        return Ok(None);
    };
    let mut result = Fields::new();
    for (index, name) in names.into_iter().enumerate() {
        if name != "DUMMY" {
            result
                .entry(name)
                .or_default()
                .push(captures[index + 1].into());
        }
    }
    Ok(Some(result))
}

fn decode_codepage(value: &str, codepage: &str) -> Result<String, String> {
    decode_codepage_diagnostic(value, codepage).map_err(|error| error.to_string())
}

fn decode_codepage_diagnostic(value: &str, codepage: &str) -> Result<String, ErrorCause> {
    let normalized = codepage.trim().to_ascii_lowercase();
    let normalized = normalized.strip_prefix("cp").unwrap_or(&normalized);
    // Numeric identifiers follow Microsoft's Code Page Identifiers table.
    let label = match normalized {
        "65001" => "utf-8",
        "1200" => "utf-16le",
        "1201" => "utf-16be",
        "1250" => "windows-1250",
        "1251" => "windows-1251",
        "1252" => "windows-1252",
        "1253" => "windows-1253",
        "1254" => "windows-1254",
        "1255" => "windows-1255",
        "1256" => "windows-1256",
        "1257" => "windows-1257",
        "1258" => "windows-1258",
        "866" => "ibm866",
        "874" => "windows-874",
        "932" => "shift_jis",
        "936" => "gbk",
        "949" => "euc-kr",
        "950" => "big5",
        "10000" => "macintosh",
        "10007" => "x-mac-cyrillic",
        "20866" => "koi8-r",
        "21866" => "koi8-u",
        "20932" | "51932" => "euc-jp",
        "51949" => "euc-kr",
        "54936" => "gb18030",
        "28591" => "iso-8859-1",
        "28592" => "iso-8859-2",
        "28593" => "iso-8859-3",
        "28594" => "iso-8859-4",
        "28595" => "iso-8859-5",
        "28596" => "iso-8859-6",
        "28597" => "iso-8859-7",
        "28598" => "iso-8859-8",
        "28603" => "iso-8859-13",
        "28605" => "iso-8859-15",
        "20127" => "us-ascii",
        other => other,
    };
    // Preserve legacy byte values. Windows-1252 glyphs above U+00FF are encoded
    // back to their byte only when they have a lossless mapping.
    let mut bytes = Vec::new();
    for c in value.chars() {
        if u32::from(c) <= 255 {
            bytes.push(c as u8);
        } else {
            let text = c.to_string();
            let (encoded, _, lossy) = encoding_rs::WINDOWS_1252.encode(&text);
            if lossy {
                return Err(ErrorCause::owned_text(format!(
                    "Character {value:?} has no legacy byte representation",
                    value = c
                )));
            }
            bytes.extend_from_slice(&encoded);
        }
    }
    // WHATWG aliases ISO-8859-1 and ASCII to Windows-1252; Windows codepage
    // semantics require their actual byte mappings instead.
    if ["iso-8859-1", "latin1", "latin-1"].contains(&label) {
        return Ok(bytes.into_iter().map(char::from).collect());
    }
    if ["us-ascii", "ascii"].contains(&label) {
        if bytes.iter().any(|byte| *byte > 127) {
            return Err(ErrorCause::owned(
                "Invalid non-ASCII byte for US-ASCII codepage",
            ));
        }
        return String::from_utf8(bytes).map_err(ErrorCause::external);
    }
    // DOS/OEM pages other than 866 have no WHATWG encoding.
    if let Some(decoded) = label
        .parse::<u16>()
        .ok()
        .and_then(|page| crate::oem_codepages::decode(page, &bytes))
    {
        return decoded.map_err(|byte| {
            ErrorCause::owned_text(format!(
                "Byte 0x{byte:02X} is undefined in codepage {codepage}"
            ))
        });
    }
    let encoding = encoding_rs::Encoding::for_label(label.as_bytes())
        .ok_or_else(|| ErrorCause::owned_text(format!("Unsupported codepage {codepage}")))?;
    encoding
        .decode_without_bom_handling_and_without_replacement(&bytes)
        .map(|value| value.into_owned())
        .ok_or_else(|| {
            ErrorCause::owned_text(format!("Invalid byte sequence for codepage {codepage}"))
        })
}

pub fn validate_action(action: &ActionSpec) -> Result<(), String> {
    match action {
        ActionSpec::UnresolvedMta {
            type_id, reason, ..
        } => Err(format!(
            "Unresolved imported action type {type_id}: {reason}"
        )),
        ActionSpec::AdjustCover {
            quality, resize, ..
        } if *quality > 100 || resize.as_ref().is_some_and(|s| s.pixels == 0) => {
            Err("Cover quality must be 0–100 and resize pixels must be positive".into())
        }
        ActionSpec::Replace { from, .. } if from.is_empty() => {
            Err("Replacement search must not be empty".into())
        }
        ActionSpec::SplitField { separator, .. } if separator.is_empty() => {
            Err("Split separator must not be empty".into())
        }
        ActionSpec::ImportTextFile { field, .. }
        | ActionSpec::FormatValue { field, .. }
        | ActionSpec::CaseConversion { field, .. }
        | ActionSpec::ConvertCodepage { field, .. }
        | ActionSpec::MergeDuplicateFields { field, .. }
        | ActionSpec::RemoveDuplicateFields { field, .. }
        | ActionSpec::SplitField { field, .. }
        | ActionSpec::Replace { field, .. }
        | ActionSpec::RegexReplace { field, .. }
            if field.trim().is_empty() =>
        {
            Err("Action field must not be empty".into())
        }
        ActionSpec::RegexReplace {
            pattern,
            replacement,
            case_sensitive,
            ..
        } => regex_values(&[], pattern, replacement, *case_sensitive).map(|_| ()),
        ActionSpec::ConvertCodepage { codepage, .. } => decode_codepage("", codepage).map(|_| ()),
        ActionSpec::ImportCover { picture_type, .. }
        | ActionSpec::ExportCover { picture_type, .. } => cover_type(picture_type.as_deref()),
        ActionSpec::SetCoverProperties {
            picture_type,
            only_picture_type,
            ..
        } => cover_type(picture_type.as_deref())
            .and_then(|_| cover_type(only_picture_type.as_deref())),
        _ => Ok(()),
    }
}

pub fn validate_action_diagnostic(action: &ActionSpec) -> Result<(), ErrorCause> {
    match action {
        ActionSpec::RegexReplace {
            field,
            pattern,
            replacement,
            case_sensitive,
        } if !field.trim().is_empty() => {
            regex_values_diagnostic(&[], pattern, replacement, *case_sensitive).map(|_| ())
        }
        ActionSpec::ConvertCodepage { field, codepage } if !field.trim().is_empty() => {
            decode_codepage_diagnostic("", codepage).map(|_| ())
        }
        ActionSpec::UnresolvedMta {
            type_id, reason, ..
        } => Err(ErrorCause::message(
            "Unresolved imported action type {typeId}: {reason}",
            [("typeId", type_id.to_string()), ("reason", reason.clone())],
        )),
        _ => validate_action(action).map_err(ErrorCause::owned_text),
    }
}
fn cover_type(name: Option<&str>) -> Result<(), String> {
    name.map(crate::picture_type_from_name)
        .transpose()
        .map(|_| ())
}

/// Apply one action's metadata changes and return mandatory native effects.
pub fn apply_action_spec(
    state: &mut ActionState,
    action: &ActionSpec,
) -> Result<Vec<ActionEffect>, String> {
    apply_action_spec_with_evaluator(state, action, &mut |template, state| {
        evaluate(template, &state.evaluation_fields())
    })
}

pub fn apply_action_spec_with_evaluator<F>(
    state: &mut ActionState,
    action: &ActionSpec,
    evaluate: &mut F,
) -> Result<Vec<ActionEffect>, String>
where
    F: FnMut(&str, &ActionState) -> Result<String, String>,
{
    apply_action_spec_with_modes(state, action, &mut |template, state, mode| match mode {
        TemplateMode::Text => evaluate(template, state),
        TemplateMode::Filename => {
            evaluate_filename_with_context(template, &EvalContext::new(&state.evaluation_fields()))
        }
    })
}

/// Like [`apply_action_spec_with_evaluator`], with the evaluator told whether a
/// template is evaluated as text or in filename mode.
pub fn apply_action_spec_with_modes<F>(
    state: &mut ActionState,
    action: &ActionSpec,
    evaluate_mode: &mut F,
) -> Result<Vec<ActionEffect>, String>
where
    F: FnMut(&str, &ActionState, TemplateMode) -> Result<String, String>,
{
    apply_action_spec_with_modes_diagnostic(state, action, &mut |template, state, mode| {
        evaluate_mode(template, state, mode).map_err(ErrorCause::external)
    })
    .map_err(|error| error.to_string())
}

pub fn apply_action_spec_with_modes_diagnostic<F>(
    state: &mut ActionState,
    action: &ActionSpec,
    evaluate_mode: &mut F,
) -> Result<Vec<ActionEffect>, ErrorCause>
where
    F: FnMut(&str, &ActionState, TemplateMode) -> Result<String, ErrorCause>,
{
    let mut evaluate =
        |template: &str, state: &ActionState| evaluate_mode(template, state, TemplateMode::Text);
    validate_action_diagnostic(action)?;
    match action {
        ActionSpec::CaseConversion {
            field,
            mode,
            word_separators,
        } => transform_diagnostic(state, field, false, |values| {
            values
                .iter()
                .map(|value| {
                    checked(case_value(value, *mode, word_separators))
                        .map_err(ErrorCause::owned_text)
                })
                .collect()
        }),
        ActionSpec::ConvertCodepage { field, codepage } => {
            transform_diagnostic(state, field, false, |values| {
                values
                    .iter()
                    .map(|value| {
                        decode_codepage_diagnostic(value, codepage)
                            .and_then(|value| checked(value).map_err(ErrorCause::owned_text))
                    })
                    .collect()
            })
        }
        ActionSpec::Replace {
            field,
            from,
            to,
            whole_word,
            case_sensitive,
        } => transform_diagnostic(state, field, false, |values| {
            replace_values_diagnostic(values, from, to, *whole_word, *case_sensitive)
        }),
        ActionSpec::RegexReplace {
            field,
            pattern,
            replacement,
            case_sensitive,
        } => transform_diagnostic(state, field, true, |values| {
            regex_values_diagnostic(values, pattern, replacement, *case_sensitive)
        }),
        ActionSpec::FormatValue { field, template } => {
            let renames_file = field.eq_ignore_ascii_case("_FILENAME");
            if renames_file || field.eq_ignore_ascii_case("_DIRECTORY") {
                let result = evaluate_mode(template, state, TemplateMode::Filename)?;
                if result.trim().is_empty() {
                    return Err(ErrorCause::owned(if renames_file {
                        "Filename must not be empty"
                    } else {
                        "Directory must not be empty"
                    }));
                }
                return Ok(vec![if renames_file {
                    ActionEffect::RenameFile {
                        filename: result,
                        includes_extension: false,
                        origin: NameOrigin::Template,
                    }
                } else {
                    ActionEffect::RenameDirectory {
                        directory: result,
                        origin: NameOrigin::Template,
                    }
                }]);
            }
            let result = evaluate(template, state)?;
            if field.starts_with('_') {
                return Err(ErrorCause::owned_text(format!(
                    "Unsupported Format Value target {field}"
                )));
            }
            let key = actual_key(&state.fields, field);
            put(&mut state.fields, key, format_values(&result));
            Ok(vec![])
        }
        ActionSpec::GuessValues { source, pattern } => {
            let Some(result) = try_guess_values_diagnostic(&evaluate(source, state)?, pattern)?
            else {
                return Ok(vec![]);
            };
            let mut effects = Vec::new();
            let mut fields = state.fields.clone();
            for (name, values) in result {
                if name == "_FILENAME" || name == "_DIRECTORY" {
                    if values.len() != 1 || values[0].is_empty() {
                        return Err(ErrorCause::owned_text(format!(
                            "Guess Values must produce one nonempty {name}"
                        )));
                    }
                    effects.push(if name == "_FILENAME" {
                        ActionEffect::RenameFile {
                            filename: values[0].clone(),
                            includes_extension: false,
                            origin: NameOrigin::Value,
                        }
                    } else {
                        ActionEffect::RenameDirectory {
                            directory: values[0].clone(),
                            origin: NameOrigin::Value,
                        }
                    });
                } else if name.starts_with('_') {
                    return Err(ErrorCause::owned_text(format!(
                        "Unsupported Guess Values target {name}"
                    )));
                } else {
                    let key = actual_key(&fields, &name);
                    put(&mut fields, key, values);
                }
            }
            state.fields = fields;
            Ok(effects)
        }
        ActionSpec::MergeDuplicateFields { field, separator } => {
            let mut fields = state.fields.clone();
            for key in metadata_keys(&fields, field).map_err(ErrorCause::owned_text)? {
                if let Some(values) = fields.get(&key) {
                    if values.len() > 1 {
                        let merged =
                            checked(values.join(separator)).map_err(ErrorCause::owned_text)?;
                        fields.insert(key, vec![merged]);
                    }
                }
            }
            state.fields = fields;
            Ok(vec![])
        }
        ActionSpec::RemoveDuplicateFields {
            field,
            only_same_content,
        } => {
            let mut fields = state.fields.clone();
            for key in metadata_keys(&fields, field).map_err(ErrorCause::owned_text)? {
                if let Some(values) = fields.get_mut(&key) {
                    if *only_same_content {
                        let mut seen = BTreeSet::new();
                        values.retain(|value| seen.insert(value.clone()));
                    } else {
                        values.truncate(1);
                    }
                }
            }
            state.fields = fields;
            Ok(vec![])
        }
        ActionSpec::SplitField { field, separator } => {
            let mut fields = state.fields.clone();
            for key in metadata_keys(&fields, field).map_err(ErrorCause::owned_text)? {
                if let Some(values) = fields.get(&key) {
                    let split = values
                        .iter()
                        .flat_map(|value| value.split(separator).map(|part| part.trim().to_owned()))
                        .collect();
                    fields.insert(key, split);
                }
            }
            state.fields = fields;
            Ok(vec![])
        }
        ActionSpec::RemoveFields { fields } => {
            let mut effects = Vec::new();
            for name in fields {
                if is_cover(name) {
                    if !effects.contains(&ActionEffect::RemoveCovers) {
                        effects.push(ActionEffect::RemoveCovers);
                    }
                } else {
                    state.fields.remove(&actual_key(&state.fields, name));
                }
            }
            Ok(effects)
        }
        ActionSpec::RemoveFieldsExcept { fields } => {
            let keep: BTreeSet<String> = fields.iter().map(|name| name.to_uppercase()).collect();
            state
                .fields
                .retain(|name, _| keep.contains(&name.to_uppercase()));
            Ok(if fields.iter().any(|name| is_cover(name)) {
                vec![]
            } else {
                vec![ActionEffect::RemoveCovers]
            })
        }
        ActionSpec::ImportCover {
            filename_template,
            picture_type,
            delete_existing,
        } => Ok(vec![ActionEffect::ImportCover {
            filename: evaluate_mode(filename_template, state, TemplateMode::Filename)?,
            picture_type: picture_type.clone(),
            delete_existing: *delete_existing,
        }]),
        ActionSpec::ExportCover {
            filename_template,
            picture_type,
            export_duplicates,
        } => Ok(vec![ActionEffect::ExportCover {
            filename: evaluate_mode(filename_template, state, TemplateMode::Filename)?,
            picture_type: picture_type.clone(),
            export_duplicates: *export_duplicates,
        }]),
        ActionSpec::ImportTextFile {
            field,
            filename_template,
        } => {
            if field.starts_with('_') {
                return Err(ErrorCause::owned(
                    "Import Text File requires a metadata field",
                ));
            }
            Ok(vec![ActionEffect::ImportTextFile {
                field: actual_key(&state.fields, field),
                filename: evaluate_mode(filename_template, state, TemplateMode::Filename)?,
            }])
        }
        ActionSpec::AdjustCover {
            format,
            quality,
            resize,
        } => Ok(vec![ActionEffect::AdjustCover {
            format: *format,
            quality: *quality,
            resize: resize.clone(),
        }]),
        ActionSpec::SetCoverProperties {
            picture_type,
            description,
            only_picture_type,
        } => Ok(vec![ActionEffect::SetCoverProperties {
            picture_type: picture_type.clone(),
            description: description.clone(),
            only_picture_type: only_picture_type.clone(),
        }]),
        ActionSpec::Export { .. } | ActionSpec::GeneratePlaylists { .. } => Err(ErrorCause::owned(
            "Set-of-files actions must execute through the group runner",
        )),
        ActionSpec::UnresolvedMta { .. } => unreachable!("validated above"),
    }
}

pub fn execute_groups<H: ActionHost>(
    states: &mut [ActionState],
    groups: &[ActionGroup],
    host: &mut H,
) -> Result<(), String> {
    // Unknown imported actions fail before any host effect can mutate files.
    for group in groups.iter().filter(|group| group.enabled) {
        for action in &group.actions {
            validate_action(action).map_err(|error| format!("{}: {error}", group.name))?;
        }
    }
    for group in groups.iter().filter(|group| group.enabled) {
        let mut deferred = Vec::new();
        for (index, action) in group.actions.iter().enumerate() {
            let batch = match action {
                ActionSpec::Export {
                    template,
                    filename_template,
                    append,
                    partition_template,
                } => Some(BatchActionEffect::Export {
                    template: template.clone(),
                    filename_template: filename_template.clone(),
                    append: *append,
                    partition_template: partition_template.clone(),
                }),
                ActionSpec::GeneratePlaylists {
                    filename_template,
                    partition_template,
                } => Some(BatchActionEffect::GeneratePlaylists {
                    filename_template: filename_template.clone(),
                    partition_template: partition_template.clone(),
                }),
                _ => None,
            };
            if let Some(batch) = batch {
                deferred.push((index, batch));
                continue;
            }
            for state in states.iter_mut() {
                let effects =
                    apply_action_spec_with_modes(state, action, &mut |template, state, mode| {
                        match mode {
                            TemplateMode::Text => host.evaluate(template, state),
                            TemplateMode::Filename => host.evaluate_filename(template, state),
                        }
                    })
                    .map_err(|error| format!("{} / action {}: {error}", group.name, index + 1))?;
                for effect in effects {
                    host.execute(&effect, state).map_err(|error| {
                        format!("{} / action {}: {error}", group.name, index + 1)
                    })?;
                }
            }
        }
        for (index, effect) in deferred {
            host.execute_batch(&effect, states)
                .map_err(|error| format!("{} / action {}: {error}", group.name, index + 1))?;
        }
    }
    Ok(())
}

impl ActionLibrary {
    fn ensure_name(&self, name: &str, except: Option<usize>) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Action group name must not be empty".into());
        }
        if self
            .groups
            .iter()
            .enumerate()
            .any(|(index, group)| Some(index) != except && group.name.eq_ignore_ascii_case(name))
        {
            return Err(format!("Action group already exists: {name}"));
        }
        Ok(())
    }
    pub fn add_group(&mut self, group: ActionGroup) -> Result<usize, String> {
        self.ensure_name(&group.name, None)?;
        self.groups.push(group);
        Ok(self.groups.len() - 1)
    }
    pub fn duplicate_group(&mut self, index: usize, name: String) -> Result<usize, String> {
        let mut group = self
            .groups
            .get(index)
            .ok_or("Action group index out of range")?
            .clone();
        group.name = name;
        self.add_group(group)
    }
    pub fn remove_group(&mut self, index: usize) -> Result<ActionGroup, String> {
        if index >= self.groups.len() {
            return Err("Action group index out of range".into());
        }
        let group = self.groups.remove(index);
        for preset in &mut self.presets {
            preset.enabled_groups.retain(|name| name != &group.name);
        }
        Ok(group)
    }
    pub fn rename_group(&mut self, index: usize, name: String) -> Result<(), String> {
        if index >= self.groups.len() {
            return Err("Action group index out of range".into());
        }
        self.ensure_name(&name, Some(index))?;
        let old = std::mem::replace(&mut self.groups[index].name, name.clone());
        for preset in &mut self.presets {
            for selected in &mut preset.enabled_groups {
                if *selected == old {
                    *selected = name.clone();
                }
            }
        }
        Ok(())
    }
    pub fn move_group(&mut self, from: usize, to: usize) -> Result<(), String> {
        if from >= self.groups.len() || to >= self.groups.len() {
            return Err("Action group index out of range".into());
        }
        let group = self.groups.remove(from);
        self.groups.insert(to, group);
        Ok(())
    }
    pub fn move_action(&mut self, group: usize, from: usize, to: usize) -> Result<(), String> {
        let actions = &mut self
            .groups
            .get_mut(group)
            .ok_or("Action group index out of range")?
            .actions;
        if from >= actions.len() || to >= actions.len() {
            return Err("Action index out of range".into());
        }
        let action = actions.remove(from);
        actions.insert(to, action);
        Ok(())
    }
    pub fn add_action(&mut self, group: usize, action: ActionSpec) -> Result<usize, String> {
        validate_action(&action)?;
        let actions = &mut self
            .groups
            .get_mut(group)
            .ok_or("Action group index out of range")?
            .actions;
        actions.push(action);
        Ok(actions.len() - 1)
    }
    pub fn edit_action(
        &mut self,
        group: usize,
        index: usize,
        action: ActionSpec,
    ) -> Result<(), String> {
        validate_action(&action)?;
        let old = self
            .groups
            .get_mut(group)
            .ok_or("Action group index out of range")?
            .actions
            .get_mut(index)
            .ok_or("Action index out of range")?;
        *old = action;
        Ok(())
    }
    pub fn remove_action(&mut self, group: usize, index: usize) -> Result<ActionSpec, String> {
        let actions = &mut self
            .groups
            .get_mut(group)
            .ok_or("Action group index out of range")?
            .actions;
        if index >= actions.len() {
            return Err("Action index out of range".into());
        }
        Ok(actions.remove(index))
    }
    pub fn set_group_enabled(&mut self, index: usize, enabled: bool) -> Result<(), String> {
        self.groups
            .get_mut(index)
            .ok_or("Action group index out of range")?
            .enabled = enabled;
        Ok(())
    }
    pub fn duplicate_action(&mut self, group: usize, index: usize) -> Result<usize, String> {
        let actions = &mut self
            .groups
            .get_mut(group)
            .ok_or("Action group index out of range")?
            .actions;
        let action = actions
            .get(index)
            .ok_or("Action index out of range")?
            .clone();
        actions.insert(index + 1, action);
        Ok(index + 1)
    }
    pub fn save_selection_preset(&mut self, name: String) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Preset name must not be empty".into());
        }
        let enabled_groups = self
            .groups
            .iter()
            .filter(|group| group.enabled)
            .map(|group| group.name.clone())
            .collect();
        let preset = SelectionPreset {
            name,
            enabled_groups,
        };
        if let Some(index) = self.presets.iter().position(|old| old.name == preset.name) {
            self.presets[index] = preset;
        } else {
            self.presets.push(preset);
        }
        Ok(())
    }
    pub fn apply_selection_preset(&mut self, name: &str) -> Result<(), String> {
        let preset = self
            .presets
            .iter()
            .find(|preset| preset.name == name)
            .ok_or("Selection preset not found")?;
        for group in &mut self.groups {
            group.enabled = preset.enabled_groups.contains(&group.name);
        }
        Ok(())
    }
    pub fn remove_selection_preset(&mut self, name: &str) -> Result<(), String> {
        let index = self
            .presets
            .iter()
            .position(|preset| preset.name == name)
            .ok_or("Selection preset not found")?;
        self.presets.remove(index);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionMenuSegment {
    pub label: String,
    pub mnemonic: Option<char>,
}
/// '#' gives submenu segments; '&' marks each segment's mnemonic.
pub fn group_menu_segments(name: &str) -> Vec<ActionMenuSegment> {
    name.split('#')
        .map(|part| {
            let mut chars = part.chars().peekable();
            let mut label = String::new();
            let mut mnemonic = None;
            while let Some(c) = chars.next() {
                if c == '&' {
                    if chars.peek() == Some(&'&') {
                        chars.next();
                        label.push('&');
                    } else if let Some(next) = chars.next() {
                        if mnemonic.is_none() {
                            mnemonic = Some(next.to_lowercase().next().unwrap_or(next));
                        }
                        label.push(next);
                    } else {
                        label.push(c);
                    }
                } else {
                    label.push(c);
                }
            }
            ActionMenuSegment { label, mnemonic }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MtaRecord {
    pub index: usize,
    pub properties: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MtaIssue {
    pub index: usize,
    pub type_id: String,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MtaImport {
    pub group: ActionGroup,
    pub records: Vec<MtaRecord>,
    pub issues: Vec<MtaIssue>,
}

fn mta_value(value: &str) -> Result<String, String> {
    let mut out = String::new();
    let chars: Vec<char> = value.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '\\' && index + 1 < chars.len() {
            if chars[index + 1] == '\\' {
                out.push('\\');
                index += 2;
                continue;
            }
            let count = match chars[index + 1] {
                'x' => 2,
                'u' => 4,
                _ => 0,
            };
            if count > 0
                && index + 2 + count <= chars.len()
                && chars[index + 2..index + 2 + count]
                    .iter()
                    .all(char::is_ascii_hexdigit)
            {
                let digits: String = chars[index + 2..index + 2 + count].iter().collect();
                let value =
                    u32::from_str_radix(&digits, 16).map_err(|_| "Invalid MTA Unicode escape")?;
                out.push(char::from_u32(value).ok_or("MTA Unicode escape is not a scalar value")?);
                index += 2 + count;
                continue;
            }
        }
        out.push(chars[index]);
        index += 1;
    }
    Ok(out)
}
/// `.mta` type IDs TagGroove assigns to action types with no known Mp3tag ID.
/// Mp3tag lists them as unknown actions; TagGroove reads them back.
pub const MTA_OPENTAG_TYPES: [(u32, &str); 3] = [
    (1001, "Adjust Cover"),
    (1002, "Set Cover Properties"),
    (1003, "Generate Playlists"),
];

const CASE_MODES: [CaseMode; 4] = [
    CaseMode::Lower,
    CaseMode::Mixed,
    CaseMode::Sentence,
    CaseMode::Upper,
];

/// Settings of one `.mta` section, checked strictly so that an unexpected
/// key or value leaves the record unresolved instead of being guessed.
struct MtaSettings<'a> {
    record: &'a MtaRecord,
    type_id: &'a str,
}
impl MtaSettings<'_> {
    fn only(&self, keys: &[&str]) -> Result<(), String> {
        match self
            .record
            .properties
            .keys()
            .find(|key| !["T", "F"].contains(&key.as_str()) && !keys.contains(&key.as_str()))
        {
            Some(key) => Err(format!(
                "Unknown setting {key} for action type {}",
                self.type_id
            )),
            None => Ok(()),
        }
    }
    fn optional(&self, key: &str) -> Option<String> {
        self.record.properties.get(key).cloned()
    }
    fn required(&self, key: &str) -> Result<String, String> {
        self.optional(key)
            .ok_or_else(|| format!("Missing MTA setting {key}"))
    }
    fn flag(&self, key: &str) -> Result<bool, String> {
        match self.optional(key).as_deref() {
            None | Some("0") => Ok(false),
            Some("1") => Ok(true),
            Some(value) => Err(format!("Unknown flag {value:?} in setting {key}")),
        }
    }
    /// An ID3 picture type number (0–20); absent or empty means none.
    fn picture_type(&self, key: &str) -> Result<Option<String>, String> {
        match self.optional(key).filter(|value| !value.is_empty()) {
            None => Ok(None),
            Some(value) => match value.parse::<u8>() {
                Ok(number @ 0..=20) => Ok(Some(format!(
                    "{:?}",
                    lofty::picture::PictureType::from_u8(number)
                ))),
                _ => Err(format!("Unknown picture type {value:?}")),
            },
        }
    }
}

fn mta_picture_type(name: &Option<String>) -> Result<String, String> {
    name.as_deref()
        .map(|name| crate::picture_type_from_name(name).map(|kind| kind.as_u8().to_string()))
        .transpose()
        .map(Option::unwrap_or_default)
}

// Every shape/setting parser failure is a TagGroove-owned guard. Validation is
// separate because regex/codepage validation also returns foreign diagnostics.
fn mta_action_shape(record: &MtaRecord) -> Result<ActionSpec, String> {
    let type_id = record
        .properties
        .get("T")
        .cloned()
        .ok_or("Missing MTA setting T")?;
    let settings = MtaSettings {
        record,
        type_id: &type_id,
    };
    let field = || settings.required("F");
    let action = match type_id.as_str() {
        // Verified on the 3.36.1 action files: mode 1 with its word starts.
        // The other modes follow the documented order of the conversions.
        "1" => {
            settings.only(&["1", "2"])?;
            let mode = settings.required("1")?;
            let mode = mode
                .parse::<usize>()
                .ok()
                .and_then(|index| CASE_MODES.get(index))
                .ok_or_else(|| format!("Unknown case mode {mode:?}"))?;
            ActionSpec::CaseConversion {
                field: field()?,
                mode: *mode,
                word_separators: settings.optional("2").unwrap_or_default(),
            }
        }
        "2" => {
            settings.only(&["1", "2", "3"])?;
            let flags = settings.required("3")?;
            let (whole_word, case_sensitive) = match flags.as_str() {
                "0" | "0|0" => (false, false),
                "1|0" => (true, false),
                "0|1" => (false, true),
                "1|1" => (true, true),
                _ => return Err(format!("Unknown Replace flags {flags:?}")),
            };
            ActionSpec::Replace {
                field: field()?,
                from: settings.required("1")?,
                to: settings.required("2")?,
                whole_word,
                case_sensitive,
            }
        }
        "4" => {
            settings.only(&["1", "2", "3"])?;
            ActionSpec::RegexReplace {
                field: field()?,
                pattern: settings.required("1")?,
                replacement: settings.optional("2").unwrap_or_default(),
                case_sensitive: settings.flag("3")?,
            }
        }
        "5" | "7" | "8" | "16" => {
            settings.only(&["1"])?;
            let value = settings.required("1")?;
            match type_id.as_str() {
                "5" => ActionSpec::FormatValue {
                    field: field()?,
                    template: value,
                },
                "7" => ActionSpec::GuessValues {
                    source: field()?,
                    pattern: value,
                },
                "8" => ActionSpec::MergeDuplicateFields {
                    field: field()?,
                    separator: value,
                },
                _ => ActionSpec::SplitField {
                    field: field()?,
                    separator: value,
                },
            }
        }
        "6" => {
            settings.only(&["1"])?;
            ActionSpec::RemoveDuplicateFields {
                field: field()?,
                only_same_content: settings.flag("1")?,
            }
        }
        "9" | "10" => {
            settings.only(&[])?;
            let fields = field()?
                .split(';')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .collect();
            if type_id == "9" {
                ActionSpec::RemoveFields { fields }
            } else {
                ActionSpec::RemoveFieldsExcept { fields }
            }
        }
        "11" => {
            settings.only(&["1"])?;
            ActionSpec::ConvertCodepage {
                field: field()?,
                codepage: settings.required("1")?,
            }
        }
        "12" => {
            settings.only(&["1", "2"])?;
            ActionSpec::ImportCover {
                filename_template: field()?,
                picture_type: settings.picture_type("1")?,
                delete_existing: settings.flag("2")?,
            }
        }
        "13" => {
            settings.only(&["1", "2"])?;
            ActionSpec::ExportCover {
                filename_template: field()?,
                picture_type: settings.picture_type("1")?,
                export_duplicates: settings.flag("2")?,
            }
        }
        "14" => {
            settings.only(&["1"])?;
            ActionSpec::ImportTextFile {
                field: field()?,
                filename_template: settings.required("1")?,
            }
        }
        "15" => {
            settings.only(&["1", "2", "3"])?;
            ActionSpec::Export {
                template: field()?,
                filename_template: settings.optional("1").unwrap_or_default(),
                append: settings.flag("2")?,
                partition_template: settings.optional("3").filter(|p| !p.is_empty()),
            }
        }
        "1001" => {
            settings.only(&["1", "2", "3", "4", "5"])?;
            let format = match settings.required("1")?.as_str() {
                "0" => CoverImageFormat::Original,
                "1" => CoverImageFormat::Jpeg,
                "2" => CoverImageFormat::Png,
                format => return Err(format!("Unknown cover format {format:?}")),
            };
            let quality = settings
                .required("2")?
                .parse::<u8>()
                .map_err(|_| "Invalid cover quality")?;
            let pixels = settings
                .optional("3")
                .unwrap_or_else(|| "0".into())
                .parse::<u32>()
                .map_err(|_| "Invalid cover size")?;
            let resize = if pixels == 0 {
                None
            } else {
                Some(CoverResize {
                    pixels,
                    size_mode: match settings.required("4")?.as_str() {
                        "0" => CoverSizeMode::Maximum,
                        "1" => CoverSizeMode::Minimum,
                        mode => return Err(format!("Unknown cover size mode {mode:?}")),
                    },
                    scale_mode: match settings.required("5")?.as_str() {
                        "0" => CoverScaleMode::Proportional,
                        "1" => CoverScaleMode::Square,
                        "2" => CoverScaleMode::CropSquare,
                        mode => return Err(format!("Unknown cover scale mode {mode:?}")),
                    },
                })
            };
            ActionSpec::AdjustCover {
                format,
                quality,
                resize,
            }
        }
        "1002" => {
            settings.only(&["1", "2", "3"])?;
            ActionSpec::SetCoverProperties {
                picture_type: settings.picture_type("1")?,
                description: settings.optional("2"),
                only_picture_type: settings.picture_type("3")?,
            }
        }
        "1003" => {
            settings.only(&["1"])?;
            ActionSpec::GeneratePlaylists {
                filename_template: field()?,
                partition_template: settings.optional("1").unwrap_or_default(),
            }
        }
        _ => {
            return Err(format!(
                "Numeric action type {type_id} has no known settings schema"
            ))
        }
    };
    Ok(action)
}
fn mta_action_diagnostic(record: &MtaRecord) -> Result<ActionSpec, ErrorCause> {
    let action = mta_action_shape(record).map_err(ErrorCause::owned_text)?;
    validate_action_diagnostic(&action)?;
    Ok(action)
}
/// Read UTF-8 or BOM-marked UTF-16. Legacy encodings require explicit selection.
/// Unknown type IDs/settings become preserved, non-executable records.
pub fn import_mta(
    name: String,
    bytes: &[u8],
    legacy_encoding: Option<&str>,
) -> Result<MtaImport, String> {
    import_mta_diagnostic(name, bytes, legacy_encoding).map(|report| report.result)
}
#[derive(Debug, Serialize)]
pub struct MtaImportReport {
    #[serde(flatten)]
    pub result: MtaImport,
    pub issue_diagnostics: Vec<IndexedCause>,
}
/// Add presentation companions without changing the legacy import or reasons.
pub fn import_mta_diagnostic(
    name: String,
    bytes: &[u8],
    legacy_encoding: Option<&str>,
) -> Result<MtaImportReport, String> {
    if bytes.len() > 4 * LIMIT {
        return Err("Action group import limit exceeded".into());
    }
    let (encoding, input) = if bytes.starts_with(&[0xff, 0xfe]) {
        (encoding_rs::UTF_16LE, &bytes[2..])
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        (encoding_rs::UTF_16BE, &bytes[2..])
    } else if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        (encoding_rs::UTF_8, &bytes[3..])
    } else if let Some(label) = legacy_encoding {
        (
            encoding_rs::Encoding::for_label(label.as_bytes())
                .ok_or("Unsupported action-file encoding")?,
            bytes,
        )
    } else {
        (encoding_rs::UTF_8, bytes)
    };
    let text = encoding
        .decode_without_bom_handling_and_without_replacement(input)
        .ok_or("Invalid action-file encoding; choose its original legacy codepage")?;
    let mut records: BTreeMap<usize, MtaRecord> = BTreeMap::new();
    let mut current = None;
    for (line_index, line) in text.lines().enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.trim().is_empty()
            || line.trim_start().starts_with('!')
            || line.trim_start().starts_with(';')
        {
            continue;
        }
        if line.starts_with("[#") && line.ends_with(']') {
            let index = line[2..line.len() - 1]
                .parse::<usize>()
                .map_err(|_| format!("Invalid action section at line {}", line_index + 1))?;
            if records
                .insert(
                    index,
                    MtaRecord {
                        index,
                        properties: BTreeMap::new(),
                    },
                )
                .is_some()
            {
                return Err(format!("Duplicate action section #{index}"));
            }
            current = Some(index);
            continue;
        }
        let index = current.ok_or_else(|| {
            format!(
                "Setting outside an action section at line {}",
                line_index + 1
            )
        })?;
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("Invalid action setting at line {}", line_index + 1))?;
        if key.trim().is_empty() {
            return Err("Empty action setting name".into());
        }
        if records
            .get_mut(&index)
            .unwrap()
            .properties
            .insert(key.trim().into(), mta_value(value)?)
            .is_some()
        {
            return Err(format!("Duplicate setting {key} in action #{index}"));
        }
    }
    let records: Vec<MtaRecord> = records.into_values().collect();
    if records.is_empty() {
        return Err("Action group contains no actions".into());
    }
    let mut actions = Vec::new();
    let mut issues = Vec::new();
    let mut issue_diagnostics = Vec::new();
    for record in &records {
        match mta_action_diagnostic(record) {
            Ok(action) => actions.push(action),
            Err(cause) => {
                let reason = cause.to_string();
                let type_id = record.properties.get("T").cloned().unwrap_or_default();
                issue_diagnostics.push(IndexedCause {
                    index: issues.len(),
                    cause,
                });
                issues.push(MtaIssue {
                    index: record.index,
                    type_id: type_id.clone(),
                    reason: reason.clone(),
                });
                actions.push(ActionSpec::UnresolvedMta {
                    type_id,
                    properties: record.properties.clone(),
                    reason,
                });
            }
        }
    }
    Ok(MtaImportReport {
        result: MtaImport {
            group: ActionGroup {
                name,
                enabled: issues.is_empty(),
                actions,
            },
            records,
            issues,
        },
        issue_diagnostics,
    })
}

fn mta_escape(value: &str, opaque: bool) -> Result<String, String> {
    if !opaque && value.contains(['\r', '\n']) {
        return Err("Line breaks cannot be stored in an .mta setting".into());
    }
    let chars: Vec<char> = value.chars().collect();
    let mut out = String::new();
    for (index, c) in chars.iter().enumerate() {
        match *c {
            // Opaque records already accepted these escaped values on import.
            // Preserve them without assigning executable semantics to the action.
            '\r' => out.push_str("\\x0d"),
            '\n' => out.push_str("\\x0a"),
            // A backslash stays literal unless the importer would read it as
            // the start of an escape sequence (including one written below).
            '\\' if chars.get(index + 1).is_some_and(|next| {
                matches!(next, 'x' | 'u' | '\\') || (0x80..=0xffff).contains(&u32::from(*next))
            }) =>
            {
                out.push_str("\\\\")
            }
            c if u32::from(c) > 0x7f && u32::from(c) <= 0xff => {
                out.push_str(&format!("\\x{:02x}", u32::from(c)))
            }
            c if u32::from(c) > 0xff && u32::from(c) <= 0xffff => {
                out.push_str(&format!("\\u{:04x}", u32::from(c)))
            }
            c => out.push(c),
        }
    }
    Ok(out)
}
/// Serialize a group in the `.mta` layout [`import_mta`] reads: every action
/// type, preserving decoded properties of unresolved records. Line breaks in
/// opaque values use the existing escape grammar; resolved actions still refuse
/// line breaks because their original serialized semantics are unqualified.
pub fn export_mta(group: &ActionGroup) -> Result<String, String> {
    if group.actions.is_empty() {
        return Err("Action group contains no actions".into());
    }
    let mut out = String::new();
    for (index, action) in group.actions.iter().enumerate() {
        let flag = |value: bool| if value { "1" } else { "0" };
        let pair = |key: &str, value: String| (key.to_string(), value);
        let settings: Vec<(String, String)> = match action {
            ActionSpec::CaseConversion {
                field,
                mode,
                word_separators,
            } => vec![
                pair("T", "1".into()),
                pair("F", field.clone()),
                pair(
                    "1",
                    CASE_MODES
                        .iter()
                        .position(|candidate| candidate == mode)
                        .unwrap_or(1)
                        .to_string(),
                ),
                pair("2", word_separators.clone()),
            ],
            ActionSpec::Replace {
                field,
                from,
                to,
                whole_word,
                case_sensitive,
            } => vec![
                pair("T", "2".into()),
                pair("F", field.clone()),
                pair("1", from.clone()),
                pair("2", to.clone()),
                pair(
                    "3",
                    format!("{}|{}", flag(*whole_word), flag(*case_sensitive)),
                ),
            ],
            ActionSpec::RegexReplace {
                field,
                pattern,
                replacement,
                case_sensitive,
            } => vec![
                pair("T", "4".into()),
                pair("F", field.clone()),
                pair("1", pattern.clone()),
                pair("2", replacement.clone()),
                pair("3", flag(*case_sensitive).into()),
            ],
            ActionSpec::FormatValue { field, template } => vec![
                pair("T", "5".into()),
                pair("F", field.clone()),
                pair("1", template.clone()),
            ],
            ActionSpec::RemoveDuplicateFields {
                field,
                only_same_content,
            } => vec![
                pair("T", "6".into()),
                pair("F", field.clone()),
                pair("1", flag(*only_same_content).into()),
            ],
            ActionSpec::GuessValues { source, pattern } => vec![
                pair("T", "7".into()),
                pair("F", source.clone()),
                pair("1", pattern.clone()),
            ],
            ActionSpec::MergeDuplicateFields { field, separator } => vec![
                pair("T", "8".into()),
                pair("F", field.clone()),
                pair("1", separator.clone()),
            ],
            ActionSpec::SplitField { field, separator } => vec![
                pair("T", "16".into()),
                pair("F", field.clone()),
                pair("1", separator.clone()),
            ],
            ActionSpec::RemoveFields { fields } => {
                vec![pair("T", "9".into()), pair("F", fields.join(";"))]
            }
            ActionSpec::RemoveFieldsExcept { fields } => {
                vec![pair("T", "10".into()), pair("F", fields.join(";"))]
            }
            ActionSpec::ConvertCodepage { field, codepage } => vec![
                pair("T", "11".into()),
                pair("F", field.clone()),
                pair("1", codepage.clone()),
            ],
            ActionSpec::ImportCover {
                filename_template,
                picture_type,
                delete_existing,
            } => vec![
                pair("T", "12".into()),
                pair("F", filename_template.clone()),
                pair("1", mta_picture_type(picture_type)?),
                pair("2", flag(*delete_existing).into()),
            ],
            ActionSpec::ExportCover {
                filename_template,
                picture_type,
                export_duplicates,
            } => vec![
                pair("T", "13".into()),
                pair("F", filename_template.clone()),
                pair("1", mta_picture_type(picture_type)?),
                pair("2", flag(*export_duplicates).into()),
            ],
            ActionSpec::ImportTextFile {
                field,
                filename_template,
            } => vec![
                pair("T", "14".into()),
                pair("F", field.clone()),
                pair("1", filename_template.clone()),
            ],
            ActionSpec::Export {
                template,
                filename_template,
                append,
                partition_template,
            } => {
                let mut settings = vec![
                    pair("T", "15".into()),
                    pair("F", template.clone()),
                    pair("1", filename_template.clone()),
                    pair("2", flag(*append).into()),
                ];
                if let Some(partition) = partition_template {
                    settings.push(pair("3", partition.clone()));
                }
                settings
            }
            ActionSpec::AdjustCover {
                format,
                quality,
                resize,
            } => {
                let mut settings = vec![
                    pair("T", "1001".into()),
                    pair(
                        "1",
                        match format {
                            CoverImageFormat::Original => "0",
                            CoverImageFormat::Jpeg => "1",
                            CoverImageFormat::Png => "2",
                        }
                        .into(),
                    ),
                    pair("2", quality.to_string()),
                ];
                if let Some(resize) = resize {
                    settings.extend([
                        pair("3", resize.pixels.to_string()),
                        pair(
                            "4",
                            match resize.size_mode {
                                CoverSizeMode::Maximum => "0",
                                CoverSizeMode::Minimum => "1",
                            }
                            .into(),
                        ),
                        pair(
                            "5",
                            match resize.scale_mode {
                                CoverScaleMode::Proportional => "0",
                                CoverScaleMode::Square => "1",
                                CoverScaleMode::CropSquare => "2",
                            }
                            .into(),
                        ),
                    ]);
                }
                settings
            }
            ActionSpec::SetCoverProperties {
                picture_type,
                description,
                only_picture_type,
            } => {
                let mut settings = vec![
                    pair("T", "1002".into()),
                    pair("1", mta_picture_type(picture_type)?),
                ];
                if let Some(description) = description {
                    settings.push(pair("2", description.clone()));
                }
                settings.push(pair("3", mta_picture_type(only_picture_type)?));
                settings
            }
            ActionSpec::GeneratePlaylists {
                filename_template,
                partition_template,
            } => vec![
                pair("T", "1003".into()),
                pair("F", filename_template.clone()),
                pair("1", partition_template.clone()),
            ],
            ActionSpec::UnresolvedMta { properties, .. } => {
                let mut ordered: Vec<(String, String)> = ["T", "F"]
                    .iter()
                    .filter_map(|key| properties.get(*key).map(|v| (key.to_string(), v.clone())))
                    .collect();
                ordered.extend(
                    properties
                        .iter()
                        .filter(|(key, _)| !["T", "F"].contains(&key.as_str()))
                        .map(|(key, value)| (key.clone(), value.clone())),
                );
                ordered
            }
        };
        out.push_str(&format!("[#{index}]\r\n"));
        let opaque = matches!(action, ActionSpec::UnresolvedMta { .. });
        for (key, value) in settings {
            out.push_str(&format!("{key}={}\r\n", mta_escape(&value, opaque)?));
        }
        out.push_str("\r\n");
    }
    Ok(out)
}

/// Adapter for existing five-type Quick Actions requests during migration.
pub fn from_quick_action(action: &Action) -> Result<ActionSpec, String> {
    Ok(match action {
        Action::Format { field, template } => ActionSpec::FormatValue {
            field: field.clone(),
            template: template.clone(),
        },
        Action::Replace {
            field,
            from,
            to,
            case_sensitive,
        } => ActionSpec::Replace {
            field: field.clone(),
            from: from.clone(),
            to: to.clone(),
            case_sensitive: *case_sensitive,
            whole_word: false,
        },
        Action::RegexReplace {
            field,
            pattern,
            replacement,
        } => ActionSpec::RegexReplace {
            field: field.clone(),
            pattern: pattern.clone(),
            replacement: replacement.clone(),
            case_sensitive: true,
        },
        Action::Case { field, mode } => ActionSpec::CaseConversion {
            field: field.clone(),
            mode: match mode.as_str() {
                "lower" => CaseMode::Lower,
                "upper" => CaseMode::Upper,
                "title" => CaseMode::Mixed,
                "sentence" => CaseMode::Sentence,
                _ => return Err(format!("Unsupported case mode {mode}")),
            },
            word_separators: String::new(),
        },
        Action::Remove { fields } => ActionSpec::RemoveFields {
            fields: fields.clone(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn action_diagnostics_distinguish_owned_guards_from_external_evaluator_errors() {
        use crate::batch_errors::ErrorCause;
        let owned = apply_action_spec_with_modes_diagnostic(
            &mut state(),
            &ActionSpec::FormatValue {
                field: "_FILENAME".into(),
                template: String::new(),
            },
            &mut |_, _, _| Ok(String::new()),
        )
        .unwrap_err();
        assert!(
            matches!(&owned, ErrorCause::Owned { key, params, children } if key == "Filename must not be empty" && params.is_empty() && children.is_empty())
        );
        let external = apply_action_spec_with_modes_diagnostic(
            &mut state(),
            &ActionSpec::FormatValue {
                field: "TITLE".into(),
                template: String::new(),
            },
            &mut |_, _, _| {
                Err(ErrorCause::external(std::io::Error::other(
                    "Filename must not be empty",
                )))
            },
        )
        .unwrap_err();
        assert!(
            matches!(&external, ErrorCause::External { value } if value == "Filename must not be empty")
        );
        assert_eq!(owned.to_string(), external.to_string());
        let action = ActionSpec::RegexReplace {
            field: "TITLE".into(),
            pattern: "[".into(),
            replacement: String::new(),
            case_sensitive: false,
        };
        let error = validate_action_diagnostic(&action).unwrap_err();
        assert!(matches!(&error, ErrorCause::External { .. }));
        assert_eq!(error.to_string(), validate_action(&action).unwrap_err());
    }
    fn state() -> ActionState {
        ActionState {
            fields: Fields::from([
                (
                    "ARTIST".into(),
                    vec!["Ada".into(), "Bob".into(), "Ada".into()],
                ),
                ("TITLE".into(), vec!["mIXED (aRTIST-title_name".into()]),
                ("COMMENT".into(), vec!["one\ntwo".into()]),
            ]),
            context: Fields::from([
                ("_FILENAME".into(), vec!["old".into()]),
                ("_FILENAME_EXT".into(), vec!["old.flac".into()]),
                ("_DIRECTORY".into(), vec!["album".into()]),
            ]),
        }
    }
    #[derive(Default)]
    struct TestHost {
        events: Vec<String>,
    }
    impl ActionHost for TestHost {
        fn execute(
            &mut self,
            effect: &ActionEffect,
            state: &mut ActionState,
        ) -> Result<(), String> {
            self.events.push(format!("{effect:?}"));
            match effect {
                ActionEffect::ImportTextFile { field, .. } => {
                    state
                        .fields
                        .insert(field.clone(), vec!["imported\ntext".into()]);
                }
                ActionEffect::RenameFile { filename, .. } => {
                    state
                        .context
                        .insert("_FILENAME".into(), vec![filename.clone()]);
                }
                _ => {}
            }
            Ok(())
        }
        fn execute_batch(
            &mut self,
            effect: &BatchActionEffect,
            states: &mut [ActionState],
        ) -> Result<(), String> {
            self.events
                .push(format!("{effect:?} title={}", states[0].fields["TITLE"][0]));
            Ok(())
        }
    }
    #[test]
    fn guessing_targets_preserve_repetition_and_ignore_dummy() {
        assert_eq!(
            guess_value_targets("%artist% / %DUMMY% / %ARTIST% - %TITLE%").unwrap(),
            vec!["ARTIST", "ARTIST", "TITLE"]
        );
        let fields = guess_values(
            "Ada / ignored / Bob - Mémoire\n猫",
            "%artist% / %DUMMY% / %ARTIST% - %TITLE%",
        )
        .unwrap();
        assert_eq!(fields["ARTIST"], vec!["Ada", "Bob"]);
        assert_eq!(fields["TITLE"], vec!["Mémoire\n猫"]);
        assert!(!fields.contains_key("DUMMY"));
    }

    #[test]
    fn guessing_target_inspection_rejects_the_execution_grammar_errors() {
        for pattern in ["literal text", "%TITLE", "%%", "%ARTIST%%TITLE%"] {
            assert!(guess_value_targets(pattern).is_err(), "{pattern}");
            assert!(guess_values("sample", pattern).is_err(), "{pattern}");
        }
    }

    #[test]
    fn guessing_targets_identify_technical_names_without_evaluating_values() {
        assert_eq!(
            guess_value_targets("%TITLE% - %_filename% - %_DIRECTORY%").unwrap(),
            vec!["TITLE", "_FILENAME", "_DIRECTORY"]
        );
    }

    #[test]
    fn mixed_case_uses_configured_delimiters_and_whitespace() {
        let mut state = state();
        apply_action_spec(
            &mut state,
            &ActionSpec::CaseConversion {
                field: "TITLE".into(),
                mode: CaseMode::Mixed,
                word_separators: "(-_".into(),
            },
        )
        .unwrap();
        assert_eq!(state.fields["TITLE"], ["Mixed (Artist-Title_Name"]);
    }
    #[test]
    fn merge_and_split_preserve_order_and_trim_split_values() {
        let mut state = state();
        apply_action_spec(
            &mut state,
            &ActionSpec::MergeDuplicateFields {
                field: "ARTIST".into(),
                separator: " ; ".into(),
            },
        )
        .unwrap();
        assert_eq!(state.fields["ARTIST"], ["Ada ; Bob ; Ada"]);
        apply_action_spec(
            &mut state,
            &ActionSpec::SplitField {
                field: "ARTIST".into(),
                separator: ";".into(),
            },
        )
        .unwrap();
        assert_eq!(state.fields["ARTIST"], ["Ada", "Bob", "Ada"]);
        assert_eq!(state.fields["COMMENT"], ["one\ntwo"]);
    }
    #[test]
    fn duplicate_fields_same_content_differs_from_keep_first() {
        let mut same = state();
        apply_action_spec(
            &mut same,
            &ActionSpec::RemoveDuplicateFields {
                field: "ARTIST".into(),
                only_same_content: true,
            },
        )
        .unwrap();
        assert_eq!(same.fields["ARTIST"], ["Ada", "Bob"]);
        let mut first = state();
        apply_action_spec(
            &mut first,
            &ActionSpec::RemoveDuplicateFields {
                field: "_ALL".into(),
                only_same_content: false,
            },
        )
        .unwrap();
        assert_eq!(first.fields["ARTIST"], ["Ada"]);
    }
    #[test]
    fn remove_except_explicitly_preserves_or_removes_covers() {
        let mut first = state();
        let effects = apply_action_spec(
            &mut first,
            &ActionSpec::RemoveFieldsExcept {
                fields: vec!["artist".into(), "picture".into()],
            },
        )
        .unwrap();
        assert!(effects.is_empty());
        assert_eq!(first.fields.len(), 1);
        let mut second = state();
        let effects = apply_action_spec(
            &mut second,
            &ActionSpec::RemoveFieldsExcept {
                fields: vec!["TITLE".into()],
            },
        )
        .unwrap();
        assert_eq!(effects, [ActionEffect::RemoveCovers]);
    }
    #[test]
    fn cover_aliases_produce_one_effect() {
        let mut state = state();
        let effects = apply_action_spec(
            &mut state,
            &ActionSpec::RemoveFields {
                fields: vec!["COMMENT".into(), "cover".into(), "PICTURE".into()],
            },
        )
        .unwrap();
        assert!(!state.fields.contains_key("COMMENT"));
        assert_eq!(effects, [ActionEffect::RemoveCovers]);
    }
    #[test]
    fn whole_word_boundaries_match_documented_punctuation() {
        let values = vec!["DJ, dj! xdj DJ-name DJ_name (DJ) DJ.".into()];
        assert_eq!(
            replace_values(&values, "DJ", "X", true, false).unwrap(),
            ["X, X! xdj DJ-name DJ_name (DJ) X."]
        );
    }
    #[test]
    fn regex_filename_uses_extension_while_literal_uses_stem() {
        let mut first = state();
        let effects = apply_action_spec(
            &mut first,
            &ActionSpec::RegexReplace {
                field: "_FILENAME".into(),
                pattern: "\\.flac$".into(),
                replacement: ".FLAC".into(),
                case_sensitive: true,
            },
        )
        .unwrap();
        assert_eq!(
            effects,
            [ActionEffect::RenameFile {
                filename: "old.FLAC".into(),
                includes_extension: true,
                origin: NameOrigin::Edited,
            }]
        );
        let mut second = state();
        let effects = apply_action_spec(
            &mut second,
            &ActionSpec::Replace {
                field: "_ALL".into(),
                from: "old".into(),
                to: "new".into(),
                whole_word: false,
                case_sensitive: true,
            },
        )
        .unwrap();
        assert_eq!(
            effects,
            [ActionEffect::RenameFile {
                filename: "new".into(),
                includes_extension: false,
                origin: NameOrigin::Edited,
            }]
        );
    }
    #[test]
    fn filename_targets_use_filename_mode_and_record_their_origin() {
        let mut state = state();
        state
            .fields
            .insert("TITLE".into(), vec!["AC/DC: Live".into()]);
        let effects = apply_action_spec(
            &mut state,
            &ActionSpec::FormatValue {
                field: "_filename".into(),
                template: "Live\\%title%.flac".into(),
            },
        )
        .unwrap();
        assert_eq!(
            effects,
            [ActionEffect::RenameFile {
                filename: "Live\\AC_DC_ Live.flac".into(),
                includes_extension: false,
                origin: NameOrigin::Template,
            }]
        );
        let effects = apply_action_spec(
            &mut state,
            &ActionSpec::FormatValue {
                field: "_DIRECTORY".into(),
                template: "%title%".into(),
            },
        )
        .unwrap();
        assert_eq!(
            effects,
            [ActionEffect::RenameDirectory {
                directory: "AC_DC_ Live".into(),
                origin: NameOrigin::Template,
            }]
        );
        // Guessed text stays raw here; the host replaces its separators.
        let effects = apply_action_spec(
            &mut state,
            &ActionSpec::GuessValues {
                source: "%title%".into(),
                pattern: "%_filename%".into(),
            },
        )
        .unwrap();
        assert_eq!(
            effects,
            [ActionEffect::RenameFile {
                filename: "AC/DC: Live".into(),
                includes_extension: false,
                origin: NameOrigin::Value,
            }]
        );
    }
    #[test]
    fn format_value_double_backslash_creates_several_values() {
        let mut state = state();
        let format = |state: &mut ActionState, template: &str| {
            apply_action_spec(
                state,
                &ActionSpec::FormatValue {
                    field: "genre".into(),
                    template: template.into(),
                },
            )
            .unwrap();
            state.fields["GENRE"].clone()
        };
        assert_eq!(format(&mut state, "Rock\\\\Pop"), ["Rock", "Pop"]);
        // Separator spacing as the Tag Panel shows it, and empty pieces.
        assert_eq!(
            format(&mut state, "Jazz \\\\ \\\\ Blues \\\\"),
            ["Jazz", "Blues"]
        );
        // One backslash is text, and a single value keeps its spaces.
        assert_eq!(format(&mut state, " AC\\DC "), [" AC\\DC "]);
        apply_action_spec(
            &mut state,
            &ActionSpec::FormatValue {
                field: "genre".into(),
                template: " \\\\ ".into(),
            },
        )
        .unwrap();
        assert!(!state.fields.contains_key("GENRE"));
    }
    #[test]
    fn cover_and_text_file_names_use_filename_mode() {
        let mut state = state();
        state
            .fields
            .insert("ALBUM".into(), vec!["AC/DC: Live?".into()]);
        state
            .context
            .insert("_FOLDERPATH".into(), vec!["/music/album/".into()]);
        let effects = |state: &mut ActionState, action: ActionSpec| {
            apply_action_spec(state, &action).unwrap()
        };
        assert_eq!(
            effects(
                &mut state,
                ActionSpec::ExportCover {
                    filename_template: "%_folderpath%Covers\\%album%".into(),
                    picture_type: None,
                    export_duplicates: false,
                }
            ),
            [ActionEffect::ExportCover {
                filename: "/music/album/Covers\\AC_DC_ Live_".into(),
                picture_type: None,
                export_duplicates: false,
            }]
        );
        assert_eq!(
            effects(
                &mut state,
                ActionSpec::ImportCover {
                    filename_template: "%album%\\*.jpg".into(),
                    picture_type: None,
                    delete_existing: false,
                }
            ),
            [ActionEffect::ImportCover {
                filename: "AC_DC_ Live_\\*.jpg".into(),
                picture_type: None,
                delete_existing: false,
            }]
        );
        assert_eq!(
            effects(
                &mut state,
                ActionSpec::ImportTextFile {
                    field: "LYRICS".into(),
                    filename_template: "%album%.txt".into(),
                }
            ),
            [ActionEffect::ImportTextFile {
                field: "LYRICS".into(),
                filename: "AC_DC_ Live_.txt".into(),
            }]
        );
    }
    #[test]
    fn field_name_actions_accept_the_displayed_ape_and_vorbis_tags() {
        let action = ActionSpec::CaseConversion {
            field: "_FIELDNAME".into(),
            mode: CaseMode::Upper,
            word_separators: String::new(),
        };
        for (tag, read, accepted) in [
            ("ID3v1 APEv2", "APEv2", true),
            ("VorbisComment", "VorbisComment", true),
            ("ID3v2.3 ID3v1", "ID3v2.3", false),
            ("MP4", "MP4", false),
        ] {
            let mut state = state();
            state.context.insert("_TAG".into(), vec![tag.into()]);
            state.context.insert("_TAG_READ".into(), vec![read.into()]);
            let result = apply_action_spec(&mut state, &action);
            assert_eq!(result.is_ok(), accepted, "{read}: {result:?}");
            if accepted {
                assert!(
                    state.fields.contains_key("ARTIST") && !state.fields.contains_key("Artist")
                );
            }
        }
        let mut state = state();
        state.fields = Fields::from([("Artist".into(), vec!["Ada".into()])]);
        state
            .context
            .insert("_TAG_READ".into(), vec!["APEv2".into()]);
        apply_action_spec(&mut state, &action).unwrap();
        assert_eq!(
            state.fields,
            Fields::from([("ARTIST".into(), vec!["Ada".into()])])
        );
    }
    #[test]
    fn regex_references_and_case_flag_reuse_legacy_expander() {
        let mut state = state();
        state.fields.insert("TITLE".into(), vec!["ABC-12".into()]);
        apply_action_spec(
            &mut state,
            &ActionSpec::RegexReplace {
                field: "TITLE".into(),
                pattern: "(abc)-(\\d+)".into(),
                replacement: "$2:$1suffix".into(),
                case_sensitive: false,
            },
        )
        .unwrap();
        assert_eq!(state.fields["TITLE"], ["12:ABCsuffix"]);
    }
    #[test]
    fn regex_action_supports_native_boost_lookaround_backrefs_and_case_replacement() {
        assert_eq!(
            regex_values(&["abc abc".into()], r"\b(\w+) \1\b", r"\U$1\E", true).unwrap(),
            ["ABC"]
        );
        assert_eq!(
            regex_values(&["cat dog".into()], r"(?<=\b)cat(?=\b)", "X", true).unwrap(),
            ["X dog"]
        );
    }
    #[test]
    fn guess_values_handles_paths_dots_dummy_and_repeated_values() {
        assert_eq!(
            guess_values(
                "C:/Ada/Bob. live / skip / ignored",
                "%artist% / %title% / %dummy%"
            )
            .unwrap(),
            Fields::from([
                ("ARTIST".into(), vec!["C:/Ada/Bob. live".into()]),
                ("TITLE".into(), vec!["skip".into()])
            ])
        );
    }
    #[test]
    fn guess_values_has_no_filename_extension_processing() {
        let fields = guess_values("Artist / A.title.flac", "%artist% / %title%").unwrap();
        assert_eq!(fields["TITLE"], ["A.title.flac"]);
        let fields = guess_values("A + B", "%artist% + %artist%").unwrap();
        assert_eq!(fields["ARTIST"], ["A", "B"]);
        assert!(guess_values("anything", "%artist%%title%").is_err());
    }
    #[test]
    fn guess_values_leaves_fields_unchanged_when_the_source_does_not_match() {
        let mut state = state();
        state
            .fields
            .insert("TITLE".into(), vec!["No separator".into()]);
        let before = state.fields.clone();
        let effects = apply_action_spec(
            &mut state,
            &ActionSpec::GuessValues {
                source: "%title%".into(),
                pattern: "%artist% / %title%".into(),
            },
        )
        .unwrap();
        assert!(effects.is_empty());
        assert_eq!(state.fields, before);
        assert_eq!(
            try_guess_values("No separator", "%artist% / %title%").unwrap(),
            None
        );
        // An invalid pattern is still an error.
        assert!(try_guess_values("A B", "%artist%%title%").is_err());
    }
    #[test]
    fn guess_source_and_target_can_reference_same_field() {
        let mut state = state();
        state
            .fields
            .insert("TITLE".into(), vec!["Ada / Song".into()]);
        apply_action_spec(
            &mut state,
            &ActionSpec::GuessValues {
                source: "%title%".into(),
                pattern: "%artist% / %title%".into(),
            },
        )
        .unwrap();
        assert_eq!(state.fields["ARTIST"], ["Ada"]);
        assert_eq!(state.fields["TITLE"], ["Song"]);
    }
    #[test]
    fn technical_action_diagnostics_preserve_authored_tokens() {
        assert_eq!(
            decode_codepage("λ", "1251").unwrap_err(),
            "Character 'λ' has no legacy byte representation"
        );
        assert_eq!(
            decode_codepage("\u{d5}", "857").unwrap_err(),
            "Byte 0xD5 is undefined in codepage 857"
        );
        assert_eq!(
            apply_action_spec(
                &mut state(),
                &ActionSpec::RegexReplace {
                    field: "_CUSTOM".into(),
                    pattern: "a".into(),
                    replacement: "b".into(),
                    case_sensitive: true
                }
            )
            .unwrap_err(),
            "Unsupported action selector _CUSTOM"
        );
        for token in ["Title", "Help", "My  Mode"] {
            assert_eq!(
                decode_codepage("abc", token).unwrap_err(),
                format!("Unsupported codepage {token}")
            );
            assert_eq!(
                from_quick_action(&Action::Case {
                    field: "TITLE".into(),
                    mode: token.into()
                })
                .unwrap_err(),
                format!("Unsupported case mode {token}")
            );
        }
    }
    #[test]
    fn codepage_conversion_recovers_known_legacy_bytes_losslessly() {
        assert_eq!(
            decode_codepage("\u{cf}\u{f0}\u{e8}\u{e2}\u{e5}\u{f2}", "1251").unwrap(),
            "Привет"
        );
        assert_eq!(decode_codepage("\u{c3}\u{a9}", "65001").unwrap(), "é");
        assert!(decode_codepage("💿", "1251").is_err());
        assert!(decode_codepage("abc", "not-a-codepage").is_err());
        assert_eq!(decode_codepage("\u{80}", "28591").unwrap(), "\u{80}");
        assert_eq!(decode_codepage("\u{80}", "1252").unwrap(), "€");
        assert!(decode_codepage("\u{80}", "20127").is_err());
    }
    #[test]
    fn codepage_conversion_covers_dos_oem_pages() {
        // "Größe" written in the DOS console's code page 437 and 850.
        assert_eq!(decode_codepage("Gr\u{94}\u{e1}e", "437").unwrap(), "Größe");
        assert_eq!(
            decode_codepage("Gr\u{94}\u{e1}e", "cp850").unwrap(),
            "Größe"
        );
        // "Łódź" in code page 852 (Latin 2).
        assert_eq!(
            decode_codepage("\u{9d}\u{a2}d\u{ab}", "852").unwrap(),
            "Łódź"
        );
        // Box drawing in 437 and the euro sign that 858 adds over 850.
        assert_eq!(decode_codepage("\u{c9}\u{cd}", "437").unwrap(), "╔═");
        assert_eq!(decode_codepage("\u{d5}", "858").unwrap(), "€");
        assert_eq!(decode_codepage("\u{d5}", "850").unwrap(), "ı");
        // ASCII stays ASCII, and undefined bytes are refused.
        assert_eq!(decode_codepage("100%", "864").unwrap(), "100%");
        assert!(decode_codepage("\u{d5}", "857").is_err());
    }
    #[test]
    fn context_sensitive_action_evaluation_uses_real_app_options() {
        let mut state = state();
        let columns = vec![crate::script::ScriptColumn {
            name: "Title".into(),
            value: "Song".into(),
        }];
        let mut evaluate = |template: &str, state: &ActionState| {
            let fields = state.evaluation_fields();
            let mut context = crate::script::EvalContext::new(&fields);
            context.columns = Some(&columns);
            context.app_directory = Some("/applications/opentag");
            crate::script::evaluate_with_context(template, &context)
        };
        apply_action_spec_with_evaluator(
            &mut state,
            &ActionSpec::FormatValue {
                field: "COMMENT".into(),
                template: "$columnnames('<','>'):$getenv(MP3TAGAPP)".into(),
            },
            &mut evaluate,
        )
        .unwrap();
        assert_eq!(state.fields["COMMENT"], ["<Title>:/applications/opentag"]);
    }
    #[test]
    fn group_action_management_preserves_order_and_settings() {
        let mut library = ActionLibrary::default();
        let index = library
            .add_group(ActionGroup {
                name: "Workflow".into(),
                enabled: true,
                actions: vec![],
            })
            .unwrap();
        let a = ActionSpec::FormatValue {
            field: "TITLE".into(),
            template: "A".into(),
        };
        let b = ActionSpec::FormatValue {
            field: "TITLE".into(),
            template: "B".into(),
        };
        library.add_action(index, a.clone()).unwrap();
        library.add_action(index, b.clone()).unwrap();
        library.duplicate_action(index, 0).unwrap();
        library.move_action(index, 2, 0).unwrap();
        assert_eq!(
            library.groups[index].actions,
            vec![b.clone(), a.clone(), a.clone()]
        );
        library.edit_action(index, 1, b.clone()).unwrap();
        assert_eq!(library.remove_action(index, 2).unwrap(), a);
        library.set_group_enabled(index, false).unwrap();
        assert!(!library.groups[index].enabled);
        assert_eq!(library.groups[index].actions, vec![b.clone(), b]);
        assert!(library
            .add_group(ActionGroup {
                name: "workflow".into(),
                enabled: true,
                actions: vec![]
            })
            .is_err());
    }
    #[test]
    fn groups_execute_enabled_order_and_defer_batch_actions() {
        let mut states = vec![state()];
        let groups = vec![
            ActionGroup {
                name: "first".into(),
                enabled: true,
                actions: vec![
                    ActionSpec::Export {
                        template: "test".into(),
                        filename_template: "out.txt".into(),
                        append: false,
                        partition_template: None,
                    },
                    ActionSpec::FormatValue {
                        field: "TITLE".into(),
                        template: "updated".into(),
                    },
                    ActionSpec::GeneratePlaylists {
                        filename_template: "list.m3u8".into(),
                        partition_template: "%album%".into(),
                    },
                ],
            },
            ActionGroup {
                name: "disabled".into(),
                enabled: false,
                actions: vec![ActionSpec::FormatValue {
                    field: "TITLE".into(),
                    template: "wrong".into(),
                }],
            },
        ];
        let mut host = TestHost::default();
        execute_groups(&mut states, &groups, &mut host).unwrap();
        assert_eq!(states[0].fields["TITLE"], ["updated"]);
        assert_eq!(host.events.len(), 2);
        assert!(host.events[0].starts_with("Export"));
        assert!(host
            .events
            .iter()
            .all(|event| event.ends_with("title=updated")));
    }
    #[test]
    fn imported_text_is_available_to_later_actions() {
        let mut states = vec![state()];
        let group = ActionGroup {
            name: "import".into(),
            enabled: true,
            actions: vec![
                ActionSpec::ImportTextFile {
                    field: "LYRICS".into(),
                    filename_template: "%_directory%/lyrics.txt".into(),
                },
                ActionSpec::FormatValue {
                    field: "COMMENT".into(),
                    template: "%lyrics%".into(),
                },
            ],
        };
        let mut host = TestHost::default();
        execute_groups(&mut states, &[group], &mut host).unwrap();
        assert_eq!(states[0].fields["COMMENT"], ["imported\ntext"]);
        assert!(host.events[0].contains("album/lyrics.txt"));
    }
    #[test]
    fn unresolved_import_preflight_prevents_earlier_native_effects() {
        let mut states = vec![state()];
        let group = ActionGroup {
            name: "unsupported".into(),
            enabled: true,
            actions: vec![
                ActionSpec::ImportCover {
                    filename_template: "cover.jpg".into(),
                    picture_type: None,
                    delete_existing: false,
                },
                ActionSpec::UnresolvedMta {
                    type_id: "99".into(),
                    properties: BTreeMap::new(),
                    reason: "unknown ID".into(),
                },
            ],
        };
        let mut host = TestHost::default();
        assert!(execute_groups(&mut states, &[group], &mut host).is_err());
        assert!(host.events.is_empty());
    }
    #[test]
    fn cover_effects_preserve_keep_and_empty_description_distinction() {
        let mut state = state();
        let effect = apply_action_spec(
            &mut state,
            &ActionSpec::SetCoverProperties {
                picture_type: None,
                description: Some(String::new()),
                only_picture_type: Some("CoverFront".into()),
            },
        )
        .unwrap();
        assert_eq!(
            effect,
            [ActionEffect::SetCoverProperties {
                picture_type: None,
                description: Some(String::new()),
                only_picture_type: Some("CoverFront".into())
            }]
        );
    }
    #[test]
    fn invalid_operations_do_not_mutate_fields() {
        let mut state = state();
        let original = state.clone();
        assert!(apply_action_spec(
            &mut state,
            &ActionSpec::SplitField {
                field: "ARTIST".into(),
                separator: String::new()
            }
        )
        .is_err());
        assert_eq!(state, original);
        assert!(apply_action_spec(
            &mut state,
            &ActionSpec::RegexReplace {
                field: "ABSENT".into(),
                pattern: "(".into(),
                replacement: String::new(),
                case_sensitive: true
            }
        )
        .is_err());
        assert_eq!(state, original);
    }
    #[test]
    fn mta_preserves_values_section_order_and_decodes_observed_escapes() {
        let imported=import_mta("Original test".into(),b"[#1]\r\nT=2\r\nF=ARTIST\r\n1=\\xdf\r\n2=ss \r\n3=0\r\n\r\n[#0]\r\nT=1\r\nF=_ALL\r\n1=1\r\n2=(-_\r\n",None).unwrap();
        assert!(imported.issues.is_empty());
        assert!(matches!(
            &imported.group.actions[0],
            ActionSpec::CaseConversion { .. }
        ));
        assert!(
            matches!(&imported.group.actions[1],ActionSpec::Replace{from,to,..}if from=="ß"&&to=="ss ")
        );
    }
    #[test]
    fn imported_action_diagnostics_preserve_quoted_tokens_and_unknown_type_ids() {
        for (text, expected) in [
            (
                "[#0]\nT=Title",
                "Numeric action type Title has no known settings schema",
            ),
            (
                "[#0]\nT=4\nF=TITLE\n1=a\n3=Title",
                "Unknown flag \"Title\" in setting 3",
            ),
            (
                "[#0]\nT=2\nF=TITLE\n1=a\n2=b\n3=Title",
                "Unknown Replace flags \"Title\"",
            ),
            (
                "[#0]\nT=1001\n1=Title\n2=90",
                "Unknown cover format \"Title\"",
            ),
            (
                "[#0]\nT=1001\n1=0\n2=90\n3=100\n4=Title\n5=0",
                "Unknown cover size mode \"Title\"",
            ),
            (
                "[#0]\nT=1001\n1=0\n2=90\n3=100\n4=0\n5=Title",
                "Unknown cover scale mode \"Title\"",
            ),
        ] {
            let imported = import_mta("Unknown".into(), text.as_bytes(), None).unwrap();
            assert_eq!(imported.issues[0].reason, expected);
            assert!(validate_action(&imported.group.actions[0])
                .unwrap_err()
                .ends_with(expected));
        }
    }
    #[test]
    fn mta_unknown_types_flags_and_modes_are_retained_without_guessing() {
        for text in [
            "[#0]\nT=44\nF=TITLE\n1=test",
            "[#0]\nT=1\nF=TITLE\n1=7\n2=",
            "[#0]\nT=4\nF=TITLE\n1=a\n3=yes",
            "[#0]\nT=12\nF=cover.jpg\n1=21",
            "[#0]\nT=5\nF=TITLE\n1=x\n9=extra",
            "[#0]\nT=2\nF=TITLE\n1=a\n2=b\n3=2|1",
        ] {
            let import = import_mta("Unknown".into(), text.as_bytes(), None).unwrap();
            assert!(!import.group.enabled);
            assert_eq!(import.issues.len(), 1);
            assert!(matches!(
                import.group.actions[0],
                ActionSpec::UnresolvedMta { .. }
            ));
        }
        assert!(import_mta("Duplicate".into(), b"[#0]\nT=1\nT=2", None).is_err());
    }
    #[test]
    fn mta_utf16_is_detected_only_by_bom() {
        let text = "[#0]\r\nT=1\r\nF=TITLE\r\n1=1\r\n2=\r\n";
        let bytes: Vec<u8> = std::iter::once(0xfeffu16)
            .chain(text.encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(import_mta("UTF16".into(), &bytes, None)
            .unwrap()
            .issues
            .is_empty());
    }
    #[test]
    fn mta_verified_extra_schemas_import_without_type_guessing() {
        let input=b"[#0]\nT=5\nF=TITLE\n1=%artist%\n[#1]\nT=7\nF=%title%\n1=%artist% / %title%\n[#2]\nT=8\nF=ARTIST\n1=; \n[#3]\nT=9\nF=COMMENT;PICTURE\n[#4]\nT=10\nF=ARTIST;COVER\n[#5]\nT=16\nF=ARTIST\n1=;\n";
        let import = import_mta("Original syntax".into(), input, None).unwrap();
        assert!(import.issues.is_empty());
        assert_eq!(import.group.actions.len(), 6);
        assert!(
            matches!(&import.group.actions[3],ActionSpec::RemoveFields{fields}if fields==&["COMMENT","PICTURE"])
        );
    }
    #[test]
    fn mta_verified_replace_flags_and_escaped_backslashes_are_preserved() {
        let import = import_mta(
            "Flags".into(),
            b"[#0]\nT=2\nF=TITLE\n1=Word\n2=\\\\folder\n3=1|1\n",
            None,
        )
        .unwrap();
        assert!(
            matches!(&import.group.actions[0],ActionSpec::Replace{whole_word:true,case_sensitive:true,to,..}if to==r"\folder")
        );
        assert_eq!(mta_value(r"\\x24").unwrap(), r"\x24");
    }
    #[test]
    fn presets_capture_only_selection_and_follow_group_rename() {
        let mut library = ActionLibrary::default();
        let a = library
            .add_group(ActionGroup {
                name: "A".into(),
                enabled: true,
                actions: vec![],
            })
            .unwrap();
        library
            .add_group(ActionGroup {
                name: "B".into(),
                enabled: false,
                actions: vec![],
            })
            .unwrap();
        library.save_selection_preset("Only A".into()).unwrap();
        library.rename_group(a, "Renamed".into()).unwrap();
        library.groups[0].enabled = false;
        library.groups[1].enabled = true;
        library.apply_selection_preset("Only A").unwrap();
        assert!(library.groups[0].enabled);
        assert!(!library.groups[1].enabled);
        library.move_group(0, 1).unwrap();
        assert_eq!(library.groups[1].name, "Renamed");
        library.remove_group(1).unwrap();
        assert!(library.presets[0].enabled_groups.is_empty());
    }
    #[test]
    fn menu_names_support_documented_submenu_mnemonics() {
        assert_eq!(
            group_menu_segments("&Default values#Set &COMMENT"),
            vec![
                ActionMenuSegment {
                    label: "Default values".into(),
                    mnemonic: Some('d')
                },
                ActionMenuSegment {
                    label: "Set COMMENT".into(),
                    mnemonic: Some('c')
                }
            ]
        );
    }
    #[test]
    fn action_specs_serde_roundtrip_without_silent_unknown_settings() {
        let action = ActionSpec::AdjustCover {
            format: CoverImageFormat::Jpeg,
            quality: 90,
            resize: Some(CoverResize {
                pixels: 600,
                size_mode: CoverSizeMode::Maximum,
                scale_mode: CoverScaleMode::CropSquare,
            }),
        };
        let text = serde_json::to_string(&action).unwrap();
        assert_eq!(serde_json::from_str::<ActionSpec>(&text).unwrap(), action);
        assert!(serde_json::from_str::<ActionSpec>(
            r#"{"type":"format_value","field":"TITLE","template":"text","unknown":true}"#
        )
        .is_err());
    }
    #[test]
    fn import_cover_options_default_for_saved_libraries_and_reach_the_effect() {
        let saved: ActionSpec =
            serde_json::from_str(r#"{"type":"import_cover","filename_template":"cover.jpg"}"#)
                .unwrap();
        assert_eq!(
            saved,
            ActionSpec::ImportCover {
                filename_template: "cover.jpg".into(),
                picture_type: None,
                delete_existing: false,
            }
        );
        let mut current = state();
        let effects = apply_action_spec(
            &mut current,
            &ActionSpec::ImportCover {
                filename_template: "%_directory%/back.png".into(),
                picture_type: Some("CoverBack".into()),
                delete_existing: true,
            },
        )
        .unwrap();
        assert_eq!(
            effects,
            vec![ActionEffect::ImportCover {
                filename: "album/back.png".into(),
                picture_type: Some("CoverBack".into()),
                delete_existing: true,
            }]
        );
    }
    #[test]
    fn unknown_cover_types_are_rejected_before_any_effect() {
        for action in [
            ActionSpec::ImportCover {
                filename_template: "cover.jpg".into(),
                picture_type: Some("Poster".into()),
                delete_existing: true,
            },
            ActionSpec::ExportCover {
                filename_template: "cover".into(),
                picture_type: Some("Poster".into()),
                export_duplicates: false,
            },
            ActionSpec::SetCoverProperties {
                picture_type: None,
                description: None,
                only_picture_type: Some("Poster".into()),
            },
        ] {
            assert!(validate_action(&action).unwrap_err().contains("Poster"));
        }
        assert!(validate_action(&ActionSpec::SetCoverProperties {
            picture_type: Some("CoverFront".into()),
            description: Some(String::new()),
            only_picture_type: Some("Other".into()),
        })
        .is_ok());
    }
    #[test]
    fn mta_export_roundtrips_every_verified_schema_and_unresolved_record() {
        let mut unresolved = BTreeMap::new();
        unresolved.insert("T".to_string(), "44".to_string());
        unresolved.insert("F".to_string(), "TITLE".to_string());
        unresolved.insert("1".to_string(), "1251".to_string());
        let group = ActionGroup {
            name: "Shared".into(),
            enabled: false,
            actions: vec![
                ActionSpec::CaseConversion {
                    field: "_ALL".into(),
                    mode: CaseMode::Mixed,
                    word_separators: "(-_".into(),
                },
                ActionSpec::Replace {
                    field: "_FILENAME".into(),
                    from: "ß".into(),
                    to: "ss \\x41 \\d \\é 猫 😀".into(),
                    whole_word: true,
                    case_sensitive: false,
                },
                ActionSpec::FormatValue {
                    field: "ALBUMARTIST".into(),
                    template: "$if2(%albumartist%,%artist%)".into(),
                },
                ActionSpec::GuessValues {
                    source: "%title%".into(),
                    pattern: "%artist% - %title%".into(),
                },
                ActionSpec::MergeDuplicateFields {
                    field: "_TAG".into(),
                    separator: "\\\\".into(),
                },
                ActionSpec::SplitField {
                    field: "ARTIST".into(),
                    separator: "; ".into(),
                },
                ActionSpec::RemoveFields {
                    fields: vec!["COMMENT".into(), "COVER".into()],
                },
                ActionSpec::RemoveFieldsExcept {
                    fields: vec!["TITLE".into(), "ARTIST".into()],
                },
                ActionSpec::UnresolvedMta {
                    type_id: "44".into(),
                    properties: unresolved,
                    reason: "unverified".into(),
                },
            ],
        };
        let text = export_mta(&group).unwrap();
        assert!(text.starts_with("[#0]\r\nT=1\r\nF=_ALL\r\n1=1\r\n2=(-_\r\n\r\n[#1]\r\nT=2\r\n"));
        assert!(text.contains("1=\\xdf\r\n"));
        assert!(text.contains("3=1|0\r\n"));
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        let imported = import_mta("Shared".into(), &bytes, None).unwrap();
        assert_eq!(imported.issues.len(), 1);
        assert_eq!(imported.group.actions[..8], group.actions[..8]);
        assert_eq!(
            imported.records[8].properties,
            match &group.actions[8] {
                ActionSpec::UnresolvedMta { properties, .. } => properties.clone(),
                _ => unreachable!(),
            }
        );
    }
    #[test]
    fn mta_roundtrips_every_action_type() {
        let actions = vec![
            ActionSpec::CaseConversion {
                field: "TITLE".into(),
                mode: CaseMode::Upper,
                word_separators: String::new(),
            },
            ActionSpec::CaseConversion {
                field: "TITLE".into(),
                mode: CaseMode::Lower,
                word_separators: String::new(),
            },
            ActionSpec::CaseConversion {
                field: "_FILENAME".into(),
                mode: CaseMode::Sentence,
                word_separators: " ".into(),
            },
            ActionSpec::RegexReplace {
                field: "TITLE".into(),
                pattern: "\\s+\\(live\\)$".into(),
                replacement: String::new(),
                case_sensitive: true,
            },
            ActionSpec::RemoveDuplicateFields {
                field: "_TAG".into(),
                only_same_content: true,
            },
            ActionSpec::ConvertCodepage {
                field: "_ALL".into(),
                codepage: "1251".into(),
            },
            ActionSpec::ImportCover {
                filename_template: "%_folderpath%folder.jpg".into(),
                picture_type: Some("CoverBack".into()),
                delete_existing: true,
            },
            ActionSpec::ImportCover {
                filename_template: "cover.jpg".into(),
                picture_type: None,
                delete_existing: false,
            },
            ActionSpec::ExportCover {
                filename_template: "%album%".into(),
                picture_type: Some("CoverFront".into()),
                export_duplicates: true,
            },
            ActionSpec::ImportTextFile {
                field: "LYRICS".into(),
                filename_template: "%_filename%.txt".into(),
            },
            ActionSpec::Export {
                template: "HTML List".into(),
                filename_template: "%_folderpath%list.html".into(),
                append: true,
                partition_template: Some("%album%".into()),
            },
            ActionSpec::Export {
                template: "Text".into(),
                filename_template: String::new(),
                append: false,
                partition_template: None,
            },
            ActionSpec::AdjustCover {
                format: CoverImageFormat::Jpeg,
                quality: 85,
                resize: Some(CoverResize {
                    pixels: 500,
                    size_mode: CoverSizeMode::Minimum,
                    scale_mode: CoverScaleMode::CropSquare,
                }),
            },
            ActionSpec::AdjustCover {
                format: CoverImageFormat::Original,
                quality: 90,
                resize: None,
            },
            ActionSpec::SetCoverProperties {
                picture_type: Some("CoverFront".into()),
                description: Some(String::new()),
                only_picture_type: Some("Other".into()),
            },
            ActionSpec::SetCoverProperties {
                picture_type: None,
                description: None,
                only_picture_type: None,
            },
            ActionSpec::GeneratePlaylists {
                filename_template: "%album%.m3u".into(),
                partition_template: "%_folderpath%".into(),
            },
        ];
        let group = ActionGroup {
            name: "Every type".into(),
            enabled: true,
            actions: actions.clone(),
        };
        let text = export_mta(&group).unwrap();
        assert!(text.contains("[#6]\r\nT=12\r\nF=%_folderpath%folder.jpg\r\n1=4\r\n2=1\r\n"));
        assert!(text.contains("[#0]\r\nT=1\r\nF=TITLE\r\n1=3\r\n2=\r\n"));
        let imported = import_mta("Every type".into(), text.as_bytes(), None).unwrap();
        assert!(imported.issues.is_empty(), "{:?}", imported.issues);
        assert!(imported.group.enabled);
        assert_eq!(imported.group.actions, actions);
    }

    #[test]
    fn mta_export_rejects_line_breaks() {
        let group = |action| ActionGroup {
            name: "x".into(),
            enabled: true,
            actions: vec![action],
        };
        assert!(export_mta(&group(ActionSpec::FormatValue {
            field: "COMMENT".into(),
            template: "a\nb".into(),
        }))
        .unwrap_err()
        .contains("Line breaks"));
    }
    #[test]
    fn import_issue_companions_preserve_three_owned_reasons_and_real_foreign_validation() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/import-diagnostics.json"
        ))
        .unwrap();
        let input = &fixture["mta"];
        let report = import_mta_diagnostic(
            input["name"].as_str().unwrap().into(),
            input["text"].as_str().unwrap().as_bytes(),
            None,
        )
        .unwrap();
        let legacy = import_mta(
            input["name"].as_str().unwrap().into(),
            input["text"].as_str().unwrap().as_bytes(),
            None,
        )
        .unwrap();
        assert_eq!(report.result, legacy);
        assert_eq!(
            serde_json::to_value(&report.result.issues).unwrap(),
            input["issues"]
        );
        assert_eq!(report.issue_diagnostics.len(), 3);
        for (position, item) in report.issue_diagnostics.iter().enumerate() {
            assert_eq!(item.index, position);
            assert_eq!(item.cause.to_string(), legacy.issues[position].reason);
            assert!(matches!(
                item.cause,
                ErrorCause::Owned { .. } | ErrorCause::OwnedText { .. }
            ));
        }
        assert!(!legacy.group.enabled);
        assert_eq!(legacy.group.actions.len(), 4);
        let regex =
            import_mta_diagnostic("Help".into(), b"[#0]\nT=4\nF=TITLE\n1=(\n2=\n3=0\n", None)
                .unwrap();
        assert_eq!(regex.issue_diagnostics.len(), 1);
        assert!(matches!(
            regex.issue_diagnostics[0].cause,
            ErrorCause::External { .. }
        ));
        assert_eq!(
            regex.issue_diagnostics[0].cause.to_string(),
            regex.result.issues[0].reason
        );
    }
}
