// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use crate::lint::{RuleId, lint, lint_all};

fn rules_of(text: &str) -> Vec<&'static str> {
    lint_all(text)
        .into_iter()
        .filter_map(|f| f.rule.house().map(RuleId::name))
        .collect()
}

#[test]
fn unanchored_magnitudes_flag_only_without_anchors() {
    assert!(lint("It saves thousands of hours and runs 10x faster.").is_empty());
    assert_eq!(
        rules_of("It saves thousands of hours."),
        ["unanchored-magnitude"]
    );
    assert_eq!(
        rules_of("The engine runs 10x faster."),
        ["unanchored-magnitude"]
    );
    assert_eq!(
        rules_of("Users see a 90% reduction."),
        ["unanchored-magnitude"]
    );
    assert_eq!(
        rules_of("Users see a 2.5x improvement."),
        ["unanchored-magnitude"]
    );
    for clean in [
        "It saves thousands of hours per year.",
        "It saves thousands of hours according to the audit.",
        "The engine runs 10x faster in our [benchmark](https://example.com/bench).",
        "The engine runs 10x faster (2026).",
        "The engine runs 10x faster [1].",
        "Users saw a 90% reduction from 100 ms to 10 ms.",
        "Users saw a 90% reduction across 12 teams.",
        "Users saw a 90% reduction in 2025.",
        "The tool cut latency by 10x in a benchmark.",
        "The 10x budget item was approved.",
    ] {
        assert!(
            !rules_of(clean).contains(&"unanchored-magnitude"),
            "unexpected flag in {clean:?}: {:?}",
            rules_of(clean)
        );
    }
}

#[test]
fn unattributed_superlatives_flag_only_without_attribution() {
    assert_eq!(
        rules_of("This is a best-in-class parser."),
        ["unattributed-superlative"]
    );
    assert_eq!(
        rules_of("The engine significantly outperforms rivals."),
        ["unattributed-superlative"]
    );
    for clean in [
        "The engine significantly outperforms the baseline compared with v1.",
        "This is a best-in-class parser according to the audit.",
        "The engine significantly outperforms rivals [see https://example.com].",
        "The engine significantly outperforms rivals (2026).",
    ] {
        assert!(
            !rules_of(clean).contains(&"unattributed-superlative"),
            "unexpected flag in {clean:?}: {:?}",
            rules_of(clean)
        );
    }
}

#[test]
fn unnamed_evidence_flags_only_without_named_source() {
    assert_eq!(
        rules_of("Studies show faster builds."),
        ["unnamed-evidence"]
    );
    assert_eq!(
        rules_of("Experts agree the layout works."),
        ["unnamed-evidence"]
    );
    assert_eq!(rules_of("Research proves the claim."), ["unnamed-evidence"]);
    assert_eq!(
        rules_of("Models train on vast datasets."),
        ["unnamed-evidence"]
    );
    for clean in [
        "Studies show faster builds [1].",
        "Studies show faster builds in [the report](https://example.com/r).",
        "Studies show faster builds by CSIRO.",
        "Research proves the claim from the University of Sydney.",
        "Experts agree at Monash.",
        "Studies show faster builds according to the bureau.",
        "Studies show faster builds from local teams and by CSIRO.",
    ] {
        assert!(
            !rules_of(clean).contains(&"unnamed-evidence"),
            "unexpected flag in {clean:?}: {:?}",
            rules_of(clean)
        );
    }
}

#[test]
fn ai_placeholder_catches_residue_brackets_and_todo_tbd() {
    assert_eq!(
        rules_of("As an AI, I cannot verify this."),
        ["ai-placeholder"]
    );
    assert_eq!(
        rules_of("Add [insert metric here] before release."),
        ["ai-placeholder"]
    );
    assert_eq!(rules_of("The final total is TBD."), ["ai-placeholder"]);
    assert_eq!(
        rules_of("Finish the checklist TODO before sending."),
        ["ai-placeholder"]
    );
    assert_eq!(rules_of("See [TODO]."), ["ai-placeholder"]);
    assert_eq!(rules_of("See [TODO: add details]."), ["ai-placeholder"]);
    assert_eq!(rules_of("See [TBD]."), ["ai-placeholder"]);
    assert_eq!(rules_of("See [TBD - owner]."), ["ai-placeholder"]);
    assert_eq!(rules_of("See [insert TODO here]."), ["ai-placeholder"]);
    assert_eq!(rules_of("See [TODO and finish."), ["ai-placeholder"]);
    assert_eq!(rules_of("See [see TODO]."), ["ai-placeholder"]);
    assert!(!rules_of("Run `TODO` and `[insert x]` in code.").contains(&"ai-placeholder"));
    assert!(!rules_of("Check the todo list tomorrow.").contains(&"ai-placeholder"));
    for clean in [
        "Click [Add details](https://example.com) to continue.",
        "Open [Your profile](https://example.com/profile) to edit settings.",
        "Read [the house color guide](https://example.com/guide).",
    ] {
        assert!(
            !rules_of(clean).contains(&"ai-placeholder"),
            "unexpected flag in {clean:?}: {:?}",
            rules_of(clean)
        );
    }
}

