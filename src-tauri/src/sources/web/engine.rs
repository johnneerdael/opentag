//! The parser language of web source scripts (`[ParserScriptIndex]` and
//! `[ParserScriptAlbum]`), implemented from the public Mp3tag documentation.
//!
//! A cursor (line and position) moves over the downloaded text; Say commands
//! append to the current output buffer. In JSON mode the JSON commands select
//! values from the parsed document and place them on the input, where the
//! usual Say commands read them.
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fmt;

/// Commands executed per run before the script is stopped as an endless loop.
const STEP_LIMIT: usize = 2_000_000;
/// Total size of all output buffers.
const OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
/// Cumulative bytes appended by retained-line joins during one run.
const JOIN_GROWTH_LIMIT: usize = 16 * 1024 * 1024;
/// Debug files without an explicit size limit stop growing at this size.
const DEBUG_LIMIT: usize = 10 * 1024 * 1024;
/// The output Say commands write to until OutputTo names another; the index
/// script's results are read from it (published scripts return to it with
/// `OutputTo "Output"`).
pub const DEFAULT_OUTPUT: &str = "OUTPUT";
/// What `$verticalBar()` produces: a pipe that is content rather than the
/// separator between track values. It becomes `|` once values are split.
pub const VERTICAL_BAR: char = '\u{E000}';

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptError {
    /// Line in the source file; 0 when the error is not tied to a command.
    pub line: usize,
    pub command: String,
    pub message: String,
}
impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(
                f,
                "line {line} ({text}): {error}",
                line = self.line,
                text = self.command,
                error = self.message
            )
        }
    }
}

#[derive(Clone, Debug)]
struct Command {
    /// Lower-case command name.
    name: String,
    args: Vec<String>,
    line: usize,
    text: String,
    /// Matching Else/Endif, While→Do, json_foreach↔json_foreach_end.
    target: usize,
}

/// A compiled parser script.
#[derive(Clone, Debug, Default)]
pub struct Script {
    commands: Vec<Command>,
}

/// Command names with their minimum and maximum number of parameters.
const COMMANDS: &[(&str, usize, usize)] = &[
    ("findline", 1, 3),
    ("findlinenocase", 1, 3),
    ("findinline", 1, 3),
    ("gotochar", 1, 1),
    ("gotoline", 1, 1),
    ("movechar", 1, 1),
    ("moveline", 1, 2),
    ("say", 1, 1),
    ("sayuntil", 1, 1),
    ("sayuntilml", 1, 1),
    ("sayrest", 0, 0),
    ("saynchars", 1, 1),
    ("saynextnumber", 0, 0),
    ("saynextword", 0, 0),
    ("sayoutput", 1, 1),
    ("saynewline", 0, 0),
    ("saydate", 0, 1),
    ("sayduration", 0, 3),
    ("sayregexp", 1, 3),
    ("sayformat", 1, 2),
    ("sayvar", 1, 1),
    ("set", 1, 2),
    ("skipchars", 1, 1),
    ("insert", 1, 1),
    ("use", 1, 1),
    ("useformat", 1, 2),
    ("if", 1, 1),
    ("ifnot", 1, 1),
    ("ifoutput", 1, 1),
    ("ifnotoutput", 1, 1),
    ("ifgreater", 1, 1),
    ("ifless", 1, 1),
    ("ifvar", 2, 2),
    ("ifnotvar", 2, 2),
    ("else", 0, 0),
    ("endif", 0, 0),
    ("outputto", 1, 1),
    ("do", 0, 0),
    ("while", 1, 2),
    ("replace", 2, 2),
    ("regexpreplace", 2, 2),
    ("joinuntil", 1, 1),
    ("joinlines", 1, 1),
    ("killtag", 1, 2),
    ("trim", 1, 1),
    ("unspace", 0, 0),
    ("debug", 1, 3),
    ("debugwriteinput", 0, 1),
    ("json", 1, 2),
    ("json_foreach", 1, 1),
    ("json_foreach_reverse", 1, 1),
    ("json_foreach_counter", 0, 1),
    ("json_foreach_end", 0, 0),
    ("json_select_object", 1, 1),
    ("json_unselect_object", 0, 0),
    ("json_select", 1, 1),
    ("json_select_array", 2, 3),
    ("json_select_many", 2, 6),
    ("json_select_many_count", 1, 3),
];

/// Every command the interpreter accepts.
#[cfg(test)]
pub fn command_names() -> impl Iterator<Item = &'static str> {
    COMMANDS.iter().map(|(name, _, _)| *name)
}

const IFS: &[&str] = &[
    "if",
    "ifnot",
    "ifoutput",
    "ifnotoutput",
    "ifgreater",
    "ifless",
    "ifvar",
    "ifnotvar",
];

/// Split a command line into its name and parameters. Parameters are quoted
/// strings or bare words such as numbers; a `#` outside quotes starts a
/// comment. Inside quotes `\"` is a quote and `\\` a backslash; any other
/// backslash stays, so regular expressions keep `\s` or `\d` (published
/// scripts write `"\\s*\\(\\d+\\)$"` and `"\s+"` for the same pattern).
fn tokenize(line: &str) -> Result<(String, Vec<String>), String> {
    let mut chars = line.trim().chars().peekable();
    let mut name = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            break;
        }
        name.push(c);
        chars.next();
    }
    let mut args = Vec::new();
    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        match chars.next() {
            None | Some('#') => break,
            Some('"') => {
                let mut arg = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    match c {
                        '\\' if matches!(chars.peek(), Some('"' | '\\')) => {
                            arg.extend(chars.next());
                        }
                        '"' => {
                            closed = true;
                            break;
                        }
                        c => arg.push(c),
                    }
                }
                if !closed {
                    return Err("a quoted parameter is not closed".into());
                }
                args.push(arg);
            }
            Some(first) => {
                let mut arg = first.to_string();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() {
                        break;
                    }
                    arg.push(c);
                    chars.next();
                }
                args.push(arg);
            }
        }
    }
    Ok((name, args))
}

impl Script {
    /// Compile `(line number, text)` pairs. Blank lines and `#` comments are
    /// skipped; blocks (If/Else/Endif, Do/While, json_foreach) must balance.
    pub fn compile(lines: &[(usize, String)]) -> Result<Script, ScriptError> {
        let mut commands = Vec::new();
        for (line, text) in lines {
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let fail = |message: String| ScriptError {
                line: *line,
                command: trimmed.to_owned(),
                message,
            };
            let (name, args) = tokenize(trimmed).map_err(fail)?;
            let lower = name.to_ascii_lowercase();
            let Some((_, min, max)) = COMMANDS.iter().find(|(known, _, _)| *known == lower) else {
                return Err(fail(format!("{name} is not a known command")));
            };
            if args.len() < *min || args.len() > *max {
                return Err(fail(if min == max {
                    if *min == 1 {
                        format!("{name} takes {min} parameter")
                    } else {
                        format!("{name} takes {min} parameters")
                    }
                } else {
                    format!("{name} takes {min} to {max} parameters")
                }));
            }
            commands.push(Command {
                name: lower,
                args,
                line: *line,
                text: trimmed.to_owned(),
                target: usize::MAX,
            });
        }
        let mut blocks: Vec<usize> = Vec::new();
        let unbalanced = |command: &Command, message: &str| ScriptError {
            line: command.line,
            command: command.text.clone(),
            message: message.into(),
        };
        for index in 0..commands.len() {
            let name = commands[index].name.as_str();
            if IFS.contains(&name)
                || name == "do"
                || name == "json_foreach"
                || name == "json_foreach_reverse"
            {
                blocks.push(index);
                continue;
            }
            let open = blocks.last().map(|&i| commands[i].name.as_str());
            match name {
                "else" => {
                    let Some(&start) = blocks
                        .last()
                        .filter(|_| open.is_some_and(|o| IFS.contains(&o)))
                    else {
                        return Err(unbalanced(&commands[index], "Else without If"));
                    };
                    if commands[start].target != usize::MAX {
                        return Err(unbalanced(
                            &commands[index],
                            "a second Else for the same If",
                        ));
                    }
                    commands[start].target = index;
                }
                "endif" => {
                    if !open.is_some_and(|o| IFS.contains(&o)) {
                        return Err(unbalanced(&commands[index], "Endif without If"));
                    }
                    let start = blocks.pop().unwrap_or_default();
                    match commands[start].target {
                        usize::MAX => commands[start].target = index,
                        branch => commands[branch].target = index,
                    }
                    commands[index].target = index;
                }
                "while" => {
                    if open != Some("do") {
                        return Err(unbalanced(&commands[index], "While without Do"));
                    }
                    commands[index].target = blocks.pop().unwrap_or_default();
                }
                "json_foreach_end" => {
                    if !matches!(open, Some("json_foreach" | "json_foreach_reverse")) {
                        return Err(unbalanced(
                            &commands[index],
                            "json_foreach_end without json_foreach",
                        ));
                    }
                    let start = blocks.pop().unwrap_or_default();
                    commands[start].target = index;
                    commands[index].target = start;
                }
                _ => {}
            }
        }
        if let Some(&open) = blocks.last() {
            let command = &commands[open];
            let name = match command.name.as_str() {
                "do" => "While",
                "json_foreach" | "json_foreach_reverse" => "json_foreach_end",
                _ => "Endif",
            };
            return Err(unbalanced(command, &format!("this block has no {name}")));
        }
        Ok(Script { commands })
    }
}

