// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use crate::lint::{RuleId, lint, lint_all};

fn ref_findings(text: &str) -> Vec<(usize, usize, String)> {
    lint_all(text)
        .into_iter()
        .filter(|f| f.rule == RuleId::ReferenceShape)
        .map(|f| (f.line, f.column, f.message))
        .collect()
}

#[test]
fn valid_isbns_and_non_isbn_prose_pass() {
    let text = concat!(
        "Valid ISBN-10: 0-306-40615-2 and ISBN-10 0-8044-2957-X and isbn 080442957x.\n\n",
        "Valid ISBN-13: 978-0-306-40615-7 and ISBN 13: 979-10-325-0123-8 and ISBN:9780306406157.\n\n",
        "Productivity Commission: ISBN 978-1-74037-738-6 and ISBN 978-1-74037-739-3.\n\n",
        "Space separated with year: ISBN 978 0 306 40615 7 2026, ISBN 979 10 325 0123 8 2026 and ISBN 0 306 40615 2 2026.\n\n",
        "Hyphenated with trailing text: ISBN 978-0-306-40615-7 Xerox and ISBN 0-306-40615-2-.\n\n",
        "Space separated with word starting in X: ISBN 978 0 306 40615 7 Xerox.\n\n",
        "Bare numbers: 978-0-306-40615-8 and 0-306-40615-3 without a label.\n\n",
        "Not identifiers: ISBN 10 format, ISBN 13, ISBNs, myISBN: 978-0-306-40615-8, ISBN 2026, café.\n\n",
        "Ends with ISBN 10\n\n",
        "Ends with ISBN:\n",
    );
    assert_eq!(ref_findings(text), []);
    assert_eq!(lint(text), []);
}

#[test]
fn invalid_isbns_report_specific_reasons() {
    let cases = [
        ("See ISBN 0-306-40615-3.", "ISBN-10 checksum fails"),
        ("See ISBN 0-X06-40615-2.", "ISBN-10 checksum fails"),
        ("See ISBN 1000000000.", "ISBN-10 checksum fails"),
        ("See ISBN 978-0-306-40615-8.", "ISBN-13 checksum fails"),
        ("See ISBN 978-0-306-40615-X.", "ISBN-13 checksum fails"),
        (
            "See ISBN 977-0-306-40615-0.",
            "ISBN-13 must start with 978 or 979",
        ),
        (
            "See ISBN-13: 0-306-40615-2.",
            "ISBN-13 label has 10 characters; expected 13",
        ),
        (
            "See ISBN-10: 978-0-306-40615-7.",
            "ISBN-10 label has 13 characters; expected 10",
        ),
        (
            "See ISBN 978-0-306-40615.",
            "ISBN has 12 characters; expected 10 or 13",
        ),
        (
            "See ISBN: 978-0-306-40615-78.",
            "ISBN has 14 characters; expected 10 or 13",
        ),
        (
            "See ISBN 10: 12345678.",
            "ISBN has 8 characters; expected 10 or 13",
        ),
    ];
    for (input, want) in cases {
        let got = ref_findings(input);
        assert_eq!(got.len(), 1, "{input}: {got:?}");
        assert!(got[0].2.contains(want), "{input}: got {:?}", got[0]);
        // Opt-in: default lint produces no reference-shape finding.
        assert!(
            lint(input).iter().all(|f| f.rule != RuleId::ReferenceShape),
            "{input}"
        );
    }
}

#[test]
fn valid_arxiv_and_non_arxiv_prose_pass() {
    let text = concat!(
        "Valid modern: arXiv:0704.0001, arXiv 1412.9999v1, arXiv: 1501.00001V12.\n\n",
        "Valid URLs: https://arxiv.org/abs/2301.12345 and https://www.arxiv.org/pdf/2301.12345v2.pdf.\n\n",
        "Old style untouched: arXiv:hep-th/9901001 and https://arxiv.org/abs/math.GT/0309136.\n\n",
        "Ordinary prose: arXiv 2.0, arXiv 2024, arxiv.org/list/cs.CL/recent, arXivs, myarXiv:2313.12345.\n\n",
        "Bare number without label: 2313.12345 and 0612.1234.\n",
    );
    assert_eq!(ref_findings(text), []);
}

#[test]
fn invalid_arxiv_identifiers_report_specific_reasons() {
    let cases = [
        (
            "See arXiv:23.12345.",
            "arXiv YYMM prefix must have 4 digits",
        ),
        (
            "See arXiv:231.12345.",
            "arXiv YYMM prefix must have 4 digits",
        ),
        (
            "See arXiv:202301.12345.",
            "arXiv YYMM prefix must have 4 digits",
        ),
        ("See arXiv:2300.12345.", "arXiv month '00' is not 01 to 12"),
        ("See arXiv:2313.12345.", "arXiv month '13' is not 01 to 12"),
        (
            "See arXiv:0703.1234.",
            "arXiv YYMM.NNNN identifiers start at 0704 (April 2007)",
        ),
        (
            "See arXiv:0612.1234.",
            "arXiv YYMM.NNNN identifiers start at 0704 (April 2007)",
        ),
        (
            "See https://arxiv.org/pdf/1412.12345.pdf.",
            "arXiv identifiers before 1501 use a 4-digit sequence number",
        ),
        (
            "See https://arxiv.org/abs/1501.1234.",
            "arXiv identifiers from 1501 onward use a 5-digit sequence number",
        ),
        (
            "See arXiv:2301.12.",
            "arXiv identifiers from 1501 onward use a 5-digit sequence number",
        ),
        ("See arXiv 2301.00000.", "arXiv sequence numbers start at 1"),
        (
            "See arXiv:2301.12345v0.",
            "arXiv version numbers start at v1",
        ),
    ];
    for (input, want) in cases {
        let got = ref_findings(input);
        assert_eq!(got.len(), 1, "{input}: {got:?}");
        assert!(got[0].2.contains(want), "{input}: got {:?}", got[0]);
    }
}

#[test]
fn skips_code_comments_and_blockquotes_but_checks_alerts() {
    let text = concat!(
        "Inline `ISBN 978-0-306-40615-8` and `arXiv:2313.12345` pass, then ISBN 978-0-306-40615-8 flags.\n\n",
        "```text\nISBN 978-0-306-40615-8\narXiv:2313.12345\n```\n\n",
        "    ISBN 978-0-306-40615-8\n\n",
        "<!-- ISBN 978-0-306-40615-8 -->\n\n",
        "<!--\narXiv:2313.12345\n-->\n\n",
        "> Quoted ISBN 978-0-306-40615-8 and arXiv:2313.12345.\n\n",
        "> [!NOTE]\n> Alert flags arXiv:2313.12345.\n",
    );
    let got = ref_findings(text);
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(got[0].0, 1);
    assert!(got[0].2.contains("978-0-306-40615-8"), "{got:?}");
    assert_eq!(got[1].0, 19);
    assert!(got[1].2.contains("2313.12345"), "{got:?}");
}

#[test]
fn reports_exact_columns_across_wrapped_lines() {
    let text = "Line one cites a book,\n  ISBN 978-0-306-40615-8 here.\n";
    let got = ref_findings(text);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!((got[0].0, got[0].1), (2, 3));
}
