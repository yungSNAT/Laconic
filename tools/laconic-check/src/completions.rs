// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Shell completion scripts for `laconic-check`, built with no external dependencies.

use crate::lint;
use crate::profile::Profile;

/// Supported shells for `laconic-check completions <shell>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl Shell {
    /// Parses a shell name accepted by the CLI.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" => Some(Self::Fish),
            "powershell" => Some(Self::PowerShell),
            _ => None,
        }
    }
}

/// Returns a completion script for `shell`, covering every subcommand, flag and built-in rule.
pub fn script(shell: Shell) -> String {
    let rules: Vec<&str> = lint::RULES.iter().map(|rule| rule.name).collect();
    match shell {
        Shell::Bash => bash(&rules),
        Shell::Zsh => zsh(&rules),
        Shell::Fish => fish(&rules),
        Shell::PowerShell => powershell(&rules),
    }
}

fn bash(rules: &[&str]) -> String {
    let rules = rules.join(" ");
    let profiles = Profile::NAMES.join(" ");
    format!(
        r#"_laconic_check() {{
    local cur prev cmd
    cur="${{COMP_WORDS[COMP_CWORD]}}"
    prev="${{COMP_WORDS[COMP_CWORD-1]}}"

    if [[ $COMP_CWORD -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "lint preserve mcp lsp hook completions instructions --help --version" -- "$cur") )
        return
    fi

    cmd="${{COMP_WORDS[1]}}"
    case "$cmd" in
        lint)
            case "$prev" in
                --disable|--only)
                    COMPREPLY=( $(compgen -W "{rules}" -- "$cur") )
                    return
                    ;;
                --format)
                    COMPREPLY=( $(compgen -W "text json sarif" -- "$cur") )
                    return
                    ;;
                --profile)
                    COMPREPLY=( $(compgen -W "{profiles}" -- "$cur") )
                    return
                    ;;
                --config)
                    COMPREPLY=( $(compgen -f -- "$cur") )
                    return
                    ;;
                --max-findings)
                    return
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--format --json --sarif --metrics --profile --disable --only --max-findings --warn --commit-msg --config" -- "$cur") )
            else
                COMPREPLY=( $(compgen -f -- "$cur") )
            fi
            ;;
        preserve)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--strict --loose" -- "$cur") )
            else
                COMPREPLY=( $(compgen -f -- "$cur") )
            fi
            ;;
        completions)
            if [[ $COMP_CWORD -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "bash zsh fish powershell" -- "$cur") )
            fi
            ;;
        instructions)
            if [[ $COMP_CWORD -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "config rules" -- "$cur") )
            fi
            ;;
        mcp|lsp|hook)
            ;;
    esac
}}

complete -F _laconic_check laconic-check
"#
    )
}

fn zsh(rules: &[&str]) -> String {
    let rules = rules.join(" ");
    let profiles = Profile::NAMES.join(" ");
    format!(
        r#"#compdef laconic-check

_laconic-check() {{
    local -a commands
    commands=(
        'lint:Flag house-style candidates in Markdown prose'
        'preserve:Check that an edit kept numbers, dates, code, URLs and quotations'
        'mcp:Serve lint and preserve as MCP tools over standard input and output'
        'lsp:Run the language server over standard input and output'
        'hook:Run the Stop hook for Claude Code and Codex'
        'completions:Print a shell completion script'
        'instructions:Print what a coding agent needs to use laconic-check'
    )

    _arguments -C \
        '--help[Print usage information]' \
        '--version[Print version information]' \
        '1:command:->command' \
        '*::arg:->args'

    case "$state" in
        command)
            _describe -t commands 'laconic-check command' commands
            ;;
        args)
            case "${{words[1]}}" in
                lint)
                    _arguments \
                        '--format[Output format]:format:(text json sarif)' \
                        '--json[Print one JSON document instead of one line a finding]' \
                        '--sarif[Print a SARIF 2.1.0 log instead of one line a finding]' \
                        '--metrics[Print distributional prose metrics per file and overall]' \
                        '--profile=[Apply a register preset]:profile:({profiles})' \
                        '*--disable=[Drop findings for these rules]:rules:({rules})' \
                        '*--only=[Keep findings only for these rules]:rules:({rules})' \
                        '--max-findings=[Fail only on more than n failing findings]:count:' \
                        '--warn[Never fail on findings]' \
                        '--commit-msg[Read each file as a Git commit message]' \
                        '--config=[Read settings from this file instead of laconic.toml or .laconic-check]:file:_files' \
                        '*:file:_files'
                    ;;
                preserve)
                    _arguments \
                        '--strict[Fail on added items as well as dropped items]' \
                        '--loose[List lost hedge or frequency words to check]' \
                        '1:original:_files' \
                        '2:edited:_files'
                    ;;
                completions)
                    _arguments \
                        '1:shell:(bash zsh fish powershell)'
                    ;;
                instructions)
                    _arguments \
                        '1:topic:(config rules)'
                    ;;
                mcp|lsp|hook)
                    ;;
            esac
            ;;
    esac
}}

