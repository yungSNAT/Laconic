// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The rewrite run's steps: its pairs, its rows and its table.

use super::*;
use crate::py::Num;
use std::fmt::Write as _;

/// The rewriters asked through Claude Code, then patina, in the run's order.
pub(super) const MODEL_REWRITERS: [&str; 5] = ["laconic", "humanizer", "avoid", "work", "plain"];

/// The sample id at the end of a rewrite case id, `aihb-2026-10-<id>`.
fn sample_id(row: &Value) -> Result<String, String> {
    let id = py::text(row, "case_id")?;
    id.rsplit_once('-')
        .map(|(_, sid)| sid.to_owned())
        .ok_or_else(|| format!("case id {} has no '-'", py::str_repr(id)))
}

/// JSON Lines text, one row a line, as `json.dumps(row) + "\n"` writes it.
fn jsonl(rows: &[Value]) -> String {
    rows.iter()
        .map(|r| py::dumps(r, None, true) + "\n")
        .collect()
}

/// Writes `pairs-<rewriter>.jsonl` ({id, source, rewrite}) for `factalign.py` from
/// `run-<rewriter>.jsonl` and, for patina, `patina-out/<id>.json`, whose `verification` goes to
/// `patina-verify.jsonl`; and `preserve/<rewriter>-<id>/original` and `edit` for `run.sh` to
/// check with `laconic-check preserve`.
pub fn rewrite_pairs(work: &Path) -> Result<(), String> {
    let mut sources = Map::new();
    for r in read_jsonl(&work.join("aihb-2026-10.jsonl"))? {
        sources.insert(
            py::text(&r, "sample_id")?.to_owned(),
            py::field(&r, "input")?.clone(),
        );
    }
    let source = |sid: &str| {
        sources
            .get(sid)
            .cloned()
            .ok_or_else(|| format!("no source text {}", py::str_repr(sid)))
    };
    let mut pairs: Vec<(&str, Vec<Value>)> = Vec::new();
    for s in MODEL_REWRITERS {
        let mut rows = Vec::new();
        for r in read_jsonl(&work.join(format!("run-{s}.jsonl")))? {
            let sid = sample_id(&r)?;
            rows.push(
                json!({"id": sid, "source": source(&sid)?, "rewrite": py::field(&r, "response")?}),
            );
        }
        pairs.push((s, rows));
    }
    let mut patina = Vec::new();
    let mut verify = Vec::new();
    for (sid, text) in &sources {
        let d = cases::parse(&read_text(&work.join(format!("patina-out/{sid}.json")))?)?;
        if !d.is_object() {
            return Err(format!("patina-out/{sid}.json is not a JSON object"));
        }
        let output = py::get(&d, "output").filter(|o| py::truthy(o));
        patina.push(
            json!({"id": sid, "source": text, "rewrite": output.cloned().unwrap_or(json!(""))}),
        );
        let mut row = Map::new();
        row.insert("id".into(), json!(sid));
        match py::get(&d, "verification").filter(|v| py::truthy(v)) {
            None => {}
            Some(Value::Object(fields)) => row.extend(fields.clone()),
            Some(other) => {
                return Err(format!(
                    "patina-out/{sid}.json: verification is {}",
                    py::repr(other)
                ));
            }
        }
        verify.push(Value::Object(row));
    }
    pairs.push(("patina", patina));
    write(&work.join("patina-verify.jsonl"), &jsonl(&verify))?;
    for (s, rows) in &pairs {
        write(&work.join(format!("pairs-{s}.jsonl")), &jsonl(rows))?;
        for row in rows {
            let id = py::text(row, "id")?;
            let folder = work.join(format!("preserve/{s}-{id}"));
            fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
            write(&folder.join("original"), py::text(row, "source")?)?;
            write(&folder.join("edit"), py::text(row, "rewrite")?)?;
        }
    }
    Ok(())
}

/// Rows of `path` by `id`, the later of 2 with one id winning in the earlier one's place.
pub(super) fn by_id(
    rows: Vec<Value>,
    id: fn(&Value) -> Result<String, String>,
) -> Result<Map<String, Value>, String> {
    let mut map = Map::new();
    for row in rows {
        map.insert(id(&row)?, row);
    }
    Ok(map)
}

