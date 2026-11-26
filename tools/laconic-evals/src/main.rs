// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Laconic's evaluation harness: one subcommand per former Python entry point, with the same
//! flags, defaults, outputs and exit statuses.

mod agent_bench;
mod cases;
mod cli;
mod fact_audit;
mod fact_setups;
mod fact_survival;
mod humanise;
mod judge;
mod proc;
mod py;
mod run_evals;
mod scenario;
#[cfg(test)]
mod testing;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, Subcommand};
use cli::*;
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

/// Why a command stopped: a bad argument (exit 2, with the usage) or a failure (exit 1).
#[derive(Debug, PartialEq)]
enum Stop {
    Usage(String),
    Failed(String),
}

impl From<String> for Stop {
    fn from(message: String) -> Self {
        Stop::Failed(message)
    }
}

impl From<&str> for Stop {
    fn from(message: &str) -> Self {
        Stop::Failed(message.to_owned())
    }
}

/// A failed write to standard output.
impl From<std::io::Error> for Stop {
    fn from(error: std::io::Error) -> Self {
        Stop::Failed(error.to_string())
    }
}

/// A file in the repository's evals folder.
fn evals(name: &str) -> PathBuf {
    cases::root().join("evals").join(name)
}

fn or_evals(path: Option<PathBuf>, name: &str) -> PathBuf {
    path.unwrap_or_else(|| evals(name))
}

/// One judge call with a timeout.
type Judge<'a> = &'a (dyn Fn(&str, Duration) -> Result<(serde_json::Value, f64), String> + Sync);

/// The outside programs the commands call, so tests can stand in for each.
struct Tools<'a> {
    judge: Judge<'a>,
    /// laconic-check and the other checkers, with a 120-second timeout.
    checker: fact_audit::Exec<'a>,
    /// laconic-check preserve for the fact audit, without a timeout.
    preserve: fact_audit::Exec<'a>,
    /// The fact-audit downloads.
    get: &'a dyn Fn(&str) -> Result<Vec<u8>, String>,
    git: fact_setups::CheckCall<'a>,
    claude: fact_setups::Exec<'a>,
    run_in: agent_bench::Run<'a>,
    which: &'a dyn Fn(&str) -> Option<PathBuf>,
    /// The runner and judge commands of `run`, `judge` and `scenario run`.
    runner: run_evals::Exec<'a>,
    sleep: run_evals::Sleep<'a>,
    /// Today's date in ISO form, in local time.
    today: &'a (dyn Fn() -> String + Sync),
    /// A new claude session id.
    session: &'a dyn Fn() -> String,
}

/// The production judge: one claude CLI call.
fn claude_judge(prompt: &str, timeout: Duration) -> Result<(serde_json::Value, f64), String> {
    fact_survival::call_judge(
        prompt,
        &fact_survival::JUDGE_COMMAND.map(str::to_owned),
        timeout,
    )
}

const TOOLS: Tools<'static> = Tools {
    judge: &claude_judge,
    checker: &humanise::exec,
    preserve: &fact_audit::exec,
    get: &fact_audit::http_get,
    git: &fact_setups::check_call,
    claude: &fact_setups::exec,
    run_in: &agent_bench::run_in,
    which: &fact_audit::which,
    runner: &proc::run,
    sleep: &std::thread::sleep,
    today: &today,
    session: &session,
};

fn today() -> String {
    jiff::ZonedDateTime::now().date().to_string()
}

fn session() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// A `--call-timeout` a timer can use.
fn call_timeout(seconds: f64) -> Result<f64, Stop> {
    Duration::try_from_secs_f64(seconds)
        .map(|_| seconds)
        .map_err(|e| Stop::Usage(format!("--call-timeout: {e}")))
}

