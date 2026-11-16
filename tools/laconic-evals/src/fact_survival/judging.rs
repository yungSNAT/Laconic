// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

/// Judges one row, retrying a reply it cannot read up to 3 calls in all.
fn judge_one(
    row: &Value,
    ledgers: &Ledgers,
    sources: &Sources,
    call: Call,
) -> Result<Value, JudgeFailed> {
    let (runner, case_id) = (&row["runner"], &row["case_id"]);
    let label = format!("{} {}", py::to_str(runner), py::to_str(case_id));
    let items = &ledgers[case_id.as_str().expect("a case with a ledger")];
    let failed = |error: String, cost: f64| JudgeFailed {
        message: format!("{label}: {error}"),
        cost,
    };
    // The prompt's inputs are read before each call, as Python built its arguments.
    let ask = || -> Result<(Value, f64, &str), String> {
        let source = sources.get(case_id.as_str().expect("a case with a ledger"));
        let source = source.ok_or_else(|| py::str_repr(&py::to_str(case_id)))?;
        let response = text(row, "response")?;
        let (reply, spent) = call(&judge_prompt(source, response, items)?)?;
        Ok((reply, spent, response))
    };
    let mut cost = 0.0;
    let mut attempt = 0;
    loop {
        attempt += 1;
        let (reply, spent, response) = ask().map_err(|error| failed(error, cost))?;
        cost += spent;
        match parse_reply(&reply, response, items) {
            Ok((verdicts, invented)) => {
                return Ok(py::object([
                    ("setup", runner.clone()),
                    ("case_id", case_id.clone()),
                    ("trial", row["trial"].clone()),
                    ("cost", py::float(cost)),
                    ("verdicts", Value::from(verdicts)),
                    ("invented", Value::from(invented)),
                ]));
            }
            Err(ReplyError::Fatal(error)) => return Err(failed(error, 0.0)),
            Err(ReplyError::Retry(error)) if attempt == 3 => return Err(failed(error, cost)),
            Err(ReplyError::Retry(_)) => {}
        }
    }
}

/// Judge rows not yet in `output`; resumable. Returns the cost of this run.
///
/// Up to `jobs` rows are judged at once, in a fixed shuffled order that spreads a budget stop
/// across setups. Once the cost reaches the budget, or a row fails, no new row starts; rows
/// already running finish and are saved. A failed row's cost goes to overhead.jsonl beside
/// `output`, and the first failure is returned once every running row has finished.
pub fn judge_rows<'a>(
    rows: &'a [Value],
    ledgers: &Ledgers,
    sources: &Sources,
    output: &Path,
    call: Call,
    jobs: i64,
    budget_usd: f64,
) -> Result<f64, String> {
    let mut done = HashSet::new();
    if output.exists() {
        for row in read_jsonl(output)? {
            done.insert(row_key(&row, "setup")?);
        }
    }
    let mut todo = Vec::new();
    for row in rows {
        if !done.contains(&row_key(row, "runner")?) {
            todo.push(row);
        }
    }
    let mut missing: Vec<&Value> = todo
        .iter()
        .map(|row| &row["case_id"])
        .filter(|id| !id.as_str().is_some_and(|id| ledgers.contains_key(id)))
        .collect();
    missing.sort_by(|a, b| py::compare(a, b).unwrap_or(std::cmp::Ordering::Equal));
    missing.dedup_by(|a, b| py::key(a) == py::key(b));
    if !missing.is_empty() {
        let shown: Vec<Value> = missing.into_iter().cloned().collect();
        return Err(format!("no ledger for: {}", py::repr(&Value::from(shown))));
    }
    py::Random::new(0).shuffle(&mut todo);
    let jobs = usize::try_from(jobs).ok().filter(|&jobs| jobs > 0);
    let jobs = jobs.ok_or("max_workers must be greater than 0")?;
    append(output, "")?;
    let overhead = output
        .parent()
        .unwrap_or(Path::new(""))
        .join("overhead.jsonl");
    let (sender, receiver) = mpsc::channel();
    let mut pending = todo.into_iter();
    let mut spent = 0.0;
    let mut failure: Option<String> = None;
    let mut error = None;
    thread::scope(|scope| {
        let submit = |row: &'a Value| {
            let sender = sender.clone();
            scope.spawn(move || sender.send(judge_one(row, ledgers, sources, call)));
        };
        let mut running = 0;
        for row in pending.by_ref().take(jobs) {
            submit(row);
            running += 1;
        }
        while running > 0 {
            running -= 1;
            let written = match receiver.recv().expect("a running row reports") {
                Err(failed) => {
                    if failure.is_none() {
                        failure = Some(failed.message);
                    }
                    let line = json!({"stage": "failed-judge", "cost_usd": py::float(failed.cost)});
                    append(&overhead, &format!("{}\n", py::dumps(&line, None, true)))
                }
                Ok(result) => {
                    spent += result["cost"].as_f64().expect("a cost");
                    let written = append(output, &format!("{}\n", py::dumps_row(&result)));
                    if written.is_ok()
                        && spent < budget_usd
                        && failure.is_none()
                        && let Some(row) = pending.next()
                    {
                        submit(row);
                        running += 1;
                    }
                    written
                }
            };
            if let Err(e) = written {
                error = Some(e);
                break;
            }
        }
    });
    match (error, failure) {
        (Some(error), _) | (None, Some(error)) => Err(error),
        (None, None) => Ok(spent),
    }
}