if [ "$funcstack[1]" = "_laconic-check" ]; then
    _laconic-check "$@"
else
    compdef _laconic-check laconic-check
fi
"#
    )
}

fn fish(rules: &[&str]) -> String {
    let rules = rules.join(" ");
    let profiles = Profile::NAMES.join(" ");
    format!(
        r#"complete -c laconic-check -n "__fish_use_subcommand" -l help -d "Print usage information"
complete -c laconic-check -n "__fish_use_subcommand" -l version -d "Print version information"
complete -c laconic-check -n "__fish_use_subcommand" -f -a "lint" -d "Flag house-style candidates in Markdown prose"
complete -c laconic-check -n "__fish_use_subcommand" -f -a "preserve" -d "Check that an edit kept protected items"
complete -c laconic-check -n "__fish_use_subcommand" -f -a "mcp" -d "Serve lint and preserve as MCP tools"
complete -c laconic-check -n "__fish_use_subcommand" -f -a "lsp" -d "Run the language server"
complete -c laconic-check -n "__fish_use_subcommand" -f -a "hook" -d "Run the Stop hook for Claude Code and Codex"
complete -c laconic-check -n "__fish_use_subcommand" -f -a "completions" -d "Print a shell completion script"
complete -c laconic-check -n "__fish_use_subcommand" -f -a "instructions" -d "Print what a coding agent needs to use laconic-check"

complete -c laconic-check -n "__fish_seen_subcommand_from lint" -l json -d "Print one JSON document"
complete -c laconic-check -n "__fish_seen_subcommand_from lint" -l sarif -d "Print a SARIF 2.1.0 log"
complete -c laconic-check -n "__fish_seen_subcommand_from lint" -l metrics -d "Print distributional prose metrics"
complete -c laconic-check -n "__fish_seen_subcommand_from lint" -l format -r -f -a "text json sarif" -d "Output format"
complete -c laconic-check -n "__fish_seen_subcommand_from lint" -l profile -r -f -a "{profiles}" -d "Apply a register preset"
complete -c laconic-check -n "__fish_seen_subcommand_from lint" -l disable -r -f -a "{rules}" -d "Drop findings for these rules"
complete -c laconic-check -n "__fish_seen_subcommand_from lint" -l only -r -f -a "{rules}" -d "Keep findings only for these rules"
complete -c laconic-check -n "__fish_seen_subcommand_from lint" -l max-findings -r -f -d "Fail only on more than n failing findings"
complete -c laconic-check -n "__fish_seen_subcommand_from lint" -l warn -d "Never fail on findings"
complete -c laconic-check -n "__fish_seen_subcommand_from lint" -l commit-msg -d "Read each file as a Git commit message"
complete -c laconic-check -n "__fish_seen_subcommand_from lint" -l config -r -F -d "Read settings from this file"

complete -c laconic-check -n "__fish_seen_subcommand_from preserve" -l strict -d "Fail on added items as well as dropped items"
complete -c laconic-check -n "__fish_seen_subcommand_from preserve" -l loose -d "List lost hedge or frequency words to check"

complete -c laconic-check -n "__fish_seen_subcommand_from completions; and not __fish_seen_subcommand_from bash zsh fish powershell" -f -a "bash zsh fish powershell" -d "Target shell"
complete -c laconic-check -n "__fish_seen_subcommand_from instructions; and not __fish_seen_subcommand_from config rules" -f -a "config rules" -d "Topic"
complete -c laconic-check -n "__fish_seen_subcommand_from mcp lsp hook" -f
"#
    )
}