pub(super) fn id_field(row: &Value) -> Result<String, String> {
    Ok(py::text(row, "id")?.to_owned())
}

/// One row per rewrite, without the texts: `factalign.py`'s units, `preserve`'s verdict,
/// lengths, cost and, for patina, its own `--verify` result.
pub fn rewrite_results(work: &Path) -> Result<String, String> {
    let rows = |name: String| rows_if_any(&work.join(name));
    let verify = by_id(rows("patina-verify.jsonl".into())?, id_field)?;
    let mut out = String::new();
    for s in MODEL_REWRITERS.iter().chain(&["patina"]) {
        let pairs = by_id(rows(format!("pairs-{s}.jsonl"))?, id_field)?;
        let runs = by_id(rows(format!("run-{s}.jsonl"))?, sample_id)?;
        for u in rows(format!("units-{s}.jsonl"))? {
            let sid = py::text(&u, "item_id")?;
            let units = match py::get(&u, "units") {
                None => &Vec::new(),
                Some(Value::Array(items)) => items,
                Some(other) => return Err(format!("units is {}", py::repr(other))),
            };
            let p = pairs
                .get(sid)
                .ok_or_else(|| format!("pairs-{s}.jsonl has no {}", py::str_repr(sid)))?;
            let mut altered = 0;
            let mut groups = Vec::new();
            for x in units {
                if !py::truthy(py::field(x, "kept")?) {
                    altered += 1;
                    groups.push(py::field(x, "group")?.clone());
                }
            }
            let words =
                |name| -> Result<usize, String> { Ok(py::words(py::text(p, name)?).count()) };
            let mut row = json!({
                "rewriter": s,
                "sample_id": sid,
                "units": units.len(),
                "units_altered": altered,
                "altered_groups": sorted_set(groups)?,
                "preserve_fails": flag(&work.join(format!("preserve/{s}-{sid}/fails")))?,
                "source_words": words("source")?,
                "rewrite_words": words("rewrite")?,
            });
            if let Some(run) = runs.get(sid) {
                let usage = py::get(run, "usage");
                row["output_tokens"] = usage_field(usage, "output_tokens")?;
                row["cost_usd"] = py::get(run, "cost_usd").cloned().unwrap_or(Value::Null);
            }
            if *s == "patina" {
                let v = verify
                    .get(sid)
                    .ok_or_else(|| format!("patina-verify.jsonl has no {}", py::str_repr(sid)))?;
                let mut fields = Map::new();
                for k in ["verified", "reason", "mps", "fidelity", "retried"] {
                    fields.insert(k.into(), py::get(v, k).cloned().unwrap_or(Value::Null));
                }
                row["patina_verify"] = Value::Object(fields);
            }
            out.push_str(&py::dumps(&row, None, true));
            out.push('\n');
        }
    }
    Ok(out)
}

/// A dict key as `json.dumps` writes one: text as it is, numbers and constants as JSON.
pub(super) fn json_key(value: &Value) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Array(_) | Value::Object(_) => Err(format!("unhashable: {}", py::repr(value))),
        other => Ok(py::dumps(other, None, true)),
    }
}

/// `dict(Counter(values))`: each distinct value's count, in order of first sight.
pub(super) fn counter(values: impl IntoIterator<Item = Value>) -> Result<Value, String> {
    let mut seen: Vec<(String, String, u64)> = Vec::new();
    for value in values {
        let key = py::key(&value);
        match seen.iter_mut().find(|(k, _, _)| *k == key) {
            Some(entry) => entry.2 += 1,
            None => {
                let name = json_key(&value)?;
                // Python would write 2 keys with one spelling, such as None and 'null'.
                if seen.iter().any(|(_, n, _)| *n == name) {
                    return Err(format!(
                        "{} and another value both count as {}",
                        py::repr(&value),
                        py::str_repr(&name)
                    ));
                }
                seen.push((key, name, 1));
            }
        }
    }
    Ok(Value::Object(
        seen.into_iter()
            .map(|(_, name, count)| (name, json!(count)))
            .collect(),
    ))
}