/// The judge against hand-labelled rewrites. '@source' stands for a verbatim copy.
pub fn calibrate(
    fixtures: &[Value],
    ledgers: &Ledgers,
    sources: &Sources,
    call: Call,
) -> Result<Value, String> {
    let (mut errors, mut compared, mut cost) = (Vec::new(), 0u64, 0.0);
    for fixture in fixtures {
        let case_id = field(fixture, "case_id")?;
        let missing = || format!("no case {}", py::repr(case_id));
        let source = case_id
            .as_str()
            .and_then(|id| sources.get(id))
            .ok_or_else(missing)?;
        let rewrite = match field(fixture, "rewrite")? {
            Value::String(r) if r == "@source" => source.as_str(),
            _ => text(fixture, "rewrite")?,
        };
        let items = case_id
            .as_str()
            .and_then(|id| ledgers.get(id))
            .ok_or_else(missing)?;
        let (reply, spent) = call(&judge_prompt(source, rewrite, items)?)?;
        cost += spent;
        let (verdicts, invented) =
            parse_reply(&reply, rewrite, items).map_err(ReplyError::message)?;
        for verdict in verdicts {
            let expected = field(fixture, "expected")?;
            if !expected.is_object() {
                return Err(format!(
                    "'expected' is not an object in {}",
                    py::repr(fixture)
                ));
            }
            let default = field(fixture, "default")?;
            let given = verdict["id"].as_str().and_then(|id| py::get(expected, id));
            let expected = given.unwrap_or(default);
            compared += 1;
            if &verdict["verdict"] != expected {
                errors.push(py::object([
                    ("fixture", field(fixture, "name")?.clone()),
                    ("item", verdict["id"].clone()),
                    ("expected", expected.clone()),
                    ("got", verdict["verdict"].clone()),
                ]));
            }
        }
        compared += 1;
        let wanted = field(fixture, "invented")?;
        if py::key(&Value::from(!invented.is_empty())) != py::key(wanted) {
            errors.push(py::object([
                ("fixture", field(fixture, "name")?.clone()),
                ("item", json!("invented")),
                ("expected", wanted.clone()),
                ("got", Value::from(invented)),
            ]));
        }
    }
    if compared == 0 {
        return Err("division by zero: no calibration fixtures".into());
    }
    #[expect(clippy::cast_precision_loss, reason = "counts are below 2**53")]
    let agreement = 1.0 - errors.len() as f64 / compared as f64;
    Ok(py::object([
        ("compared", Value::from(compared)),
        ("agreement", py::float(agreement)),
        ("errors", Value::from(errors)),
        ("cost_usd", py::float(cost)),
    ]))
}
