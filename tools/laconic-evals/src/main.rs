// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Laconic's evaluation harness: one subcommand per former Python entry point, with the same
//! flags, defaults, outputs and exit statuses.

mod agent_bench;
mod cases;
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
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "laconic-evals",
    version,
    about,
    infer_long_args = true,
    args_override_self = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

const CONDITIONS: [&str; 3] = run_evals::CONDITIONS;
const REFERENCES: [&str; 2] = ["baseline", "comparator"];

#[derive(Debug, Subcommand)]
enum Command {
    /// Validates the case catalogue.
    Validate {
        /// [default: evals/cases.jsonl]
        #[arg(long)]
        cases: Option<PathBuf>,
    },
    /// Prints the paired run matrix as JSON Lines.
    Plan {
        /// [default: evals/cases.jsonl]
        #[arg(long)]
        cases: Option<PathBuf>,
        #[arg(long, allow_negative_numbers = true, default_value_t = 3, value_parser = clap::value_parser!(i64).range(1..))]
        trials: i64,
        #[arg(long)]
        include_comparator: bool,
    },
    /// Aggregates judged score rows and applies the release gate.
    Score {
        scores: PathBuf,
        /// Condition the candidate is compared with.
        #[arg(long, default_value = "baseline", value_parser = REFERENCES)]
        reference: String,
    },
    /// Aggregates token and cost usage from recorded responses.
    Measure {
        responses: PathBuf,
        /// Condition the candidate is compared with.
        #[arg(long, default_value = "baseline", value_parser = REFERENCES)]
        reference: String,
    },
    /// Scores recorded responses against the cases' regex checks, without a judge.
    Checks {
        responses: PathBuf,
        /// [default: evals/cases.jsonl]
        #[arg(long)]
        cases: Option<PathBuf>,
    },
    /// Runs one evaluation condition; resumable.
    Run {
        /// [default: evals/cases.jsonl]
        #[arg(long)]
        cases: Option<PathBuf>,
        /// [default: evals/runners.example.json]
        #[arg(long)]
        runner_config: Option<PathBuf>,
        #[arg(long)]
        runner: String,
        #[arg(long, value_parser = CONDITIONS)]
        condition: String,
        #[arg(long)]
        condition_skill: Option<PathBuf>,
        #[arg(long)]
        case: Vec<String>,
        #[arg(long, allow_negative_numbers = true, default_value_t = 3, value_parser = clap::value_parser!(i64).range(1..))]
        trials: i64,
        #[arg(long, allow_negative_numbers = true, default_value_t = 2)]
        retries: i64,
        /// Send the task unchanged; the runner config supplies the setup being compared.
        #[arg(long)]
        no_injection: bool,
        /// Seconds before one model call counts as failed.
        #[arg(long, allow_negative_numbers = true, default_value_t = 300.0)]
        call_timeout: f64,
        /// Model calls to run at once; the budget is a soft threshold that up to jobs - 1
        /// running calls can pass.
        #[arg(long, allow_negative_numbers = true, default_value_t = 1)]
        jobs: i64,
        #[arg(long, allow_negative_numbers = true, default_value_t = 25.0)]
        budget_usd: f64,
        #[arg(long)]
        allow_unmetered: bool,
        #[arg(long)]
        output: PathBuf,
    },
    /// Blind-judges paired responses and writes scorable rows.
    Judge {
        /// Judge these hand-labelled responses and report blocker agreement instead of
        /// judging --responses.
        #[arg(long)]
        calibrate: Option<PathBuf>,
        #[arg(long)]
        responses: Option<PathBuf>,
        /// [default: evals/cases.jsonl]
        #[arg(long)]
        cases: Option<PathBuf>,
        /// [default: evals/rubric.md]
        #[arg(long)]
        rubric: Option<PathBuf>,
        /// [default: evals/runners.example.json]
        #[arg(long)]
        runner_config: Option<PathBuf>,
        #[arg(long)]
        runner: String,
        /// Conditions that every response group must contain.
        #[arg(long, num_args = 1.., value_parser = CONDITIONS, default_values = ["baseline", "candidate"])]
        conditions: Vec<String>,
        #[arg(long, allow_negative_numbers = true, default_value_t = 2)]
        retries: i64,
        /// Seconds before one judge call counts as failed.
        #[arg(long, allow_negative_numbers = true, default_value_t = 300.0)]
        call_timeout: f64,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Captures a resumed Claude conversation for the judge and score commands.
    Scenario {
        #[command(subcommand)]
        command: Scenario,
    },
    /// Checks laconic-check preserve against the fact audit of the AI Humanizer Benchmark.
    FactAudit {
        /// The laconic-check binary.
        #[arg(long, default_value = "laconic-check")]
        laconic_check: String,
        /// Where the downloads are kept [default: evals/results/fact-audit].
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Run preserve with --loose.
        #[arg(long)]
        loose: bool,
    },
    /// Measures whether rewrites keep each fact and qualifier in a source text.
    FactSurvival {
        #[command(subcommand)]
        command: Survival,
    },
    /// Fetches rival skills at pinned commits and writes the fact-survival runner config.
    FactSetups {
        /// Make one call per skill setup, proving it loads its own skill.
        #[arg(long)]
        preflight: bool,
        folder: PathBuf,
    },
    /// Scores humanising responses with deterministic checkers, without a judge.
    HumaniseMetrics {
        responses: PathBuf,
        /// [default: evals/humanise-cases.jsonl]
        #[arg(long)]
        cases: Option<PathBuf>,
        /// Path to avoid-ai-writing's bin/avoid-ai-writing.js.
        #[arg(long)]
        avoid: Option<PathBuf>,
        /// Also run dslop through uvx with evals/humanise-dslop.toml.
        #[arg(long)]
        dslop: bool,
    },
    /// Agentic cost benchmark: <rules-a.md> <rules-b.md> <trials> <out.jsonl>.
    AgentBench {
        #[arg(num_args = 0.., allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum Scenario {
    /// Checks a scenario folder without calling a provider.
    Validate { scenario: PathBuf },
    /// Runs every turn in one claude session and saves the conversation, ungraded.
    Run {
        #[arg(long)]
        scenario: PathBuf,
        #[arg(long, value_parser = ["baseline", "candidate"])]
        condition: String,
        #[arg(long)]
        condition_skill: Option<PathBuf>,
        #[arg(long)]
        model: String,
        #[arg(long, allow_negative_numbers = true, default_value_t = 1)]
        trial: i64,
        #[arg(long, allow_negative_numbers = true)]
        budget_usd: f64,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum Survival {
    /// Writes the fact cases: the humanise sources under one neutral instruction.
    BuildCases {
        /// [default: evals/humanise-cases.jsonl]
        #[arg(long)]
        sources: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Judges each response against its ledger; resumable.
    Judge {
        responses: PathBuf,
        /// [default: evals/fact-ledgers.jsonl]
        #[arg(long)]
        ledgers: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, allow_negative_numbers = true, default_value_t = 4)]
        jobs: i64,
        /// [default: evals/fact-cases.jsonl]
        #[arg(long)]
        cases: Option<PathBuf>,
        #[arg(long, allow_negative_numbers = true, default_value_t = 12.0)]
        budget_usd: f64,
        #[arg(long, allow_negative_numbers = true, default_value_t = 300.0)]
        call_timeout: f64,
    },
    /// Measures the judge against hand-labelled rewrites.
    Calibrate {
        /// [default: evals/fact-calibration.jsonl]
        #[arg(long)]
        fixtures: Option<PathBuf>,
        /// [default: evals/fact-ledgers.jsonl]
        #[arg(long)]
        ledgers: Option<PathBuf>,
        /// [default: evals/fact-cases.jsonl]
        #[arg(long)]
        cases: Option<PathBuf>,
        /// Append this run's cost to this file.
        #[arg(long)]
        overhead: Option<PathBuf>,
    },
    /// Exit 1 once reported cost reaches the cap.
    Spend {
        folder: PathBuf,
        #[arg(long, allow_negative_numbers = true)]
        cap: f64,
    },
    /// Summarises each setup's scores against the reference setup.
    Report {
        scores: PathBuf,
        responses: PathBuf,
        #[arg(long)]
        cases: PathBuf,
        #[arg(long)]
        reference: String,
        #[arg(long, allow_negative_numbers = true, default_value_t = 2)]
        trials: i64,
    },
}

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

/// A file in the repository's evals folder.
fn evals(name: &str) -> PathBuf {
    cases::root().join("evals").join(name)
}

fn or_evals(path: Option<PathBuf>, name: &str) -> PathBuf {
    path.unwrap_or_else(|| evals(name))
}

fn append(path: &Path, line: &str) -> Result<(), String> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(line.as_bytes()))
        .map_err(|e| format!("{}: {e}", path.display()))
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
            writeln!(out, "judge cost: ${}", fact_survival::money(spent))
                .map_err(|e| e.to_string())?;
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
            writeln!(out, "{}", py::dumps_pretty(&result)).map_err(|e| e.to_string())?;
            if let Some(overhead) = overhead {
                let line =
                    serde_json::json!({"stage": "calibrate", "cost_usd": result["cost_usd"]});
                append(&overhead, &format!("{}\n", py::dumps(&line, None, true)))?;
            }
        }
        Survival::Spend { folder, cap } => {
            let total = fact_survival::total_spend(&folder)?;
            writeln!(
                out,
                "reported cost: ${} of ${}",
                fact_survival::money(total),
                fact_survival::money(cap)
            )
            .map_err(|e| e.to_string())?;
            return Ok(u8::from(total >= cap));
        }
        Survival::Report {
            scores,
            responses,
            cases,
            reference,
            trials,
        } => {
            let setups: BTreeSet<String> = fact_survival::SETUPS.map(str::to_owned).into();
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
            writeln!(out, "{}", py::dumps_pretty(&summary)).map_err(|e| e.to_string())?;
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
            let cache = cache.unwrap_or_else(|| evals("results").join("fact-audit"));
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
    out.write_all(printed.as_bytes())
        .map_err(|e| e.to_string())?;
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