#[test]
fn chat_filler_catches_gates_and_seams_bloat() {
    assert_eq!(
        rules_of("Define clean gates and seams before coding."),
        ["chat-filler"]
    );
    assert_eq!(
        rules_of("Add a verification gate at the natural seam."),
        ["chat-filler", "chat-filler"]
    );
    assert!(lint("Define clean gates and seams before coding.").is_empty());
}

#[test]
fn sentence_isolation_keeps_unanchored_claim_in_second_sentence_flagged() {
    let text =
        "See https://example.com for context. It saves thousands of hours! Also 1.5x faster.";
    let found = rules_of(text);
    assert_eq!(found, ["unanchored-magnitude", "unanchored-magnitude"]);
}

#[test]
fn partial_words_unclosed_brackets_and_unpaired_from_behave_cleanly() {
    assert_eq!(
        rules_of("From home, it saves thousands of hours."),
        ["unanchored-magnitude"]
    );
    assert!(!rules_of("It skyrocketedx overnight.").contains(&"unanchored-magnitude"));
    assert!(!rules_of("Has an aim in sight.").contains(&"ai-placeholder"));
    assert!(!rules_of("See [insert unclosed text.").contains(&"ai-placeholder"));
    assert!(!rules_of("The load-bearingx frame held.").contains(&"chat-filler"));
}

#[test]
fn unicode_casefold_length_change_and_later_prepositions_still_anchor() {
    // 'İ' (2 bytes) -> 'i' (1 byte) and 'K' (3 bytes) -> 'k' (1 byte) must not shift `orig` offsets.
    assert!(
        !rules_of("\u{0130}stanbul studies show gains by CSIRO.").contains(&"unnamed-evidence")
    );
    assert!(
        !rules_of("\u{212a}elvin research shows gains from the CSIRO.")
            .contains(&"unnamed-evidence")
    );
    // Later `of <digit>` after an earlier non-numeric `of <word>` still anchors the magnitude.
    assert!(
        !rules_of("The audit of operations found a 90% reduction of 200 hours.")
            .contains(&"unanchored-magnitude")
    );
    assert!(
        !rules_of("It saves thousands of hours of 12 engineers.").contains(&"unanchored-magnitude")
    );
}

#[test]
fn substrings_ending_in_per_or_of_do_not_anchor_magnitudes() {
    for unanchored in [
        "Our developer saves thousands of hours.",
        "The paper claims 10x faster builds.",
        "A cheaper option gives a 90% reduction.",
        "The waterproof coat saves thousands of hours.",
    ] {
        assert!(
            rules_of(unanchored).contains(&"unanchored-magnitude"),
            "expected flag in {unanchored:?}: {:?}",
            rules_of(unanchored)
        );
    }
}

#[test]
fn markdown_links_labelled_todo_or_tbd_do_not_trigger_ai_placeholder() {
    assert!(
        !rules_of("See [TODO](https://example.com) for the board.").contains(&"ai-placeholder")
    );
    assert!(
        !rules_of("Open [TBD](https://example.com) and [Add details](https://example.com).")
            .contains(&"ai-placeholder")
    );
    let mixed = rules_of("See [TODO](https://example.com) and TODO later.");
    assert_eq!(
        mixed.iter().filter(|&&r| r == "ai-placeholder").count(),
        1,
        "only standalone prose TODO should flag: {mixed:?}"
    );
    let mixed_tbd = rules_of("See [TBD](https://example.com) and TBD next week.");
    assert_eq!(
        mixed_tbd.iter().filter(|&&r| r == "ai-placeholder").count(),
        1,
        "only standalone prose TBD should flag: {mixed_tbd:?}"
    );
    let count = |text: &str| {
        rules_of(text)
            .iter()
            .filter(|&&r| r == "ai-placeholder")
            .count()
    };
    assert_eq!(count("See ![TBD](img.png) above."), 0);
    assert_eq!(count("See [TODO](url \"Title\") later."), 0);
    // Escaped brackets render as literal text, so no link hides the placeholder.
    assert_eq!(count(r"See \[TODO\](https://example.com) later."), 1);
    assert_eq!(count(r"See \[TODO](https://example.com) later."), 1);
    assert_eq!(count(r"See [TODO\](https://example.com) later."), 1);
    // An even run of backslashes escapes itself, so the bracket still opens a link.
    assert_eq!(count(r"See \\[TODO](https://example.com) later."), 0);
    // A destination with a space and no title is not a link.
    assert_eq!(count("See [TODO](see below) later."), 1);
}
