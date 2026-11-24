// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Register presets for `laconic-check lint --profile <name>`.
//!
//! Each profile adjusts rule enablement and severity for its writing register without changing
//! default `lint` behaviour when no profile is selected.

use crate::lint::RuleId;

/// Writing register presets accepted by `--profile`, `laconic.toml` and `.laconic-check`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Email,
    Report,
    Docs,
    Chat,
    Essay,
}

impl Profile {
    /// Every supported profile name, in the order shown in help and completions.
    pub const NAMES: &[&str] = &["email", "report", "docs", "chat", "essay"];

    /// Parses a profile name accepted by the CLI and project file.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "email" => Some(Self::Email),
            "report" => Some(Self::Report),
            "docs" => Some(Self::Docs),
            "chat" => Some(Self::Chat),
            "essay" => Some(Self::Essay),
            _ => None,
        }
    }

    /// The profile's CLI name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Report => "report",
            Self::Docs => "docs",
            Self::Chat => "chat",
            Self::Essay => "essay",
        }
    }

    /// True when this profile enables an opt-in rule such as `unglossed-term` or the
    /// substance-heuristic and chat-filler checks.
    pub fn enables(self, id: RuleId) -> bool {
        match self {
            Self::Email => matches!(id, RuleId::UnglossedTerm | RuleId::AiPlaceholder),
            Self::Report | Self::Essay => matches!(
                id,
                RuleId::UnglossedTerm
                    | RuleId::UnanchoredMagnitude
                    | RuleId::UnattributedSuperlative
                    | RuleId::UnnamedEvidence
                    | RuleId::AiPlaceholder
            ),
            Self::Docs => matches!(id, RuleId::UnattributedSuperlative | RuleId::AiPlaceholder),
            Self::Chat => matches!(id, RuleId::ChatFiller | RuleId::AiPlaceholder),
        }
    }

    /// True when this profile enables at least one opt-in rule.
    pub fn enables_opt_in(self) -> bool {
        true
    }

    /// True when this profile relaxes a default-on rule by disabling it.
    pub fn disables(self, id: RuleId) -> bool {
        match self {
            Self::Chat => matches!(
                id,
                RuleId::AcronymBeforeDefinition | RuleId::HyphenConsistency
            ),
            Self::Docs => matches!(id, RuleId::BoldLabel | RuleId::AcronymBeforeDefinition),
            Self::Email | Self::Report | Self::Essay => false,
        }
    }

    /// Effective advisory flag for `id` under this profile (`false` means the rule fails `lint`).
    pub fn is_advisory(self, id: RuleId) -> bool {
        match (self, id) {
            (
                Self::Chat,
                RuleId::BoldLabel
                | RuleId::StackedHedge
                | RuleId::ClauseDashes
                | RuleId::ChatFiller,
            )
            | (Self::Email, RuleId::StackedHedge | RuleId::BoldLabel)
            | (
                Self::Report,
                RuleId::HyphenConsistency | RuleId::AcronymBeforeDefinition | RuleId::StackedHedge,
            )
            | (Self::Essay, RuleId::ClauseDashes | RuleId::BoldLabel | RuleId::StackedHedge) => {
                false
            }
            (Self::Docs, RuleId::NumberWord) => true,
            _ => id.rule().advisory,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_supported_profile_and_rejects_others() {
        for (name, expected) in [
            ("email", Profile::Email),
            ("report", Profile::Report),
            ("docs", Profile::Docs),
            ("chat", Profile::Chat),
            ("essay", Profile::Essay),
        ] {
            assert_eq!(Profile::parse(name), Some(expected));
            assert_eq!(expected.name(), name);
            assert!(Profile::NAMES.contains(&name));
        }
        for bad in ["", "memo", "DOCS", "blog"] {
            assert_eq!(Profile::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn profiles_tune_enablement_and_severity_per_register() {
        assert!(Profile::Email.enables(RuleId::UnglossedTerm));
        assert!(Profile::Email.enables(RuleId::AiPlaceholder));
        assert!(!Profile::Email.enables(RuleId::UnanchoredMagnitude));
        assert!(Profile::Report.enables(RuleId::UnglossedTerm));
        assert!(Profile::Report.enables(RuleId::UnanchoredMagnitude));
        assert!(Profile::Report.enables(RuleId::UnattributedSuperlative));
        assert!(Profile::Report.enables(RuleId::UnnamedEvidence));
        assert!(Profile::Report.enables(RuleId::AiPlaceholder));
        assert!(!Profile::Report.enables(RuleId::ChatFiller));
        assert!(Profile::Essay.enables(RuleId::UnglossedTerm));
        assert!(Profile::Essay.enables(RuleId::UnanchoredMagnitude));
        assert!(!Profile::Docs.enables(RuleId::UnglossedTerm));
        assert!(Profile::Docs.enables(RuleId::UnattributedSuperlative));
        assert!(Profile::Docs.enables(RuleId::AiPlaceholder));
        assert!(!Profile::Chat.enables(RuleId::UnglossedTerm));
        assert!(Profile::Chat.enables(RuleId::ChatFiller));
        assert!(Profile::Chat.enables(RuleId::AiPlaceholder));

        for p in [
            Profile::Email,
            Profile::Report,
            Profile::Docs,
            Profile::Chat,
            Profile::Essay,
        ] {
            assert!(p.enables_opt_in());
        }

        assert!(Profile::Chat.disables(RuleId::AcronymBeforeDefinition));
        assert!(Profile::Chat.disables(RuleId::HyphenConsistency));
        assert!(Profile::Docs.disables(RuleId::BoldLabel));
        assert!(Profile::Docs.disables(RuleId::AcronymBeforeDefinition));
        assert!(!Profile::Email.disables(RuleId::BoldLabel));
        assert!(!Profile::Report.disables(RuleId::BoldLabel));
        assert!(!Profile::Essay.disables(RuleId::BoldLabel));

        assert!(!Profile::Chat.is_advisory(RuleId::BoldLabel));
        assert!(!Profile::Chat.is_advisory(RuleId::StackedHedge));
        assert!(!Profile::Chat.is_advisory(RuleId::ClauseDashes));
        assert!(!Profile::Chat.is_advisory(RuleId::ChatFiller));
        assert!(Profile::Docs.is_advisory(RuleId::NumberWord));
        assert!(!Profile::Email.is_advisory(RuleId::StackedHedge));
        assert!(!Profile::Email.is_advisory(RuleId::BoldLabel));
        assert!(!Profile::Report.is_advisory(RuleId::HyphenConsistency));
        assert!(!Profile::Report.is_advisory(RuleId::AcronymBeforeDefinition));
        assert!(!Profile::Report.is_advisory(RuleId::StackedHedge));
        assert!(!Profile::Essay.is_advisory(RuleId::ClauseDashes));
        assert!(!Profile::Essay.is_advisory(RuleId::BoldLabel));
        assert!(!Profile::Essay.is_advisory(RuleId::StackedHedge));
        assert!(!Profile::Report.is_advisory(RuleId::UsSpelling));
        assert!(Profile::Report.is_advisory(RuleId::UnglossedTerm));
        assert!(Profile::Report.is_advisory(RuleId::UnanchoredMagnitude));
    }
}