fn survival(command: Survival, out: &mut dyn Write, tools: &Tools) -> Result<u8, Stop> {
    let call = tools.judge;
    let judge = |timeout: Duration| move |prompt: &str| call(prompt, timeout);
    use fact_survival::{load_ledgers, read_jsonl, sources};
    match command {
        Survival::BuildCases { sources, output } => {
            let built = fact_survival::build_cases(&read_jsonl(&or_evals(
                sources,
                "humanise-cases.jsonl",
            ))?)?;
            let lines: Vec<String> = built.iter().map(py::dumps_row).collect();
            std::fs::write(&output, format!("{}\n", lines.join("\n")))
                .map_err(|e| format!("{}: {e}", output.display()))?;
        }
        Survival::Judge {
            responses,
            ledgers,
            output,
            jobs,
            cases,
            budget_usd,
            call_timeout,
        } => {
            let timeout = Duration::try_from_secs_f64(call_timeout)
                .map_err(|e| Stop::Usage(format!("--call-timeout: {e}")))?;
            let (rows, ledgers) = (
                read_jsonl(&responses)?,
                load_ledgers(&or_evals(ledgers, "fact-ledgers.jsonl"))?,
            );
            let sources = sources(&or_evals(cases, "fact-cases.jsonl"))?;
            let spent = fact_survival::judge_rows(
                &rows,
                &ledgers,
                &sources,
                &output,
                &judge(timeout),
                jobs,
                budget_usd,
            )?;
            writeln!(out, "judge cost: ${}", fact_survival::money(spent))?;
        }
        Survival::Calibrate {
            fixtures,
            ledgers,
            cases,
            overhead,
        } => {
            let fixtures = read_jsonl(&or_evals(fixtures, "fact-calibration.jsonl"))?;
            let (ledgers, sources) = (
                load_ledgers(&or_evals(ledgers, "fact-ledgers.jsonl"))?,
                sources(&or_evals(cases, "fact-cases.jsonl"))?,
            );
            let result = fact_survival::calibrate(
                &fixtures,
                &ledgers,
                &sources,
                &judge(Duration::from_secs(300)),
            )?;
            writeln!(out, "{}", py::dumps_pretty(&result))?;
            if let Some(overhead) = overhead {
                let line =
                    serde_json::json!({"stage": "calibrate", "cost_usd": result["cost_usd"]});
                cases::append(&overhead, &format!("{}\n", py::dumps(&line, None, true)))?;
            }
        }
        Survival::Spend { folder, cap } => {
            let total = fact_survival::total_spend(&folder)?;
            writeln!(
                out,
                "reported cost: ${} of ${}",
                fact_survival::money(total),
                fact_survival::money(cap)
            )?;
            return Ok(u8::from(total >= cap));
        }
        Survival::Report {
            scores,
            responses,
            cases,
            reference,
            trials,
            setups,
        } => {
            let setups: BTreeSet<String> = setups
                .unwrap_or_else(|| fact_survival::SETUPS.map(str::to_owned).into())
                .into_iter()
                .collect();
            let added = |source: &str, rewrite: &str| {
                fact_survival::added_count(tools.checker, source, rewrite)
            };
            let summary = fact_survival::summarise(
                &read_jsonl(&scores)?,
                &read_jsonl(&responses)?,
                &sources(&cases)?,
                &reference,
                &added,
                trials,
                Some(&setups),
            )?;
            writeln!(out, "{}", py::dumps_pretty(&summary))?;
        }
    }
    Ok(0)
}