/// Values a script can read besides its input.
#[derive(Default)]
pub struct Environment<'a> {
    /// Tag source configuration (IfVar, IfNotVar, SayVar).
    pub vars: BTreeMap<String, String>,
    /// The address of the downloaded page (`CurrentUrl`).
    pub current_url: &'a str,
    /// Trace every command to this debug file name (Options → Tag Sources).
    pub trace: Option<String>,
}

/// A file a script asked to write with Debug or DebugWriteInput.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugFile {
    pub name: String,
    pub content: String,
}

pub struct Outcome {
    /// Output buffers by upper-case name; the index script's results are in
    /// [`DEFAULT_OUTPUT`].
    pub result: Result<BTreeMap<String, String>, ScriptError>,
    pub files: Vec<DebugFile>,
}

struct Each {
    items: Vec<Value>,
    /// Member names when iterating an object; json_foreach_counter emits them.
    keys: Option<Vec<String>>,
    index: usize,
    /// JSON context depth when the loop started.
    depth: usize,
}

struct Json {
    root: Value,
    /// The text input and cursor when JSON mode started; `json "off"`
    /// returns to them, so a script can go on reading the page.
    text: (Vec<String>, usize, usize),
    /// Selected objects and loop elements; the last one is current.
    stack: Vec<Value>,
    loops: Vec<Each>,
}
impl Json {
    fn current(&self) -> &Value {
        self.stack.last().unwrap_or(&self.root)
    }
}

struct Trace {
    name: String,
    limit: usize,
    text: String,
}

struct Machine<'a> {
    env: &'a Environment<'a>,
    original: &'a str,
    lines: Vec<String>,
    line: usize,
    /// Byte offset in the current line, always on a character boundary.
    pos: usize,
    outputs: BTreeMap<String, String>,
    current: String,
    size: usize,
    joined_bytes: usize,
    trim: bool,
    json: Option<Json>,
    repeats: HashMap<usize, usize>,
    trace: Option<Trace>,
    files: Vec<DebugFile>,
}

fn split_lines(text: &str) -> Vec<String> {
    text.split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_owned())
        .collect()
}

fn byte_of_char(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(i, _)| i)
}

/// A number at the start of `text` (after optional spaces): optional minus,
/// digits and an optional decimal part. Returns it and the bytes consumed.
fn leading_number(text: &str) -> Option<(String, usize)> {
    let start = text.len() - text.trim_start().len();
    let rest = &text[start..];
    let bytes = rest.as_bytes();
    let mut end = usize::from(bytes.first() == Some(&b'-'));
    let digits = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == digits {
        return None;
    }
    if end + 1 < bytes.len() && bytes[end] == b'.' && bytes[end + 1].is_ascii_digit() {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    Some((rest[..end].to_owned(), start + end))
}

/// The text of a JSON value as it is placed on the input.
fn json_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        other => other.to_string(),
    }
}

/// `key` in an object; an empty key is the value itself. A key that is not
/// present but contains dots is followed as a path (`artist.name`, `images.0`).
fn json_lookup<'v>(value: &'v Value, key: &str) -> Option<&'v Value> {
    if key.is_empty() {
        return Some(value);
    }
    if let Some(found) = value.get(key) {
        return Some(found);
    }
    if !key.contains('.') {
        return None;
    }
    key.split('.')
        .try_fold(value, |current, part| match current {
            Value::Array(items) => part.parse::<usize>().ok().and_then(|i| items.get(i)),
            _ => current.get(part),
        })
}

fn parse_number(text: &str, what: &str) -> Result<i64, String> {
    text.trim()
        .parse::<i64>()
        .map_err(|_| format!("{what} must be a whole number, not \"{text}\""))
}

fn flag(args: &[String], index: usize) -> bool {
    args.get(index).is_some_and(|value| value.trim() == "1")
}

/// Replace `<tag …>`, `</tag>` and `<tag/>` (any tag for `*`) in `line`.
fn kill_tag(line: &str, tag: &str, replacement: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find('<') {
        output.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let name_start = usize::from(after.starts_with('/'));
        let name_len = after[name_start..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == ':' || c == '-'))
            .unwrap_or(after.len() - name_start);
        let name = &after[name_start..name_start + name_len];
        let close = after.find('>');
        let matches = !name.is_empty() && (tag == "*" || name.eq_ignore_ascii_case(tag));
        match close {
            Some(close) if matches => {
                output.push_str(replacement);
                rest = &after[close + 1..];
            }
            _ => {
                output.push('<');
                rest = after;
            }
        }
    }
    output.push_str(rest);
    output
}

/// `seconds` as `m:ss`, `h:mm:ss` with `hours`, plus `.mmm` with `millis`.
fn format_duration(milliseconds: i64, hours: bool, millis: bool) -> String {
    let total = milliseconds.abs();
    let sign = if milliseconds < 0 { "-" } else { "" };
    let (h, m, s, ms) = (
        total / 3_600_000,
        total / 60_000 % 60,
        total / 1000 % 60,
        total % 1000,
    );
    let mut text = if hours {
        format!("{sign}{h}:{m:02}:{s:02}")
    } else {
        format!("{sign}{}:{s:02}", total / 60_000)
    };
    if millis {
        text.push_str(&format!(".{ms:03}"));
    }
    text
}