fn powershell(rules: &[&str]) -> String {
    let quoted_rules: Vec<String> = rules.iter().map(|rule| format!("'{rule}'")).collect();
    let rules = quoted_rules.join(", ");
    let quoted_profiles: Vec<String> = Profile::NAMES
        .iter()
        .map(|profile| format!("'{profile}'"))
        .collect();
    let profiles = quoted_profiles.join(", ");
    format!(
        r#"Register-ArgumentCompleter -Native -CommandName 'laconic-check' -ScriptBlock {{
    param($wordToComplete, $commandAst, $cursorPosition)

    $tokens = @($commandAst.CommandElements | ForEach-Object {{ $_.Extent.Text }})
    $subcommand = if ($tokens.Count -gt 1) {{ $tokens[1] }} else {{ '' }}
    $prev = if ($tokens.Count -gt 1) {{
        if ($wordToComplete -and $tokens[-1] -eq $wordToComplete) {{
            if ($tokens.Count -gt 2) {{ $tokens[-2] }} else {{ '' }}
        }} else {{
            $tokens[-1]
        }}
    }} else {{ '' }}

    $rules = @({rules})
    $profiles = @({profiles})

    $candidates = @()
    if ($tokens.Count -eq 1 -or ($tokens.Count -eq 2 -and $wordToComplete)) {{
        $candidates = @(
            [System.Management.Automation.CompletionResult]::new('lint', 'lint', [System.Management.Automation.CompletionResultType]::ParameterValue, 'Flag house-style candidates in Markdown prose')
            [System.Management.Automation.CompletionResult]::new('preserve', 'preserve', [System.Management.Automation.CompletionResultType]::ParameterValue, 'Check that an edit kept protected items')
            [System.Management.Automation.CompletionResult]::new('mcp', 'mcp', [System.Management.Automation.CompletionResultType]::ParameterValue, 'Serve lint and preserve as MCP tools')
            [System.Management.Automation.CompletionResult]::new('lsp', 'lsp', [System.Management.Automation.CompletionResultType]::ParameterValue, 'Run the language server')
            [System.Management.Automation.CompletionResult]::new('hook', 'hook', [System.Management.Automation.CompletionResultType]::ParameterValue, 'Run the Stop hook for Claude Code and Codex')
            [System.Management.Automation.CompletionResult]::new('completions', 'completions', [System.Management.Automation.CompletionResultType]::ParameterValue, 'Print a shell completion script')
            [System.Management.Automation.CompletionResult]::new('instructions', 'instructions', [System.Management.Automation.CompletionResultType]::ParameterValue, 'Print what a coding agent needs to use laconic-check')
            [System.Management.Automation.CompletionResult]::new('--help', '--help', [System.Management.Automation.CompletionResultType]::ParameterName, 'Print usage information')
            [System.Management.Automation.CompletionResult]::new('--version', '--version', [System.Management.Automation.CompletionResultType]::ParameterName, 'Print version information')
        )
    }} elseif ($subcommand -eq 'lint') {{
        if ($prev -eq '--disable' -or $prev -eq '--only') {{
            $candidates = $rules | ForEach-Object {{
                [System.Management.Automation.CompletionResult]::new($_, $_, [System.Management.Automation.CompletionResultType]::ParameterValue, 'Lint rule')
            }}
        }} elseif ($prev -eq '--format') {{
            $candidates = @('text', 'json', 'sarif') | ForEach-Object {{
                [System.Management.Automation.CompletionResult]::new($_, $_, [System.Management.Automation.CompletionResultType]::ParameterValue, 'Output format')
            }}
        }} elseif ($prev -eq '--profile') {{
            $candidates = $profiles | ForEach-Object {{
                [System.Management.Automation.CompletionResult]::new($_, $_, [System.Management.Automation.CompletionResultType]::ParameterValue, 'Register profile')
            }}
        }} elseif ($wordToComplete -like '-*' -or ($prev -ne '--config' -and $prev -ne '--max-findings')) {{
            $candidates = @(
                [System.Management.Automation.CompletionResult]::new('--format', '--format', [System.Management.Automation.CompletionResultType]::ParameterName, 'Output format (text, json or sarif)')
                [System.Management.Automation.CompletionResult]::new('--json', '--json', [System.Management.Automation.CompletionResultType]::ParameterName, 'Print one JSON document')
                [System.Management.Automation.CompletionResult]::new('--sarif', '--sarif', [System.Management.Automation.CompletionResultType]::ParameterName, 'Print a SARIF 2.1.0 log')
                [System.Management.Automation.CompletionResult]::new('--metrics', '--metrics', [System.Management.Automation.CompletionResultType]::ParameterName, 'Print distributional prose metrics')
                [System.Management.Automation.CompletionResult]::new('--profile', '--profile', [System.Management.Automation.CompletionResultType]::ParameterName, 'Apply a register preset')
                [System.Management.Automation.CompletionResult]::new('--disable', '--disable', [System.Management.Automation.CompletionResultType]::ParameterName, 'Drop findings for these rules')
                [System.Management.Automation.CompletionResult]::new('--only', '--only', [System.Management.Automation.CompletionResultType]::ParameterName, 'Keep findings only for these rules')
                [System.Management.Automation.CompletionResult]::new('--max-findings', '--max-findings', [System.Management.Automation.CompletionResultType]::ParameterName, 'Fail only on more than n failing findings')
                [System.Management.Automation.CompletionResult]::new('--warn', '--warn', [System.Management.Automation.CompletionResultType]::ParameterName, 'Never fail on findings')
                [System.Management.Automation.CompletionResult]::new('--commit-msg', '--commit-msg', [System.Management.Automation.CompletionResultType]::ParameterName, 'Read each file as a Git commit message')
                [System.Management.Automation.CompletionResult]::new('--config', '--config', [System.Management.Automation.CompletionResultType]::ParameterName, 'Read settings from this file')
            )
        }}
    }} elseif ($subcommand -eq 'preserve') {{
        $candidates = @(
            [System.Management.Automation.CompletionResult]::new('--strict', '--strict', [System.Management.Automation.CompletionResultType]::ParameterName, 'Fail on added items as well as dropped items')
            [System.Management.Automation.CompletionResult]::new('--loose', '--loose', [System.Management.Automation.CompletionResultType]::ParameterName, 'List lost hedge or frequency words to check')
        )
    }} elseif ($subcommand -eq 'completions') {{
        if ($tokens.Count -eq 2 -or ($tokens.Count -eq 3 -and $wordToComplete)) {{
            $candidates = @(
                [System.Management.Automation.CompletionResult]::new('bash', 'bash', [System.Management.Automation.CompletionResultType]::ParameterValue, 'Bash completion script')
                [System.Management.Automation.CompletionResult]::new('zsh', 'zsh', [System.Management.Automation.CompletionResultType]::ParameterValue, 'Zsh completion script')
                [System.Management.Automation.CompletionResult]::new('fish', 'fish', [System.Management.Automation.CompletionResultType]::ParameterValue, 'Fish completion script')
                [System.Management.Automation.CompletionResult]::new('powershell', 'powershell', [System.Management.Automation.CompletionResultType]::ParameterValue, 'PowerShell completion script')
            )
        }}
    }} elseif ($subcommand -eq 'instructions') {{
        if ($tokens.Count -eq 2 -or ($tokens.Count -eq 3 -and $wordToComplete)) {{
            $candidates = @('config', 'rules') | ForEach-Object {{
                [System.Management.Automation.CompletionResult]::new($_, $_, [System.Management.Automation.CompletionResultType]::ParameterValue, 'Topic')
            }}
        }}
    }}

    $candidates | Where-Object {{ $_.CompletionText -like "$wordToComplete*" }}
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_shells_and_rejects_others() {
        assert_eq!(Shell::parse("bash"), Some(Shell::Bash));
        assert_eq!(Shell::parse("zsh"), Some(Shell::Zsh));
        assert_eq!(Shell::parse("fish"), Some(Shell::Fish));
        assert_eq!(Shell::parse("powershell"), Some(Shell::PowerShell));
        for bad in ["", "sh", "tcsh", "BASH", "pwsh"] {
            assert_eq!(Shell::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn every_script_covers_subcommands_flags_shells_and_rules() {
        let subcommands = [
            "lint",
            "preserve",
            "mcp",
            "lsp",
            "hook",
            "completions",
            "instructions",
        ];
        let flags = [
            "format",
            "json",
            "sarif",
            "metrics",
            "profile",
            "disable",
            "only",
            "max-findings",
            "warn",
            "commit-msg",
            "config",
            "strict",
            "loose",
            "help",
            "version",
        ];
        let shells = ["bash", "zsh", "fish", "powershell"];

        for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell] {
            let out = script(shell);
            assert!(out.ends_with('\n'), "{shell:?}");
            for cmd in subcommands {
                assert!(out.contains(cmd), "{shell:?} missing subcommand {cmd}");
            }
            for flag in flags {
                let needle = if shell == Shell::Fish {
                    format!("-l {flag}")
                } else {
                    format!("--{flag}")
                };
                assert!(out.contains(&needle), "{shell:?} missing flag {needle}");
            }
            for target in shells {
                assert!(out.contains(target), "{shell:?} missing shell {target}");
            }
            for profile in Profile::NAMES {
                assert!(out.contains(profile), "{shell:?} missing profile {profile}");
            }
            for rule in lint::RULES {
                assert!(
                    out.contains(rule.name),
                    "{shell:?} missing rule {}",
                    rule.name
                );
            }
        }
    }
}