/// The rewrite run's table, recomputed from its rows: one line per rewriter.
pub fn rewrite_summary(rows: &[Value]) -> Result<String, String> {
    let mut rewriters: Vec<&Value> = Vec::new();
    for r in rows {
        let s = py::field(r, "rewriter")?;
        if !rewriters.contains(&s) {
            rewriters.push(s);
        }
    }
    let mut out = String::new();
    for s in rewriters {
        let rs: Vec<&Value> = rows.iter().filter(|r| &r["rewriter"] == s).collect();
        let (altered, kept): (Vec<&Value>, Vec<&Value>) = rs
            .iter()
            .copied()
            .partition(|r| py::truthy(&r["units_altered"]));
        let units = |r: &Value| num_at(r, &["units"]);
        let fails = |group: &[&Value]| -> Result<Value, String> {
            let failed = total_of(group, |r| num_at(r, &["preserve_fails"]))?.to_value();
            Ok(json!(format!("{failed} of {}", group.len())))
        };
        let mut ratios = Vec::new();
        for r in &rs {
            ratios.push(Num::Float(divide(
                num_at(r, &["rewrite_words"])?,
                num_at(r, &["source_words"])?,
            )?));
        }
        let mut groups = Vec::new();
        for r in &altered {
            groups.extend(items(py::field(r, "altered_groups")?)?.iter().cloned());
        }
        let mut line = Map::new();
        line.insert("texts".into(), json!(rs.len()));
        line.insert(
            "texts with a unit".into(),
            json!(rs.iter().filter(|r| py::truthy(&r["units"])).count()),
        );
        line.insert("units".into(), total_of(&rs, units)?.to_value());
        line.insert("documents with a unit altered".into(), json!(altered.len()));
        line.insert(
            "units altered".into(),
            total_of(&rs, |r| num_at(r, &["units_altered"]))?.to_value(),
        );
        line.insert("documents altered, by group".into(), counter(groups)?);
        line.insert("preserve fails, altered".into(), fails(&altered)?);
        line.insert("preserve fails, kept".into(), fails(&kept)?);
        line.insert(
            "mean length against source, in words".into(),
            json!(half_up(Num::Float(mean(&ratios)?), 2)?),
        );
        line.insert(
            "altered".into(),
            altered
                .iter()
                .map(|r| py::field(r, "sample_id").cloned())
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        );
        if rs.iter().any(|r| py::get(r, "cost_usd").is_some()) {
            let costs = rs
                .iter()
                .map(|r| match py::get(r, "cost_usd").filter(|c| py::truthy(c)) {
                    Some(c) => num(c),
                    None => Ok(Num::Int(0)),
                })
                .collect::<Result<Vec<_>, String>>()?;
            line.insert("cost US$".into(), json!(half_up(sum(costs), 2)?));
        }
        if s == "patina" {
            let v = rs
                .iter()
                .map(|r| py::field(r, "patina_verify"))
                .collect::<Result<Vec<_>, String>>()?;
            let column = |name| -> Result<Vec<Value>, String> {
                v.iter().map(|x| py::field(x, name).cloned()).collect()
            };
            let verified = column("verified")?;
            line.insert(
                "--verify passed".into(),
                json!(verified.iter().filter(|x| **x == json!(true)).count()),
            );
            let reasons = column("reason")?
                .into_iter()
                .zip(&verified)
                .filter(|(_, ok)| **ok != json!(true))
                .map(|(reason, _)| reason);
            line.insert("--verify reasons".into(), counter(reasons)?);
            line.insert("meaning score".into(), counter(column("mps")?)?);
            line.insert("fidelity".into(), counter(column("fidelity")?)?);
            line.insert(
                "retried".into(),
                json!(column("retried")?.iter().filter(|x| py::truthy(x)).count()),
            );
        }
        let _ = writeln!(
            out,
            "{}: {}",
            py::to_str(s),
            py::dumps(&Value::Object(line), None, true)
        );
    }
    Ok(out)
}

pub(super) fn items(value: &Value) -> Result<&Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{} is not a list", py::repr(value)))
}
