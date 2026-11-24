// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Phrase tables for opt-in substance-heuristic and chat-filler rules.

/// Vague or dramatic magnitudes that need a denominator, period or source.
pub(super) const VAGUE_MAGNITUDES: &[&str] = &[
    "thousands of hours",
    "hundreds of hours",
    "millions of users",
    "millions of dollars",
    "billions of dollars",
    "countless hours",
    "orders of magnitude",
    "massive reduction",
    "massive improvement",
    "dramatic increase",
    "dramatic reduction",
    "exponential growth",
    "skyrocketed",
];

/// Suffixes after `<N>x ` or `<N>% ` that form an unanchored performance claim.
pub(super) const MAGNITUDE_SUFFIXES: &[&str] = &[
    "faster",
    "slower",
    "more",
    "less",
    "reduction",
    "increase",
    "improvement",
];

/// Superlatives and comparative performance claims that need attribution or a baseline.
pub(super) const SUPERLATIVES: &[&str] = &[
    "best-in-class",
    "best in class",
    "world-class",
    "world class",
    "industry-leading",
    "industry leading",
    "market-leading",
    "market leading",
    "state-of-the-art",
    "state of the art",
    "unmatched",
    "unrivalled",
    "unrivaled",
    "second to none",
    "gold standard",
    "the fastest",
    "the most reliable",
    "the most advanced",
    "significantly outperforms",
    "dramatically outperforms",
    "vastly outperforms",
    "substantially outperforms",
    "far outperforms",
    "outperforms all",
    "far superior",
    "vastly superior",
    "dramatically faster",
    "significantly faster",
    "vastly faster",
];

/// Appeals to unnamed evidence or authority.
pub(super) const UNNAMED_EVIDENCE: &[&str] = &[
    "studies show",
    "studies suggest",
    "studies have shown",
    "research shows",
    "research proves",
    "research suggests",
    "research indicates",
    "experts agree",
    "experts say",
    "experts warn",
    "analysts say",
    "analysts predict",
    "industry experts",
    "scientists say",
    "evidence shows",
    "evidence suggests",
    "data shows",
    "vast datasets",
    "vast global datasets",
    "vast global dataset",
    "multiple studies",
    "numerous studies",
    "widely recognised",
    "widely recognized",
    "it is widely known",
];

/// Prose-level AI residue phrases.
pub(super) const AI_RESIDUE: &[&str] = &[
    "as an ai",
    "as an artificial intelligence",
    "as a language model",
    "as an llm",
    "i cannot browse the internet",
    "my knowledge cutoff",
    "based on my training data",
    "as of my last update",
];

/// Bracketed drafting placeholder openers (`[insert ...]`, `[todo...]`, etc.).
pub(super) const PLACEHOLDER_OPENERS: &[&str] = &[
    "[insert ",
    "[todo",
    "[tbd",
    "[placeholder",
    "[your ",
    "[client name]",
    "[company name]",
    "[name here]",
    "[date here]",
    "[add ",
];

/// Architectural riddle filler common in verbose chat plans (`gates and seams`, etc.).
pub(super) const CHAT_FILLER: &[&str] = &[
    "gates and seams",
    "seams and gates",
    "clean seam",
    "clean seams",
    "natural seam",
    "natural seams",
    "architectural seam",
    "architectural seams",
    "boundary seam",
    "boundary seams",
    "extension seam",
    "extension seams",
    "wiring seam",
    "wiring seams",
    "verification gate",
    "verification gates",
    "readiness gate",
    "readiness gates",
    "approval gate",
    "approval gates",
    "quality gate",
    "quality gates",
    "promotion gate",
    "promotion gates",
    "load-bearing",
    "moving pieces",
    "hardening pass",
];