/// Runs one parsed command, writing what it prints to `out`; returns the exit status.
fn run(cli: Cli, out: &mut dyn Write, tools: &Tools) -> Result<u8, Stop> {
    let printed = match cli.command {
        Command::Validate { cases } => {
            return Ok(run_evals::validate(&or_evals(cases, "cases.jsonl"), out)?);
        }
        Command::Plan {
            cases,
            trials,
            include_comparator,
        } => run_evals::plan(&or_evals(cases, "cases.jsonl"), trials, include_comparator)?,
        Command::Score { scores, reference } => {
            let rows = cases::read_jsonl(&scores)?;
            format!(
                "{}\n",
                py::dumps_pretty(&run_evals::summarize_scores(&rows, &reference)?)
            )
        }
        Command::Measure {
            responses,
            reference,
        } => {
            let rows = cases::read_jsonl(&responses)?;
            format!(
                "{}\n",
                py::dumps_pretty(&run_evals::summarize_usage(&rows, &reference)?)
            )
        }
        Command::Checks { responses, cases } => {
            let cases = run_evals::valid_cases(&or_evals(cases, "cases.jsonl"))?;
            let summary = run_evals::summarize_checks(&cases, &cases::read_jsonl(&responses)?)?;
            format!("{}\n", py::dumps_pretty(&summary))
        }
        Command::Run {
            cases,
            runner_config,
            runner,
            condition,
            condition_skill,
            case,
            trials,
            retries,
            no_injection,
            call_timeout: timeout,
            jobs,
            budget_usd,
            allow_unmetered,
            output,
        } => {
            let args = run_evals::RunArgs {
                cases: or_evals(cases, "cases.jsonl"),
                runner_config: or_evals(runner_config, "runners.example.json"),
                runner,
                condition,
                condition_skill,
                case,
                trials,
                retries,
                no_injection,
                call_timeout: call_timeout(timeout)?,
                jobs,
                budget_usd,
                allow_unmetered,
                output,
            };
            let world = run_evals::World {
                exec: tools.runner,
                sleep: tools.sleep,
                today: tools.today,
            };
            return Ok(run_evals::run_evaluations(&args, &world, out)?);
        }
        Command::Judge {
            calibrate,
            responses,
            cases,
            rubric,
            runner_config,
            runner,
            conditions,
            retries,
            call_timeout: timeout,
            output,
        } => {
            if calibrate.is_none() && (responses.is_none() || output.is_none()) {
                return Err(Stop::Usage(
                    "--responses and --output are required unless --calibrate is given".into(),
                ));
            }
            let args = judge::JudgeArgs {
                calibrate,
                responses,
                cases: or_evals(cases, "cases.jsonl"),
                rubric: or_evals(rubric, "rubric.md"),
                runner_config: or_evals(runner_config, "runners.example.json"),
                runner,
                conditions,
                retries,
                call_timeout: call_timeout(timeout)?,
                output,
            };
            return Ok(judge::command(&args, tools.runner, tools.sleep, out)?);
        }
        Command::Scenario {
            command: Scenario::Validate { scenario },
        } => {
            scenario::load_scenario(&scenario)?;
            let name = scenario
                .file_name()
                .map(|n| n.to_string_lossy().into_owned());
            format!("Valid scenario: {}\n", name.unwrap_or_default())
        }
        Command::Scenario {
            command:
                Scenario::Run {
                    scenario,
                    condition,
                    condition_skill,
                    model,
                    trial,
                    budget_usd,
                    output,
                },
        } => {
            let args = scenario::ScenarioArgs {
                scenario,
                condition,
                condition_skill,
                model,
                trial,
                budget_usd,
                output,
            };
            scenario::run_scenario(
                &args,
                tools.runner,
                tools.session,
                scenario::CALL_TIMEOUT,
                out,
            )?;
            String::new()
        }
        Command::FactAudit {
            laconic_check,
            cache,
            loose,
        } => {
            let binary = (tools.which)(&laconic_check).ok_or_else(|| {
                Stop::Usage(format!(
                    "--laconic-check {laconic_check} is not an executable file"
                ))
            })?;
            let check = |binary: &str, source: &str, rewrite: &str| {
                fact_audit::preserve(binary, source, rewrite, loose, tools.preserve)
            };
            let (counts, differences) =
                fact_audit::run(&binary.display().to_string(), &cache, &check, tools.get)?;
            fact_audit::report(&counts, &differences)
        }
        Command::FactSurvival { command } => return survival(command, out, tools),
        Command::FactSetups { preflight, folder } => {
            let here = fact_setups::resolve(&folder)?;
            if preflight {
                return Ok(
                    u8::try_from(fact_setups::preflight(&here, tools.claude, out)?)
                        .expect("0 or 1"),
                );
            }
            format!(
                "{}\n",
                fact_setups::setup(&here, "https://github.com", tools.git)?
            )
        }
        Command::HumaniseMetrics {
            responses,
            cases,
            avoid,
            dslop,
        } => {
            let config = dslop.then(|| evals("humanise-dslop.toml"));
            let checkers = humanise::Checkers {
                avoid: avoid.as_deref(),
                dslop: config.as_deref(),
            };
            let cases = or_evals(cases, "humanise-cases.jsonl");
            format!(
                "{}\n",
                humanise::command(&responses, &cases, &checkers, tools.checker)?
            )
        }
        Command::AgentBench { args } => {
            let python = agent_bench::python(tools.which)?;
            let session = |claude: &Path, rules: &str, root: &Path| {
                agent_bench::run(claude, rules, root, tools.run_in)
            };
            let claude = (tools.which)("claude");
            agent_bench::command(
                &args,
                claude.as_deref(),
                &python,
                &session,
                tools.run_in,
                out,
            )?;
            String::new()
        }
    };
    out.write_all(printed.as_bytes())?;
    Ok(0)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli, &mut std::io::stdout().lock(), &TOOLS) {
        Ok(code) => ExitCode::from(code),
        Err(Stop::Usage(message)) => Cli::command()
            .error(ErrorKind::ValueValidation, message)
            .exit(),
        Err(Stop::Failed(message)) => {
            eprintln!("laconic-evals: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
