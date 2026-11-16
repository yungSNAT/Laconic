// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

/// Drop a leading YAML frontmatter block, as hooks/always-on.sh does, so the eval grades the
/// text the hook injects. A block that opens but never closes is left alone, not emptied.
pub fn strip_frontmatter(text: &str) -> String {
    let lines = py::split_lines(text);
    if lines.first().is_none_or(|first| py::strip(first) != "---") {
        return text.to_owned();
    }
    for index in 1..lines.len() {
        if py::strip(lines[index]) == "---" {
            return lines[index + 1..]
                .join("\n")
                .trim_start_matches('\n')
                .to_owned();
        }
    }
    text.to_owned()
}

/// The prompt a condition sends. Without injection the runner itself supplies the setup
/// (plugins, rules files), so whole installations can be compared under their condition names.
pub fn condition_prompt(
    task: &str,
    condition: &str,
    skill: Option<&Path>,
    inject: bool,
) -> Result<String, String> {
    if condition == "baseline" || !inject {
        return Ok(task.to_owned());
    }
    let skill = skill
        .ok_or_else(|| format!("--condition-skill is required for the {condition} condition"))?;
    let instructions = strip_frontmatter(&read_text(skill)?);
    Ok(format!(
        "Follow the response-style skill below while completing the task. \
         Do not discuss or quote the skill.\n\n\
         <response_style>\n{instructions}\n</response_style>\n\n<task>\n{task}\n</task>"
    ))
}

pub(super) fn no_get(value: &Value) -> Fault {
    Fault::Other(format!(
        "'{}' object has no attribute 'get'",
        type_name(value)
    ))
}

/// A runner's reply: its text, token usage and reported cost (None when not reported).
pub fn parse_response(output: &str, format: &Value) -> Result<(String, Value, Value), Fault> {
    match format.as_str() {
        Some("text") => Ok((py::strip(output).to_owned(), json!({}), Value::Null)),
        Some("claude-json") => {
            let body = output.trim_start_matches(py::is_space);
            let payload = serde_json::Deserializer::from_str(body)
                .into_iter::<Value>()
                .next()
                .unwrap_or_else(|| serde_json::from_str("")) // the end-of-input error
                .map_err(|e| Fault::Value(e.to_string()))?;
            let Value::Object(map) = &payload else {
                return Err(no_get(&payload));
            };
            let result = map.get("result").map_or_else(String::new, py::to_str);
            let usage = map
                .get("usage")
                .filter(|u| py::truthy(u))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let cost = map.get("total_cost_usd").cloned().unwrap_or(Value::Null);
            Ok((py::strip(&result).to_owned(), usage, cost))
        }
        Some("codex-jsonl") => {
            let mut events = Vec::new();
            for line in py::split_lines(output) {
                if !py::strip(line).is_empty() {
                    events.push(
                        serde_json::from_str::<Value>(line)
                            .map_err(|e| Fault::Value(e.to_string()))?,
                    );
                }
            }
            let mut text = Value::String(String::new());
            let mut usage = json!({});
            for event in &events {
                let Value::Object(map) = event else {
                    return Err(no_get(event));
                };
                let empty = json!({});
                let item = map.get("item").unwrap_or(&empty);
                let kind = map.get("type").and_then(Value::as_str);
                if kind == Some("item.completed") {
                    let Value::Object(fields) = item else {
                        return Err(no_get(item));
                    };
                    if fields.get("type").and_then(Value::as_str) == Some("agent_message") {
                        text = fields.get("text").cloned().unwrap_or(text);
                    }
                }
                if kind == Some("turn.completed") {
                    usage = map.get("usage").cloned().unwrap_or(usage);
                }
            }
            Ok((py::strip(&py::to_str(&text)).to_owned(), usage, Value::Null))
        }
        _ => Err(Fault::Value(format!(
            "Unsupported response format: {}",
            py::to_str(format)
        ))),
    }
}

/// The model a runner command asks for, only for a directly invoked CLI recognised here: a
/// wrapper's `-m` may select a Python module, not a model.
pub fn runner_model(command: &[String]) -> Option<String> {
    let executable = Path::new(command.first()?)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())?;
    if executable != "claude" && executable != "codex" {
        return None;
    }
    let mut model = None;
    let mut arguments = command[1..].iter();
    while let Some(arg) = arguments.next() {
        if arg == "--" {
            break;
        }
        if arg == "--model" || (executable == "codex" && arg == "-m") {
            model = arguments.next().cloned();
        } else if let Some(value) = arg.strip_prefix("--model=") {
            model = Some(value.to_owned());
        }
    }
    model.filter(|m| !m.is_empty() && !m.starts_with('-'))
}

/// `shlex.quote`.
pub fn shell_quote(word: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c);
    if word.is_empty() {
        "''".into()
    } else if word.chars().all(safe) {
        word.into()
    } else {
        format!("'{}'", word.replace('\'', "'\"'\"'"))
    }
}

/// `format(x, "g")`: 6 significant digits, trailing zeros dropped, exponent form below 1e-4
/// or from 1e6.
pub fn format_g(x: f64) -> String {
    if !x.is_finite() {
        return if x.is_nan() {
            "nan".into()
        } else if x > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    if x == 0.0 {
        return if x.is_sign_negative() {
            "-0".into()
        } else {
            "0".into()
        };
    }
    let scientific = format!("{x:.5e}");
    let (mantissa, exponent) = scientific.split_once('e').expect("an exponent");
    let exponent: i32 = exponent.parse().expect("an exponent");
    let trim = |digits: &str| -> String {
        if digits.contains('.') {
            digits
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_owned()
        } else {
            digits.to_owned()
        }
    };
    if (-4..6).contains(&exponent) {
        let places = usize::try_from(5 - exponent).expect("0 to 9 places");
        trim(&format!("{x:.places$}"))
    } else {
        let sign = if exponent < 0 { '-' } else { '+' };
        format!("{}e{sign}{:02}", trim(mantissa), exponent.abs())
    }
}

/// One model call, with a hang turned into an ordinary failed attempt (exit status 124): a CLI
/// that silently waits on provider retries would otherwise stall the whole condition. Output
/// that is not UTF-8 is a `Fault::Value`, as Python's UnicodeDecodeError was a ValueError; a
/// program that cannot start is a `Fault::Other`.
pub fn run_with_timeout(
    exec: Exec,
    command: &[String],
    input: &Input,
    cwd: &Path,
    timeout: f64,
) -> Result<(i32, String, String), Fault> {
    let limit = Duration::from_secs_f64(timeout);
    match exec(command, input, Some(cwd), Some(limit)) {
        Ok(done) => {
            let decode = |bytes: &[u8]| proc::text(bytes).map_err(Fault::Value);
            Ok((done.code, decode(&done.stdout)?, decode(&done.stderr)?))
        }
        Err(Failure::Timeout) => Ok((
            124,
            String::new(),
            format!("Timed out after {} seconds", format_g(timeout)),
        )),
        Err(spawn) => Err(Fault::Other(spawn.message(&command[0]))),
    }
}

/// An empty scratch directory for one runner call. Agent CLIs adopt their working directory as
/// project context: run one inside this checkout and it answers by inspecting the harness
/// instead of the task, and not equally across conditions.
pub fn neutral_cwd(prefix: &str) -> Result<tempfile::TempDir, String> {
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .map_err(|e| e.to_string())
}

/// The pause before retry `attempt + 1`: Python's `min(2**attempt, 5)` seconds.
pub fn backoff(attempt: i64) -> Duration {
    Duration::from_secs(1 << attempt.clamp(0, 3)).min(Duration::from_secs(5))
}
