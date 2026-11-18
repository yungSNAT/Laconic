// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Parser)]
#[command(
    name = "laconic-evals",
    version,
    about,
    infer_long_args = true,
    args_override_self = true
)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Command,
}

pub(super) const CONDITIONS: [&str; 3] = run_evals::CONDITIONS;
pub(super) const REFERENCES: [&str; 2] = ["baseline", "comparator"];

#[derive(Debug, Subcommand)]
pub(super) enum Command {
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
        #[arg(long, default_value_os_t = evals("results").join("fact-audit"), hide_default_value = true)]
        cache: PathBuf,
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
    /// The steps of the dated runs in evals/runs/, which their run.sh files call.
    Record {
        #[command(subcommand)]
        command: Record,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum Scenario {
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
pub(super) enum Survival {
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

#[derive(Debug, Subcommand)]
pub(super) enum Record {
    /// Writes a run's runner configurations into its work directory, and for the rewrite run
    /// its cases and patina's input files.
    Setups {
        #[arg(value_parser = record::RUNS)]
        run: String,
        work: PathBuf,
    },
    /// Writes each answer in responses.jsonl and warm.jsonl to answers/<run>-<n>.txt.
    Answers { work: PathBuf },
    /// Prints one row per answer of the Concise or token-cost run.
    StyleResults { work: PathBuf },
    /// Prints the Concise or token-cost run's figures, recomputed from its results.jsonl.
    StyleSummary { results: PathBuf },
    /// Writes the rewrite run's source and rewrite pairs and patina's --verify results.
    RewritePairs { work: PathBuf },
    /// Prints one row per rewrite of the rewrite run.
    RewriteResults { work: PathBuf },
    /// Prints the rewrite run's table, recomputed from its results.jsonl.
    RewriteSummary { results: PathBuf },
}