/// `YYYY-MM-DD` of a Unix time in seconds (proleptic Gregorian, UTC).
fn format_date(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Evaluate a format string (SayFormat, UseFormat) with the output buffers as
/// fields and the input as `%_current%`.
fn format_value(
    template: &str,
    current: &str,
    outputs: Option<&BTreeMap<String, String>>,
) -> Result<String, String> {
    struct Bar;
    impl tag_core::ScriptExtension for Bar {
        fn call(
            &self,
            name: &str,
            _args: &[tag_core::ScriptValue],
        ) -> Result<tag_core::ScriptValue, String> {
            match name.to_ascii_lowercase().as_str() {
                "verticalbar" => Ok(tag_core::ScriptValue {
                    text: VERTICAL_BAR.to_string(),
                    present: true,
                    truth: true,
                }),
                name => Err(format!("${name} is not available here")),
            }
        }
    }
    let mut fields = tag_core::Fields::new();
    for (name, value) in outputs.into_iter().flatten() {
        if !name.is_empty() {
            fields.insert(name.clone(), vec![value.clone()]);
        }
    }
    fields.insert("_CURRENT".into(), vec![current.to_owned()]);
    let compiled = tag_core::CompiledTemplate::compile_with_functions(
        template,
        &[tag_core::ScriptFunctionSpec {
            name: "verticalBar".into(),
            min_args: 0,
            max_args: 0,
        }],
    )?;
    let mut context = tag_core::EvalContext::new(&fields);
    context.extension = Some(&Bar);
    compiled.evaluate_with_context(&context)
}

impl<'a> Machine<'a> {
    fn new(input: &'a str, env: &'a Environment<'a>) -> Self {
        Machine {
            env,
            original: input,
            lines: split_lines(input),
            line: 0,
            pos: 0,
            outputs: BTreeMap::new(),
            current: DEFAULT_OUTPUT.into(),
            size: 0,
            joined_bytes: 0,
            trim: true,
            json: None,
            repeats: HashMap::new(),
            trace: env.trace.clone().map(|name| Trace {
                name,
                limit: DEBUG_LIMIT,
                text: String::new(),
            }),
            files: Vec::new(),
        }
    }

    fn line_text(&self) -> &str {
        &self.lines[self.line]
    }
    fn rest(&self) -> &str {
        &self.lines[self.line][self.pos..]
    }
    fn input_text(&self) -> String {
        self.lines.join("\n")
    }
    fn set_input(&mut self, text: &str) {
        self.lines = split_lines(text);
        self.line = 0;
        self.pos = 0;
    }
    fn clamp(&mut self) {
        self.line = self.line.min(self.lines.len() - 1);
        let line = &self.lines[self.line];
        let mut pos = self.pos.min(line.len());
        while !line.is_char_boundary(pos) {
            pos -= 1;
        }
        self.pos = pos;
    }

    fn say(&mut self, text: &str) -> Result<(), String> {
        self.size += text.len();
        if self.size > OUTPUT_LIMIT {
            return Err("the output is larger than 16 MB".into());
        }
        self.outputs
            .entry(self.current.clone())
            .or_default()
            .push_str(text);
        Ok(())
    }
    /// Text read from the input, trimmed unless `Trim "off"`.
    fn say_read(&mut self, text: &str) -> Result<(), String> {
        let text = if self.trim { text.trim() } else { text };
        self.say(text)
    }

    fn output(&self, name: &str) -> Option<String> {
        if name.eq_ignore_ascii_case("CurrentUrl") {
            return Some(self.env.current_url.to_owned());
        }
        self.outputs.get(&name.to_uppercase()).cloned()
    }

    /// `%name%` in a parameter is the content of that output buffer. Other
    /// percent signs (`100%`, `%20`) stay as they are.
    fn expand(&self, text: &str) -> String {
        if !text.contains('%') {
            return text.to_owned();
        }
        let mut output = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(start) = rest.find('%') {
            output.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            match after.find('%').and_then(|end| {
                let name = &after[..end];
                (!name.is_empty())
                    .then(|| self.output(name))
                    .flatten()
                    .map(|value| (end, value))
            }) {
                Some((end, value)) => {
                    output.push_str(&value);
                    rest = &after[end + 1..];
                }
                None => {
                    output.push('%');
                    rest = after;
                }
            }
        }
        output.push_str(rest);
        output
    }

    fn json(&mut self) -> Result<&mut Json, String> {
        self.json
            .as_mut()
            .ok_or_else(|| "JSON mode is off; start it with json \"on\"".to_string())
    }
    fn json_array(&mut self, key: &str) -> Result<Vec<Value>, String> {
        Ok(match json_lookup(self.json()?.current(), key) {
            Some(Value::Array(items)) => items.clone(),
            _ => Vec::new(),
        })
    }

    /// `If ""` and `IfNot ""` test whether the current line has nothing left
    /// at the cursor (scripts use them on a selected JSON value).
    fn starts_with(&self, text: &str) -> bool {
        if text.is_empty() {
            self.rest().is_empty()
        } else {
            self.rest().starts_with(text)
        }
    }

    fn find_line(&mut self, needle: &str, nth: usize, ignore_case: bool) -> bool {
        let needle_lower = needle.to_lowercase();
        let contains = |text: &str| {
            if ignore_case {
                text.to_lowercase().contains(&needle_lower)
            } else {
                text.contains(needle)
            }
        };
        let mut seen = 0;
        for line in self.line..self.lines.len() {
            let text = if line == self.line {
                self.rest()
            } else {
                &self.lines[line]
            };
            if contains(text) {
                seen += 1;
                if seen == nth {
                    self.line = line;
                    self.pos = 0;
                    return true;
                }
            }
        }
        false
    }

    fn number_here(&self) -> Option<f64> {
        leading_number(self.rest()).and_then(|(number, _)| number.parse().ok())
    }
    /// The number at the cursor for SayDate/SayDuration, consumed.
    fn take_number(&mut self) -> Result<f64, String> {
        let (number, used) = leading_number(self.rest()).ok_or_else(|| {
            format!(
                "no number at the current position: \"{}\"",
                excerpt(self.rest())
            )
        })?;
        self.pos += used;
        number
            .parse()
            .map_err(|_| format!("\"{value}\" is not a number", value = number))
    }

    fn condition(&self, command: &Command, args: &[String]) -> Result<bool, String> {
        let var = |key: &str| {
            self.env
                .vars
                .get(key)
                .map(String::as_str)
                .unwrap_or_default()
        };
        Ok(match command.name.as_str() {
            "if" | "while" => self.starts_with(&args[0]),
            "ifnot" => !self.starts_with(&args[0]),
            "ifoutput" => self.output(&args[0]).is_some_and(|v| !v.is_empty()),
            "ifnotoutput" => self.output(&args[0]).is_none_or(|v| v.is_empty()),
            "ifgreater" | "ifless" => {
                let limit: f64 = args[0]
                    .trim()
                    .parse()
                    .map_err(|_| format!("\"{}\" is not a number", args[0]))?;
                match self.number_here() {
                    Some(n) if command.name == "ifgreater" => n > limit,
                    Some(n) => n < limit,
                    None => false,
                }
            }
            "ifvar" => var(&args[0]).eq_ignore_ascii_case(&args[1]),
            "ifnotvar" => !var(&args[0]).eq_ignore_ascii_case(&args[1]),
            _ => false,
        })
    }

    fn trace_step(&mut self, command: &Command) {
        if self.trace.is_none() {
            return;
        }
        let mut entry = format!("{:>5}: {}\n", command.line, command.text);
        let line = if self.line < self.lines.len() {
            self.rest()
        } else {
            ""
        };
        entry.push_str(&format!(
            "       input line {} position {}: {}\n",
            self.line + 1,
            self.lines
                .get(self.line)
                .map_or(0, |l| l[..self.pos.min(l.len())].chars().count())
                + 1,
            excerpt(line)
        ));
        if let Some(value) = self.outputs.get(&self.current) {
            entry.push_str(&format!(
                "       output {:?}: {}\n",
                self.current,
                excerpt_tail(value)
            ));
        }
        if let Some(trace) = self.trace.as_mut() {
            if trace.text.len() + entry.len() <= trace.limit {
                trace.text.push_str(&entry);
            }
        }
    }

    fn finish_trace(&mut self) {
        if let Some(trace) = self.trace.take() {
            self.files.push(DebugFile {
                name: trace.name,
                content: trace.text,
            });
        }
    }

    /// Run one command; returns the next command index.
    fn step(&mut self, script: &Script, index: usize) -> Result<usize, String> {
        let command = &script.commands[index];
        let format_first = matches!(command.name.as_str(), "sayformat" | "useformat");
        let args: Vec<String> = command
            .args
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                if format_first && i == 0 {
                    arg.clone()
                } else {
                    self.expand(arg)
                }
            })
            .collect();
        let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or_default();
        let mut next = index + 1;
        match command.name.as_str() {
            "findline" | "findlinenocase" => {
                let nth = args
                    .get(1)
                    .map(|n| parse_number(n, "The occurrence"))
                    .transpose()?
                    .unwrap_or(1)
                    .max(1);
                if !self.find_line(arg(0), nth as usize, command.name == "findlinenocase") {
                    if !flag(&args, 2) {
                        return Err(format!("\"{}\" was not found", arg(0)));
                    }
                    // The search ran to the end of the input, where `If ""`
                    // holds (scripts test that before trying another marker).
                    self.line = self.lines.len() - 1;
                    self.pos = self.line_text().len();
                }
            }
            "findinline" => {
                let nth = args
                    .get(1)
                    .map(|n| parse_number(n, "The occurrence"))
                    .transpose()?
                    .unwrap_or(1)
                    .max(1) as usize;
                let needle = arg(0);
                let mut at = self.pos;
                let mut found = 0;
                while !needle.is_empty() {
                    match self.line_text()[at..].find(needle) {
                        Some(offset) => {
                            at += offset + needle.len();
                            found += 1;
                            if found == nth {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                if found == nth || needle.is_empty() {
                    self.pos = at;
                } else if !flag(&args, 2) {
                    return Err(format!(
                        "\"{value}\" was not found in the current line",
                        value = needle
                    ));
                } else {
                    self.pos = self.line_text().len();
                }
            }
            "gotochar" => {
                let n = parse_number(arg(0), "The position")?.max(1) as usize;
                self.pos = byte_of_char(self.line_text(), n - 1);
            }
            "gotoline" => {
                let n = parse_number(arg(0), "The line")?.max(1) as usize;
                if n > self.lines.len() {
                    return Err(format!("the input has only {} lines", self.lines.len()));
                }
                self.line = n - 1;
                self.pos = 0;
            }
            "movechar" => {
                let n = parse_number(arg(0), "The distance")?;
                let before = self.line_text()[..self.pos].chars().count() as i64;
                let target = (before + n).max(0) as usize;
                self.pos = byte_of_char(self.line_text(), target);
            }
            "moveline" => {
                let n = parse_number(arg(0), "The distance")?;
                let target = self.line as i64 + n;
                if target < 0 || target >= self.lines.len() as i64 {
                    if !flag(&args, 1) {
                        return Err(format!(
                            "moving {n} lines leaves the input (line {} of {})",
                            self.line + 1,
                            self.lines.len()
                        ));
                    }
                    self.line = target.clamp(0, self.lines.len() as i64 - 1) as usize;
                } else {
                    self.line = target as usize;
                }
                self.pos = 0;
            }
            "say" => self.say(arg(0))?,
            "sayuntil" => {
                let rest = self.rest();
                let end = (!arg(0).is_empty())
                    .then(|| rest.find(arg(0)))
                    .flatten()
                    .unwrap_or(rest.len());
                let text = rest[..end].to_owned();
                self.pos += end;
                self.say_read(&text)?;
            }
            "sayuntilml" => {
                let needle = arg(0);
                let mut parts = Vec::new();
                let mut found = false;
                while !found {
                    let rest = self.rest();
                    match (!needle.is_empty()).then(|| rest.find(needle)).flatten() {
                        Some(end) => {
                            parts.push(rest[..end].to_owned());
                            self.pos += end;
                            found = true;
                        }
                        None => {
                            parts.push(rest.to_owned());
                            if self.line + 1 >= self.lines.len() {
                                self.pos = self.line_text().len();
                                break;
                            }
                            self.line += 1;
                            self.pos = 0;
                        }
                    }
                }
                self.say_read(&parts.join("\r\n"))?;
            }
            "sayrest" => {
                let text = self.rest().to_owned();
                self.pos = self.line_text().len();
                self.say_read(&text)?;
            }
            "saynchars" => {
                let n = parse_number(arg(0), "The count")?.max(0) as usize;
                let end = byte_of_char(self.rest(), n);
                let text = self.rest()[..end].to_owned();
                self.pos += end;
                self.say(&text)?;
            }
            "saynextnumber" => {
                let rest = self.rest();
                let start = rest.char_indices().find(|&(i, c)| {
                    c.is_ascii_digit()
                        || (c == '-' && rest[i + 1..].starts_with(|d: char| d.is_ascii_digit()))
                });
                if let Some((start, _)) = start {
                    if let Some((number, used)) = leading_number(&rest[start..]) {
                        self.pos += start + used;
                        self.say(&number)?;
                    }
                }
            }
            "saynextword" => {
                let rest = self.rest();
                let start = rest.len() - rest.trim_start().len();
                let length = rest[start..]
                    .find(char::is_whitespace)
                    .unwrap_or(rest.len() - start);
                let word = rest[start..start + length].to_owned();
                self.pos += start + length;
                self.say(&word)?;
            }
            "sayoutput" => {
                let value = self.output(arg(0)).unwrap_or_default();
                self.say(&value)?;
            }
            "saynewline" => self.say("\r\n")?,
            "saydate" => {
                let number = self.take_number()?;
                let seconds = if arg(0).eq_ignore_ascii_case("ms") {
                    number / 1000.
                } else {
                    number
                };
                self.say(&format_date(seconds.floor() as i64))?;
            }
            "sayduration" => {
                let number = self.take_number()?;
                let ms = match arg(0).to_ascii_lowercase().as_str() {
                    "ms" => number,
                    "m" => number * 60_000.,
                    _ => number * 1000.,
                };
                self.say(&format_duration(
                    ms.round() as i64,
                    flag(&args, 1),
                    flag(&args, 2),
                ))?;
            }
            "sayregexp" => {
                let expression = regex_compat::Regex::new(arg(0), true).map_err(|e| {
                    format!("the regular expression is not valid: {value}", value = e)
                })?;
                let rest = self.rest();
                let end = match args.get(2).filter(|end| !end.is_empty()) {
                    Some(end) => match rest.find(end.as_str()) {
                        Some(end) => end,
                        None => return Ok(next),
                    },
                    None => rest.len(),
                };
                // The cursor stays: scripts read several values from the same
                // stretch of a line, each SayRegexp bounded by the same end mark.
                let matches = expression.find_all(&rest[..end])?;
                let separator = args.get(1).map_or(" ", String::as_str);
                self.say(&matches.join(separator))?;
            }
            "sayformat" => {
                let current = self.input_text();
                let outputs = (!flag(&args, 1)).then_some(&self.outputs);
                let value = format_value(arg(0), &current, outputs)?;
                self.say(&value)?;
            }
            "sayvar" => {
                let value = self.env.vars.get(arg(0)).cloned().unwrap_or_default();
                self.say(&value)?;
            }
            "set" => {
                // Without a value the output is emptied but still exists, so
                // `%name%` in later parameters becomes nothing.
                let value = args.get(1).cloned().unwrap_or_default();
                self.size += value.len();
                self.outputs.insert(arg(0).to_uppercase(), value);
            }
            "skipchars" => {
                let skip = arg(0);
                let rest = self.rest();
                let used = rest.len() - rest.trim_start_matches(|c| skip.contains(c)).len();
                self.pos += used;
            }
            "insert" => {
                let (line, pos) = (self.line, self.pos);
                self.lines[line].insert_str(pos, arg(0));
            }
            "use" => {
                let text = arg(0).to_owned();
                self.set_input(&text);
            }
            "useformat" => {
                let current = self.input_text();
                let outputs = (!flag(&args, 1)).then_some(&self.outputs);
                let value = format_value(arg(0), &current, outputs)?;
                self.set_input(&value);
            }
            name if IFS.contains(&name) => {
                if !self.condition(command, &args)? {
                    next = command.target + 1;
                }
            }
            "else" => next = command.target + 1,
            "endif" => {}
            "outputto" => {
                self.current = match arg(0).to_uppercase() {
                    name if name.is_empty() => DEFAULT_OUTPUT.into(),
                    name => name,
                }
            }
            "do" => {
                self.repeats.insert(index, 0);
            }
            "while" => {
                let limit = args
                    .get(1)
                    .map(|n| parse_number(n, "The loop limit"))
                    .transpose()?;
                let start = command.target;
                let count = self.repeats.entry(start).or_default();
                *count += 1;
                let more = limit.is_none_or(|limit| (*count as i64) < limit);
                if more && self.condition(command, &args)? {
                    next = start + 1;
                }
            }
            // Replace and RegexpReplace change the current line, as KillTag
            // does: scripts strip the tags of one found line and then search
            // the following lines for markup.
            "replace" => {
                if !arg(0).is_empty() {
                    let text = self.line_text().replace(arg(0), arg(1));
                    self.replace_line(&text);
                }
            }
            "regexpreplace" => {
                let text = regex_compat::replace(self.line_text(), arg(0), arg(1), true)
                    .map_err(|e| format!("the regular expression failed: {value}", value = e))?;
                self.replace_line(&text);
            }
            "joinuntil" => {
                let needle = arg(0);
                let mut last = self.line;
                if !self.rest().contains(needle) {
                    last = (self.line + 1..self.lines.len())
                        .find(|&i| self.lines[i].contains(needle))
                        .unwrap_or(self.lines.len() - 1);
                }
                self.join(last)?;
            }
            "joinlines" => {
                let n = parse_number(arg(0), "The line count")?;
                let last = if n < 0 {
                    self.lines.len() - 1
                } else {
                    (self.line + n as usize).min(self.lines.len() - 1)
                };
                self.join(last)?;
            }
            "killtag" => {
                let replaced = kill_tag(self.line_text(), arg(0), arg(1));
                self.replace_line(&replaced);
            }
            "trim" => self.trim = !arg(0).eq_ignore_ascii_case("off"),
            "unspace" => {
                let line = self.line_text();
                let leading = line.len() - line.trim_start().len();
                let trimmed = line.trim().to_owned();
                let index = self.line;
                self.lines[index] = trimmed;
                self.pos = self.pos.saturating_sub(leading);
                self.clamp();
            }
            "debug" => {
                if arg(0).eq_ignore_ascii_case("on") {
                    let limit = args
                        .get(2)
                        .map(|n| parse_number(n, "The size limit"))
                        .transpose()?
                        .filter(|mb| *mb > 0)
                        .map_or(DEBUG_LIMIT, |mb| (mb as usize).saturating_mul(1024 * 1024));
                    // A forced trace (Options) keeps its own file.
                    if self.env.trace.is_none() {
                        self.finish_trace();
                        self.trace = Some(Trace {
                            name: Some(arg(1))
                                .filter(|n| !n.is_empty())
                                .unwrap_or("debug.out")
                                .to_owned(),
                            limit,
                            text: String::new(),
                        });
                    }
                } else if self.env.trace.is_none() {
                    self.finish_trace();
                }
            }
            "debugwriteinput" => self.files.push(DebugFile {
                name: Some(arg(0))
                    .filter(|n| !n.is_empty())
                    .unwrap_or("debug-input.out")
                    .to_owned(),
                content: self.input_text(),
            }),
            "json" => {
                if arg(0).eq_ignore_ascii_case("off") {
                    if let Some(json) = self.json.take() {
                        (self.lines, self.line, self.pos) = json.text;
                    }
                } else {
                    // "current" reads the transformed input from the cursor on,
                    // so a script can stop at the line holding a page's JSON.
                    // Text after the JSON value is ignored.
                    let text = if arg(1).eq_ignore_ascii_case("current") {
                        let mut text = self.rest().to_owned();
                        for line in &self.lines[self.line + 1..] {
                            text.push('\n');
                            text.push_str(line);
                        }
                        text
                    } else {
                        self.original.to_owned()
                    };
                    let root: Value =
                        serde_json::Deserializer::from_str(text.trim_start_matches('\u{feff}'))
                            .into_iter::<Value>()
                            .next()
                            .unwrap_or_else(|| serde_json::from_str(""))
                            .map_err(|e| {
                                format!("the input is not valid JSON: {value}", value = e)
                            })?;
                    let text = match self.json.take() {
                        Some(json) => json.text,
                        None => (self.lines.clone(), self.line, self.pos),
                    };
                    self.json = Some(Json {
                        root,
                        text,
                        stack: Vec::new(),
                        loops: Vec::new(),
                    });
                }
            }
            "json_foreach" | "json_foreach_reverse" => {
                // Objects are iterated member by member (their names are the
                // counter); anything else that is not an array is empty.
                let (mut items, mut keys): (Vec<Value>, Option<Vec<String>>) =
                    match json_lookup(self.json()?.current(), arg(0)).cloned() {
                        Some(Value::Array(items)) => (items, None),
                        Some(Value::Object(members)) => {
                            let (keys, items) = members.into_iter().unzip();
                            (items, Some(keys))
                        }
                        _ => (Vec::new(), None),
                    };
                if command.name == "json_foreach_reverse" {
                    items.reverse();
                    if let Some(keys) = keys.as_mut() {
                        keys.reverse();
                    }
                }
                self.set_input(&items.len().to_string());
                let json = self.json()?;
                if items.is_empty() {
                    next = command.target + 1;
                } else {
                    let depth = json.stack.len();
                    json.stack.push(items[0].clone());
                    json.loops.push(Each {
                        items,
                        keys,
                        index: 0,
                        depth,
                    });
                }
            }
            "json_foreach_end" => {
                let start = command.target;
                let json = self.json()?;
                let Some(each) = json.loops.last_mut() else {
                    return Err("json_foreach_end without a running json_foreach".into());
                };
                json.stack.truncate(each.depth);
                each.index += 1;
                if each.index < each.items.len() {
                    let item = each.items[each.index].clone();
                    json.stack.push(item);
                    next = start + 1;
                } else {
                    json.loops.pop();
                }
            }
            "json_foreach_counter" => {
                let width = args
                    .first()
                    .map(|n| parse_number(n, "The digit count"))
                    .transpose()?
                    .unwrap_or(0)
                    .max(0) as usize;
                let json = self.json()?;
                let Some(each) = json.loops.last() else {
                    return Err("json_foreach_counter outside json_foreach".into());
                };
                let counter = match &each.keys {
                    Some(keys) => keys[each.index].clone(),
                    None => format!("{:0width$}", each.index + 1),
                };
                // Unlike json_foreach it writes to the output: published
                // scripts follow it with Say "|" directly.
                self.say(&counter)?;
            }
            "json_select_object" => {
                // A missing object leaves the selection as it is (scripts
                // test the input and only then unselect).
                let key = arg(0).to_owned();
                let json = self.json()?;
                let found = json_lookup(json.current(), &key)
                    .filter(|v| v.is_object())
                    .cloned();
                let present = found.is_some();
                json.stack.extend(found);
                self.set_input(if present { &key } else { "" });
            }
            "json_unselect_object" => {
                // Leaving more objects than were selected stays where it is.
                let json = self.json()?;
                let floor = json.loops.last().map_or(0, |each| each.depth + 1);
                if json.stack.len() > floor {
                    json.stack.pop();
                }
            }
            "json_select" => {
                let json = self.json()?;
                let text = json_lookup(json.current(), arg(0))
                    .map(json_text)
                    .unwrap_or_default();
                self.set_input(&text);
            }
            "json_select_array" => {
                let n = parse_number(arg(1), "The element")?;
                let items = self.json_array(arg(0))?;
                let text = if n == -1 {
                    items.iter().map(json_text).collect::<Vec<_>>().join(arg(2))
                } else {
                    let item = usize::try_from(n - 1).ok().and_then(|i| items.get(i));
                    // An object element is selected like json_select_object, so
                    // json_select reads its members until json_unselect_object.
                    if let Some(object @ Value::Object(_)) = item {
                        self.json()?.stack.push(object.clone());
                    }
                    item.map(json_text).unwrap_or_default()
                };
                self.set_input(&text);
            }
            "json_select_many" => {
                let items = self.json_array(arg(0))?;
                let separator = args.get(2).map_or(", ", String::as_str);
                let last = args.get(3).map_or(separator, String::as_str);
                let max = args
                    .get(4)
                    .map(|n| parse_number(n, "The maximum"))
                    .transpose()?
                    .filter(|n| *n > 0);
                let empty = flag(&args, 5);
                let mut values: Vec<String> = items
                    .iter()
                    .map(|item| json_lookup(item, arg(1)).map(json_text).unwrap_or_default())
                    .filter(|value| empty || !value.is_empty())
                    .collect();
                if let Some(max) = max {
                    values.truncate(max as usize);
                }
                let text = match values.len() {
                    0 | 1 => values.concat(),
                    n => format!("{}{last}{}", values[..n - 1].join(separator), values[n - 1]),
                };
                self.set_input(&text);
            }
            "json_select_many_count" => {
                let items = self.json_array(arg(0))?;
                let empty = flag(&args, 2);
                let count = if args.len() > 1 && !arg(1).is_empty() {
                    items
                        .iter()
                        .filter(|item| match json_lookup(item, arg(1)) {
                            Some(value) => empty || !json_text(value).is_empty(),
                            None => false,
                        })
                        .count()
                } else {
                    items.len()
                };
                self.set_input(&count.to_string());
            }
            other => return Err(format!("{other} is not implemented")),
        }
        Ok(next)
    }

    /// Put `text` in place of the current line; line breaks in it start new
    /// lines after it. The cursor returns to the start of the line (scripts
    /// search the changed line again with FindInLine).
    fn replace_line(&mut self, text: &str) {
        let index = self.line;
        self.lines
            .splice(index..=index, split_lines(text))
            .for_each(drop);
        self.pos = 0;
    }

    /// Append the following lines up to `last` to the current line. The
    /// following lines stay where they are, so line counts in a script
    /// (`MoveLine -6` back to an attribute line) still hold after a join.
    fn join(&mut self, last: usize) -> Result<(), String> {
        if last <= self.line {
            return Ok(());
        }
        let appended = self.lines[self.line + 1..=last]
            .iter()
            .try_fold(0usize, |bytes, line| bytes.checked_add(line.len()));
        let growth = appended.and_then(|bytes| self.joined_bytes.checked_add(bytes));
        let Some((appended, growth)) = appended
            .zip(growth)
            .filter(|(_, growth)| *growth <= JOIN_GROWTH_LIMIT)
        else {
            return Err("joining lines would exceed the 16 MB append limit for one run".into());
        };
        // Preflight the entire append before allocating or touching any state.
        // Disjoint slices keep the retained lines unchanged without a concat
        // allocation or rebuilding the current prefix on every join.
        let index = self.line;
        let (before, following) = self.lines.split_at_mut(index + 1);
        let current = &mut before[index];
        current.reserve(appended);
        for line in following.iter().take(last - index) {
            current.push_str(line);
        }
        self.joined_bytes = growth;
        Ok(())
    }
}

fn excerpt(text: &str) -> String {
    let mut short: String = text.chars().take(80).collect();
    if short.len() < text.len() {
        short.push('…');
    }
    short
}
fn excerpt_tail(text: &str) -> String {
    let count = text.chars().count();
    if count <= 80 {
        return text.to_owned();
    }
    format!("…{}", text.chars().skip(count - 80).collect::<String>())
}

/// Run `script` over `input`.
pub fn run(script: &Script, input: &str, env: &Environment<'_>) -> Outcome {
    let mut machine = Machine::new(input, env);
    let mut index = 0;
    let mut steps = 0;
    let result = loop {
        let Some(command) = script.commands.get(index) else {
            break Ok(());
        };
        steps += 1;
        if steps > STEP_LIMIT {
            break Err(ScriptError {
                line: command.line,
                command: command.text.clone(),
                message: "the script ran too long; check Do/While loops".into(),
            });
        }
        match machine.step(script, index) {
            Ok(next) => {
                machine.trace_step(command);
                index = next;
            }
            Err(message) => {
                if let Some(trace) = machine.trace.as_mut() {
                    trace.text.push_str(&format!(
                        "{:>5}: {}\n       ERROR: {message}\n",
                        command.line, command.text
                    ));
                }
                break Err(ScriptError {
                    line: command.line,
                    command: command.text.clone(),
                    message,
                });
            }
        }
    };
    machine.finish_trace();
    Outcome {
        result: result.map(|()| machine.outputs),
        files: machine.files,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(text: &str) -> Script {
        let lines: Vec<(usize, String)> = text
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.to_owned()))
            .collect();
        Script::compile(&lines).unwrap()
    }
    fn outputs_with(text: &str, input: &str, env: &Environment<'_>) -> BTreeMap<String, String> {
        run(&script(text), input, env).result.unwrap()
    }
    fn outputs(text: &str, input: &str) -> BTreeMap<String, String> {
        outputs_with(text, input, &Environment::default())
    }
    fn out(text: &str, input: &str) -> String {
        outputs(text, input)
            .remove(DEFAULT_OUTPUT)
            .unwrap_or_default()
    }
    fn error(text: &str, input: &str) -> ScriptError {
        run(&script(text), input, &Environment::default())
            .result
            .unwrap_err()
    }

    #[test]
    fn boost_whole_subject_patterns_match_across_source_commands() {
        let pattern = r"(?i)\APaper\s+Boats\z";
        for (input, matched) in [
            ("Paper Boats", true),
            ("paper boats", true),
            ("Paper Boats Live", false),
        ] {
            assert_eq!(
                out(&format!(r#"SayRegexp "{pattern}""#), input),
                if matched { input } else { "" }
            );
            assert_eq!(
                out(
                    &format!("RegexpReplace \"{pattern}\" \"matched\"\nSayRest"),
                    input
                ),
                if matched { "matched" } else { input }
            );
        }
        assert!(error(r#"SayRegexp "(""#, "Paper Boats")
            .message
            .contains("not valid"));
        assert!(
            error("RegexpReplace \"(\" \"matched\"\nSayRest", "Paper Boats")
                .message
                .contains("failed")
        );
    }

    #[test]
    fn prefix_diagnostic_fixtures_execute_real_script_producers() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/web-scripting-diagnostics.json"
        ))
        .unwrap();
        let execute = |text: &str, page: &str| {
            let lines = text
                .lines()
                .enumerate()
                .map(|(i, line)| (i + 1, line.to_owned()))
                .collect::<Vec<_>>();
            Script::compile(&lines)
                .and_then(|script| run(&script, page, &Environment::default()).result)
        };
        let mut failures = Vec::new();
        for case in cases.as_array().unwrap() {
            let result = execute(
                case["script"].as_str().unwrap(),
                case["page"].as_str().unwrap(),
            );
            match (case["output"].as_str(), result) {
                (Some(expected), Ok(outputs)) => {
                    let actual = outputs
                        .get(DEFAULT_OUTPUT)
                        .map(String::as_str)
                        .unwrap_or_default();
                    if actual != expected {
                        failures.push(format!(
                            "{case}: expected output {expected:?}, got {actual:?}"
                        ));
                    }
                }
                (None, Err(error)) => {
                    if error.message != case["error"].as_str().unwrap() {
                        failures.push(format!("{case}: wrong diagnostic {error:?}"));
                    }
                }
                (Some(_), Err(error)) => {
                    failures.push(format!("{case}: unexpected diagnostic {error:?}"))
                }
                (None, Ok(outputs)) => {
                    failures.push(format!("{case}: expected diagnostic, got {outputs:?}"))
                }
            }
        }
        for (script, page, prefix) in [
            (
                "SayRegexp \"[\"",
                "Title",
                "the regular expression is not valid: ",
            ),
            (
                "RegexpReplace \"[\" \"Title\"",
                "Title",
                "the regular expression failed: ",
            ),
            ("json \"on\"", "Title", "the input is not valid JSON: "),
        ] {
            match execute(script, page) {
                Err(error) if error.message.starts_with(prefix) => {}
                result => failures.push(format!("{script}: expected {prefix:?}, got {result:?}")),
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn wrapped_compile_errors_keep_source_text_and_complete_message_frames() {
        for (text, message) in [
            ("Help", "Help is not a known command"),
            ("GotoChar", "GotoChar takes 1 parameter"),
            ("SayRest x", "SayRest takes 0 parameters"),
            ("FindLine", "FindLine takes 1 to 3 parameters"),
            ("Else", "Else without If"),
            ("If \"Help\"", "this block has no Endif"),
            ("If \"Alpha): Beta\"", "this block has no Endif"),
            (r#"If "Alpha\"): Beta""#, "this block has no Endif"),
            ("Help \"Alpha): Beta\"", "Help is not a known command"),
        ] {
            let error = Script::compile(&[(17, text.into())]).unwrap_err();
            assert_eq!(error.command, text);
            assert_eq!(error.message, message);
            assert_eq!(error.to_string(), format!("line 17 ({text}): {message}"));
        }
    }

    #[test]
    fn tokenizer_reads_quotes_escapes_words_and_comments() {
        assert_eq!(
            tokenize(r#"FindLine "a \"b\" c" 2 1 # note"#).unwrap(),
            (
                "FindLine".into(),
                vec![r#"a "b" c"#.into(), "2".into(), "1".into()]
            )
        );
        assert_eq!(tokenize(r##"Say "#""##).unwrap().1, ["#"]);
        assert_eq!(
            tokenize(r#"RegexpReplace "\s+" " ""#).unwrap().1,
            [r"\s+", " "]
        );
        assert!(tokenize(r#"Say "open"#).is_err());
    }

    #[test]
    fn compile_reports_unknown_commands_arity_and_unbalanced_blocks() {
        let compile = |text: &str| {
            Script::compile(
                &text
                    .lines()
                    .enumerate()
                    .map(|(i, l)| (i + 1, l.to_owned()))
                    .collect::<Vec<_>>(),
            )
            .unwrap_err()
        };
        let unknown = compile("# comment\n\nFrobnicate \"x\"");
        assert_eq!(
            (unknown.line, unknown.message.as_str()),
            (3, "Frobnicate is not a known command")
        );
        assert!(compile("SayRest \"x\"")
            .message
            .contains("takes 0 parameters"));
        assert!(compile("FindLine").message.contains("1 to 3"));
        assert!(compile("If \"a\"\nSay \"x\"").message.contains("no Endif"));
        assert!(compile("Else").message.contains("Else without If"));
        assert!(compile("Endif").message.contains("Endif without If"));
        assert!(compile("While \"x\"").message.contains("While without Do"));
        assert!(compile("Do\nSay \"x\"").message.contains("no While"));
        assert!(compile("json_foreach_end")
            .message
            .contains("without json_foreach"));
        assert!(compile("If \"a\"\nElse\nElse\nEndif")
            .message
            .contains("second Else"));
        // Names are case-insensitive.
        assert!(Script::compile(&[(1, "sAyReSt".into())]).is_ok());
        assert!(command_names().count() >= 58);
    }

    #[test]
    fn find_and_move_commands_position_the_cursor() {
        let page = "<html>\n<b>Year:</b> 1999\nnext\nYear: 2001 Year: 2002";
        assert_eq!(
            out(
                "FindLine \"Year:\"\nFindInLine \"Year:\"\nSayNextNumber",
                page
            ),
            "1999"
        );
        // Nth occurrence of a line and within a line.
        assert_eq!(
            out(
                "FindLine \"Year:\" 2\nFindInLine \"Year:\" 2\nSayRest",
                page
            ),
            "2002"
        );
        assert_eq!(out("FindLineNoCase \"YEAR:\" 2\nSayNChars 4", page), "Year");
        // A found line is not found again once the cursor passed the text.
        assert_eq!(
            out(
                "FindLine \"Year:\"\nFindInLine \"Year:\"\nFindLine \"Year:\"\nSayNChars 10",
                page
            ),
            "Year: 2001"
        );
        assert_eq!(out("GotoLine 3\nSayRest", page), "next");
        assert_eq!(out("GotoLine 2\nGotoChar 4\nSayNChars 5", page), "Year:");
        assert_eq!(
            out("GotoLine 2\nMoveChar 3\nMoveChar -1\nSayNChars 3", page),
            ">Ye"
        );
        assert_eq!(
            out("MoveLine 2\nSayRest\nMoveLine -1\nSayNChars 3", page),
            "next<b>"
        );
        // Errors unless the "no error" flag is set.
        assert_eq!(
            error("FindLine \"Nope\"", page).message,
            "\"Nope\" was not found"
        );
        // Without an error the cursor ends up past the text searched.
        assert_eq!(
            out("FindLine \"Nope\" 1 1\nIf \"\"\nSay \"end\"\nEndif\nGotoLine 1\nIfNot \"\"\nSayNChars 6\nEndif", page),
            "end<html>"
        );
        assert!(error("FindInLine \"Nope\"", page)
            .message
            .contains("current line"));
        assert_eq!(
            out("FindInLine \"Nope\" 1 1\nMoveChar -2\nSayNChars 2", page),
            "l>"
        );
        assert!(error("MoveLine 9", page)
            .message
            .contains("leaves the input"));
        assert_eq!(out("MoveLine 9 1\nSayRest", page), "Year: 2001 Year: 2002");
        assert!(error("GotoLine 99", page).message.contains("only 4 lines"));
        assert_eq!(error("GotoChar x", page).line, 1);
    }

    #[test]
    fn say_commands_read_from_the_cursor() {
        let input = "  Title One  <i>x</i>\nsecond line\nthird END rest";
        assert_eq!(out("SayUntil \"<\"", input), "Title One");
        assert_eq!(out("Trim \"off\"\nSayUntil \"<\"", input), "  Title One  ");
        assert_eq!(out("SayUntil \"missing\"", input), "Title One  <i>x</i>");
        assert_eq!(out("FindInLine \"<i>\"\nSayRest", input), "x</i>");
        assert_eq!(out("Say \"a\"\nSay \" b \"\nSayNewline", input), "a b \r\n");
        assert_eq!(
            out("MoveLine 1\nSayUntilML \"END\"", input),
            "second line\r\nthird"
        );
        assert_eq!(
            out("MoveLine 1\nSayUntilML \"none\"", input),
            "second line\r\nthird END rest"
        );
        assert_eq!(
            out("SayNextWord\nSay \"|\"\nSayNextWord", input),
            "Title|One"
        );
        assert_eq!(out("SayNextNumber", "Track -3.5 of 9"), "-3.5");
        assert_eq!(
            out("SayNextNumber\nSay \",\"\nSayNextNumber", "a1999b-c 2."),
            "1999,2"
        );
        assert_eq!(out("SayNextNumber", "no digits"), "");
        assert_eq!(out("SkipChars \" T\"\nSayNChars 3", input), "itl");
        assert_eq!(out("Insert \"[x]\"\nSayNChars 5", input), "[x]  ");
    }

    #[test]
    fn dates_durations_and_regular_expressions() {
        assert_eq!(out("SayDate", "946684800"), "2000-01-01");
        assert_eq!(out("SayDate \"ms\"", "951782400000"), "2000-02-29");
        assert_eq!(out("SayDate", "-86400"), "1969-12-31");
        assert_eq!(out("SayDuration", "245"), "4:05");
        assert_eq!(out("SayDuration \"ms\"", "245123"), "4:05");
        assert_eq!(out("SayDuration \"ms\" 1 1", "3725123"), "1:02:05.123");
        assert_eq!(out("SayDuration \"m\"", "61"), "61:00");
        assert!(error("SayDuration", "abc").message.contains("no number"));
        assert_eq!(out("SayRegexp \"\\d+\"", "a 12 b 345 c"), "12 345");
        assert_eq!(out("SayRegexp \"\\d+\" \"|\"", "a 12 b 345 c"), "12|345");
        assert_eq!(out("SayRegexp \"\\d+\" \", \" \"b\"", "a 12 b 345 c"), "12");
        assert_eq!(out("SayRegexp \"\\d+\" \", \" \"zzz\"", "a 12 b 345 c"), "");
        assert!(error("SayRegexp \"(\"", "x").message.contains("not valid"));
        // The cursor stays where it was.
        assert_eq!(
            out(
                "SayRegexp \"\\d+\" \",\" \"b\"\nSay \"|\"\nSayRest",
                "a 12 b 345"
            ),
            "12|a 12 b 345"
        );
    }

    #[test]
    fn outputs_set_and_conditions() {
        let o = outputs(
            "OutputTo \"Album\"\nSay \"A\"\nOutputTo \"ARTIST\"\nSay \"B\"\nSayOutput \"album\"\nSet \"YEAR\" \"1999\"\nSet \"album\"\nOutputTo \"x\"\nSayOutput \"CurrentUrl\"",
            "",
        );
        assert_eq!(o.get("ARTIST").unwrap(), "BA");
        assert_eq!(o.get("YEAR").unwrap(), "1999");
        assert_eq!(o["ALBUM"], "");
        assert_eq!(o.get("X").unwrap(), "");
        let input = "abc 42";
        assert_eq!(
            out("If \"ab\"\nSay \"yes\"\nElse\nSay \"no\"\nEndif", input),
            "yes"
        );
        assert_eq!(
            out("If \"b\"\nSay \"yes\"\nElse\nSay \"no\"\nEndif", input),
            "no"
        );
        assert_eq!(
            out("IfNot \"b\"\nSay \"yes\"\nEndif\nSay \".\"", input),
            "yes."
        );
        assert_eq!(
            out(
                "MoveChar 4\nIfGreater 41\nSay \"g\"\nEndif\nIfLess 42\nSay \"l\"\nEndif",
                input
            ),
            "g"
        );
        assert_eq!(
            out(
                "IfGreater 1\nSay \"g\"\nElse\nSay \"no number\"\nEndif",
                input
            ),
            "no number"
        );
        assert_eq!(
            out("OutputTo \"T\"\nSay \"x\"\nOutputTo \"\"\nIfOutput \"T\"\nSay \"has\"\nEndif\nIfNotOutput \"U\"\nSay \" none\"\nEndif", ""),
            "has none"
        );
        // Nested blocks skip correctly.
        assert_eq!(
            out("If \"z\"\nIf \"a\"\nSay \"1\"\nElse\nSay \"2\"\nEndif\nElse\nIf \"a\"\nSay \"3\"\nEndif\nEndif", input),
            "3"
        );
    }

    #[test]
    fn configuration_variables() {
        let env = Environment {
            vars: [
                ("lang".to_string(), "de".to_string()),
                ("full".to_string(), "true".to_string()),
            ]
            .into(),
            ..Environment::default()
        };
        let o = outputs_with(
            "SayVar \"lang\"\nIfVar \"full\" \"TRUE\"\nSay \"+\"\nEndif\nIfNotVar \"lang\" \"en\"\nSay \"!\"\nEndif\nIfVar \"missing\" \"x\"\nSay \"?\"\nEndif",
            "",
            &env,
        );
        assert_eq!(o[DEFAULT_OUTPUT], "de+!");
    }

    #[test]
    fn do_while_loops_with_limits() {
        let input = "<li>One</li>\n<li>Two</li>\n<li>Three</li>\n</ul>";
        let text = "OutputTo \"TITLE\"\nDo\nFindInLine \"<li>\"\nSayUntil \"<\"\nSay \"|\"\nMoveLine 1\nWhile \"<li>\"";
        assert_eq!(outputs(text, input)["TITLE"], "One|Two|Three|");
        let limited = "OutputTo \"TITLE\"\nDo\nFindInLine \"<li>\"\nSayUntil \"<\"\nSay \"|\"\nMoveLine 1\nWhile \"<li>\" 2";
        assert_eq!(outputs(limited, input)["TITLE"], "One|Two|");
        let endless = error("Do\nSay \"\"\nWhile \"x\"", "x");
        assert!(endless.message.contains("ran too long"));
    }

    #[test]
    fn input_transformations() {
        assert_eq!(
            out("Replace \"<br>\" \"|\"\nSayRest", "a<br>b<br>c"),
            "a|b|c"
        );
        assert_eq!(
            out(
                "RegexpReplace \"(\\d+)-(\\d+)\" \"$2/$1\"\nSayRest",
                "x 12-34 y"
            ),
            "x 34/12 y"
        );
        // Both work on the current line only and return to its start.
        assert_eq!(
            out(
                "FindInLine \"b\"\nReplace \"a\" \"A\"\nFindInLine \"b\"\nSayRest",
                "abcb"
            ),
            "cb"
        );
        assert_eq!(
            out(
                "RegexpReplace \"<[^>]+>\" \"\"\nSayRest\nMoveLine 1\nReplace \"x\" \"y\"\nSay \"|\"\nSayRest\nMoveLine 1\nSay \"|\"\nSayRest",
                "<b>a</b>\n<i>x</i>\n<u>x</u>"
            ),
            "a|<i>y</i>|<u>x</u>"
        );
        assert_eq!(
            out(
                "RegexpReplace \"<br>\" \"\\n\"\nSayRest\nMoveLine 1\nSay \"|\"\nSayRest",
                "a<br>b\nc"
            ),
            "a|b"
        );
        assert_eq!(
            out("JoinUntil \"</p>\"\nSayRest", "<p>a\n b\n c</p>\nafter"),
            "<p>a b c</p>"
        );
        assert_eq!(out("JoinUntil \"never\"\nSayRest", "a\nb"), "ab");
        assert_eq!(out("JoinLines 1\nSayRest", "a\nb\nc"), "ab");
        // The joined lines stay in place after the joined one.
        assert_eq!(
            out(
                "JoinLines 1\nMoveLine 1\nSayRest\nGotoLine 3\nSayRest",
                "a\nb\nc"
            ),
            "bc"
        );
        assert_eq!(out("JoinLines -1\nSayRest", "a\nb\nc"), "abc");
        assert_eq!(out("JoinLines 9\nSayRest", "a\nb"), "ab");
        assert_eq!(
            out(
                "KillTag \"b\"\nSayRest",
                "<b class=\"x\">bold</b> <i>it</i>"
            ),
            "bold <i>it</i>"
        );
        assert_eq!(
            out(
                "Trim \"off\"\nKillTag \"*\" \" \"\nSayRest",
                "<b>bold</b><br/>x"
            ),
            " bold  x"
        );
        assert_eq!(
            out("KillTag \"a\"\nSayRest", "a < b <abbr>t</abbr>"),
            "a < b <abbr>t</abbr>"
        );
        assert_eq!(
            out("Trim \"off\"\nUnspace\nSayRest\nSay \"|\"", "   padded   "),
            "padded|"
        );
        assert_eq!(
            out("Use \"new input\"\nFindInLine \"new \"\nSayRest", "old"),
            "input"
        );
        assert_eq!(
            out(
                "OutputTo \"A\"\nSay \"x\"\nOutputTo \"\"\nUseFormat \"%a%-%_current%\"\nSayRest",
                "in"
            ),
            "x-in"
        );
        assert_eq!(
            out("OutputTo \"A\"\nSay \"x\"\nOutputTo \"\"\nUseFormat \"<%a%>%_current%\" 1\nSayRest", "in"),
            "<>in"
        );
    }

    #[test]
    fn formats_and_output_references() {
        let o = outputs(
            "OutputTo \"ALBUM\"\nSay \"Discovery\"\nOutputTo \"OUT\"\nSayFormat \"$upper(%album%) %_current%\"\nOutputTo \"BAR\"\nSayFormat \"a$verticalBar()b\"\nOutputTo \"REF\"\nFindLine \"%album%\"\nSayRest\nSay \" 100% %nothing% %20\"",
            "x\nDiscovery!",
        );
        assert_eq!(o["OUT"], "DISCOVERY x\nDiscovery!");
        assert_eq!(o["BAR"], format!("a{VERTICAL_BAR}b"));
        assert_eq!(o["REF"], "Discovery! 100% %nothing% %20");
        assert!(error("SayFormat \"$nope(\"", "").message.len() > 3);
    }

    #[test]
    fn json_selection_and_iteration() {
        let doc = r#"{"title":"Album","year":1999,"live":false,"nothing":null,
            "credits":{"producer":"P","engineer":"E"},
            "artist":{"name":"Band","id":7},
            "genres":["Rock","Pop"],
            "tracks":[{"title":"One","length":61000,"artists":[{"name":"A"},{"name":"B"}]},
                      {"title":"Two","length":125000},
                      {"title":"","length":5}]}"#;
        let script_text = r#"json "on"
OutputTo "ALBUM"
json_select "title"
SayRest
OutputTo "YEAR"
json_select "year"
SayRest
OutputTo "LIVE"
json_select "live"
SayRest
OutputTo "NONE"
json_select "nothing"
SayRest
json_select "missing"
SayRest
OutputTo "ARTIST"
json_select_object "artist"
If "artist"
json_select "name"
SayRest
Endif
json_unselect_object
OutputTo "PATH"
json_select "artist.name"
SayRest
OutputTo "GENRE"
json_select_array "genres" 1
SayRest
OutputTo "GENRES"
json_select_array "genres" -1 "; "
SayRest
OutputTo "COUNT"
json_foreach "tracks"
SayRest
OutputTo "TITLE"
json_select "title"
SayRest
Say "|"
OutputTo "_LENGTH"
json_select "length"
SayDuration "ms"
Say "|"
OutputTo "TRACK"
json_foreach_counter 2
Say "|"
OutputTo "COUNT"
json_foreach_end
OutputTo "REVERSED"
json_foreach_reverse "tracks"
json_select "title"
SayRest
Say ","
json_foreach_end
OutputTo "MANY"
json_select_many "tracks" "title" ", " " & "
SayRest
OutputTo "MANYEMPTY"
json_select_many "tracks" "title" "/" "/" 0 1
SayRest
OutputTo "MAX"
json_select_many "tracks" "title" "+" "+" 1
SayRest
OutputTo "NESTED"
json_foreach "tracks"
json_select_many "artists" "name" "+"
SayRest
Say ";"
json_foreach_end
OutputTo "N"
json_select_many_count "tracks"
SayRest
json_select_many_count "tracks" "title"
SayRest
json_select_many_count "tracks" "title" 1
SayRest
json_select_many_count "tracks" "artists"
SayRest
OutputTo "ROLES"
json_foreach "credits"
json_foreach_counter
Say "="
json_select ""
SayRest
Say ";"
json_foreach_end
OutputTo "EMPTY"
json_foreach "missing"
Say "never"
json_foreach_end
Say "after"
"#;
        let o = outputs(script_text, doc);
        assert_eq!(o["ALBUM"], "Album");
        assert_eq!(o["YEAR"], "1999");
        assert_eq!(o["LIVE"], "false");
        assert_eq!(o["NONE"], "");
        assert_eq!(o["ARTIST"], "Band");
        assert_eq!(o["PATH"], "Band");
        assert_eq!(o["GENRE"], "Rock");
        assert_eq!(o["GENRES"], "Rock; Pop");
        assert_eq!(o["COUNT"], "3");
        assert_eq!(o["TITLE"], "One|Two||");
        assert_eq!(o["_LENGTH"], "1:01|2:05|0:00|");
        assert_eq!(o["TRACK"], "01|02|03|");
        assert_eq!(o["REVERSED"], ",Two,One,");
        assert_eq!(o["MANY"], "One & Two");
        assert_eq!(o["MANYEMPTY"], "One/Two/");
        assert_eq!(o["MAX"], "One");
        assert_eq!(o["NESTED"], "A+B;;;");
        assert_eq!(o["N"], "3231");
        assert_eq!(o["EMPTY"], "after");
        // Objects are iterated in document order; the counter is the name.
        assert_eq!(o["ROLES"], "producer=P;engineer=E;");
    }

    #[test]
    fn json_mode_errors_and_current_input() {
        assert!(error("json_select \"a\"", "{}")
            .message
            .contains("JSON mode is off"));
        assert!(error("json \"on\"", "not json")
            .message
            .contains("not valid JSON"));
        // A failed selection is not entered, and unselecting at the top stays.
        assert_eq!(
            out(
                "json \"on\"\njson_select_object \"none\"\njson_select \"x\"\nSayRest\njson_unselect_object\njson_unselect_object\njson_select \"x\"\nSayRest",
                "{\"x\":1}"
            ),
            "11"
        );
        // "current" parses the transformed input from the cursor instead of
        // the download; text after the JSON value is ignored.
        assert_eq!(
            out(
                "FindLine \"<script\"\nMoveLine 1\njson \"on\" \"current\"\njson_select \"a\"\nSayRest\njson \"off\"\nMoveLine 1\nSayRest",
                "<html>\n<script>\n{\"a\":\"page\"}\n</script></html>"
            ),
            // `json "off"` returns to the page where JSON mode started.
            "page</script></html>"
        );
        assert_eq!(
            out(
                "Replace \"'\" \"\\\"\"\njson \"on\" \"current\"\njson_select \"a\"\nSayRest",
                "{'a':'b'}"
            ),
            "b"
        );
        assert_eq!(
            out("json \"on\"\njson_select_object \"missing\"\nIfNot \"missing\"\nSay \"absent\"\nEndif\njson_unselect_object\njson \"off\"\nSayRest", "{\"x\":1}"),
            "absent{\"x\":1}"
        );
    }

    #[test]
    fn debug_traces_and_input_dumps() {
        let outcome = run(
            &script("DebugWriteInput \"in.out\"\nDebug \"on\" \"trace.out\"\nOutputTo \"A\"\nSayRest\nFindLine \"zz\""),
            "hello",
            &Environment::default(),
        );
        assert_eq!(outcome.result.unwrap_err().line, 5);
        assert_eq!(
            outcome.files[0],
            DebugFile {
                name: "in.out".into(),
                content: "hello".into()
            }
        );
        let trace = &outcome.files[1];
        assert_eq!(trace.name, "trace.out");
        assert!(trace.content.contains("4: SayRest"), "{}", trace.content);
        assert!(trace.content.contains("output \"A\": hello"));
        assert!(trace.content.contains("ERROR: \"zz\" was not found"));
        // Debug "off" closes the file; Options force one trace for the whole run.
        let forced = Environment {
            trace: Some("forced.out".into()),
            ..Environment::default()
        };
        let outcome = run(
            &script("Debug \"on\" \"own.out\"\nSay \"x\"\nDebug \"off\""),
            "",
            &forced,
        );
        assert_eq!(outcome.files.len(), 1);
        assert_eq!(outcome.files[0].name, "forced.out");
        assert!(outcome.files[0].content.contains("Say \"x\""));
        let limited = run(
            &script("Debug \"on\" \"a.out\"\nDebug \"off\"\nSay \"x\""),
            "",
            &Environment::default(),
        );
        assert_eq!(limited.files.len(), 1);
        assert!(!limited.files[0].content.contains("Say"));
    }

    fn one_step(machine: &mut Machine<'_>, text: &str) -> Result<usize, String> {
        let command = Script::compile(&[(1, text.to_owned())]).unwrap();
        machine.step(&command, 0)
    }

    fn repeated_join_error(command: &str) -> ScriptError {
        // At most 17 MiB even before the guard exists; never reproduce the
        // unbounded two-million-step allocation pattern in a test.
        let input = format!("A\n{}", "é".repeat(512 * 1024));
        error(&format!("Do\n{command}\nWhile \"A\" 17"), &input)
    }

    #[test]
    fn join_lines_repeated_growth_stops_before_the_step_guard() {
        let error = repeated_join_error("JoinLines 1");
        assert_eq!(error.line, 2);
        assert_eq!(error.command, "JoinLines 1");
        assert!(error.message.contains("16 MB append limit"));
    }

    #[test]
    fn join_until_repeated_growth_stops_before_the_step_guard() {
        let error = repeated_join_error("JoinUntil \"missing\"");
        assert_eq!(error.line, 2);
        assert_eq!(error.command, "JoinUntil \"missing\"");
        assert!(error.message.contains("16 MB append limit"));
    }

    #[test]
    fn join_exact_byte_limit_keeps_cursor_and_retained_lines_on_refusal() {
        let env = Environment::default();
        let payload = "é".repeat(512 * 1024); // 1 MiB in UTF-8, not 1 MiB characters.
        let input = format!("A\n{payload}");
        let mut machine = Machine::new(&input, &env);
        one_step(&mut machine, "GotoChar 2").unwrap();
        for _ in 0..16 {
            one_step(&mut machine, "JoinLines 1").unwrap();
        }
        assert_eq!(machine.lines[0].len(), 1 + 16 * 1024 * 1024);
        let before = (machine.lines.clone(), machine.line, machine.pos);
        for command in ["JoinLines 1", "JoinUntil \"missing\""] {
            assert!(one_step(&mut machine, command).is_err(), "{command}");
            assert_eq!(
                (&machine.lines, machine.line, machine.pos),
                (&before.0, before.1, before.2)
            );
        }
        one_step(&mut machine, "MoveLine 1").unwrap();
        one_step(&mut machine, "GotoChar 2").unwrap();
        one_step(&mut machine, "SayRest").unwrap();
        assert_eq!(machine.outputs[DEFAULT_OUTPUT], &payload[2..]);
        assert_eq!(machine.lines.len(), 2);
    }

    #[test]
    fn overlapping_joins_keep_each_following_line_at_its_position() {
        let env = Environment::default();
        let mut machine = Machine::new("A\nBB\nZ", &env);
        one_step(&mut machine, "GotoChar 2").unwrap();
        one_step(&mut machine, "JoinUntil \"Z\"").unwrap();
        assert_eq!((machine.line, machine.pos), (0, 1));
        one_step(&mut machine, "MoveLine 1").unwrap();
        one_step(&mut machine, "JoinLines 1").unwrap();
        one_step(&mut machine, "MoveLine -1").unwrap();
        one_step(&mut machine, "JoinLines 1").unwrap();
        assert_eq!(machine.lines, ["ABBZBBZ", "BBZ", "Z"]);
        assert_eq!((machine.line, machine.pos), (0, 0));
        one_step(&mut machine, "MoveLine 2").unwrap();
        one_step(&mut machine, "SayRest").unwrap();
        assert_eq!(machine.outputs[DEFAULT_OUTPUT], "Z");
    }

    #[test]
    fn join_budget_survives_use_replacement_and_json_input_switches() {
        let env = Environment::default();
        let input = format!("A\n{}", "x".repeat(1024 * 1024));
        let mut machine = Machine::new(&input, &env);
        for _ in 0..16 {
            one_step(&mut machine, "JoinLines 1").unwrap();
        }
        for command in [
            "Set \"fresh\" \"A\"",
            "OutputTo \"fresh\"",
            "SayNewline",
            "Say \"B\"",
            "Use \"%fresh%\"",
        ] {
            one_step(&mut machine, command).unwrap();
        }
        assert_eq!(machine.lines, ["A", "B"]);
        assert!(one_step(&mut machine, "JoinLines 1").is_err());
        one_step(&mut machine, "Replace \"A\" \"C\"").unwrap();
        assert!(one_step(&mut machine, "JoinUntil \"missing\"").is_err());
        one_step(&mut machine, "UseFormat \"%_current%\"").unwrap();
        assert!(one_step(&mut machine, "JoinLines 1").is_err());
        machine.set_input(r#"{"x":"A\nB"}"#);
        one_step(&mut machine, "json \"on\" \"current\"").unwrap();
        one_step(&mut machine, "json_select \"x\"").unwrap();
        assert_eq!(machine.lines, ["A", "B"]);
        assert!(one_step(&mut machine, "JoinLines 1").is_err());
        one_step(&mut machine, "json \"off\"").unwrap();
        one_step(&mut machine, "Use \"%fresh%\"").unwrap();
        assert!(one_step(&mut machine, "JoinLines 1").is_err());
    }

    #[test]
    fn a_refused_join_does_not_spend_the_remaining_byte_allowance() {
        let env = Environment::default();
        let input = format!("A\nQ{}", "x".repeat(1024 * 1024 - 1));
        let mut machine = Machine::new(&input, &env);
        for _ in 0..15 {
            one_step(&mut machine, "JoinLines 1").unwrap();
        }
        one_step(&mut machine, "MoveLine 1").unwrap();
        one_step(&mut machine, "Replace \"Q\" \"\"").unwrap();
        one_step(&mut machine, "MoveLine -1").unwrap();
        one_step(&mut machine, "JoinLines 1").unwrap(); // One byte still available.
        assert!(one_step(&mut machine, "JoinLines 1").is_err());
        machine.set_input("A\nB");
        one_step(&mut machine, "JoinUntil \"missing\"").unwrap();
        assert_eq!(machine.lines, ["AB", "B"]);
        assert!(one_step(&mut machine, "JoinLines 1").is_err());
    }

    #[test]
    fn helpers() {
        assert_eq!(format_date(0), "1970-01-01");
        assert_eq!(format_date(1_700_000_000), "2023-11-14");
        assert_eq!(format_duration(-61_000, false, false), "-1:01");
        assert_eq!(leading_number("  12.5x"), Some(("12.5".into(), 6)));
        assert_eq!(leading_number("x"), None);
        assert_eq!(excerpt(&"a".repeat(100)).chars().count(), 81);
    }
}
