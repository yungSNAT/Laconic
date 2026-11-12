// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Protected items an edit must keep: numbers, quantities, dated months, inline code,
//! URLs, quotations and the words that carry negation, modality, certainty, condition, order and
//! scope. Reports what the original had and the edit lost, and what the edit added. A
//! clean result does not prove the meaning survived; it only rules out these losses. An
//! addition is a reason to check the edit invented nothing, not proof that it did.

use std::collections::BTreeMap;

const MEANING_WORDS: &[&str] = &[
    "not",
    "never",
    "no",
    "none",
    "nothing",
    "nor",
    "neither",
    "without",
    "cannot",
    "must",
    "may",
    "only",
    "except",
    "unless",
    "some",
    "all",
    "any",
    "every",
    "most",
    "should",
    "might",
    "can",
    "could",
    "always",
    "will",
    "shall",
    "if",
    "until",
    "because",
    "before",
    "after",
    "likely",
    "probably",
    "possibly",
    "perhaps",
    "usually",
    "generally",
    "often",
    "sometimes",
    "approximately",
    "roughly",
];

/// Bounds and comparisons; `N` matches a number, so 'over the weekend' is not a bound.
const MEANING_PHRASES: &[&str] = &[
    "up to",
    "at least",
    "at most",
    "more than",
    "less than",
    "fewer than",
    "over N",
    "under N",
    "about N",
    "nearly N",
    "almost N",
    "subject to",
];

/// Attached or following words that give a number its size or unit, with the form a
/// quantity reports. Upper-case `M` is million and lower-case `m` is metres, `Mb` is
/// bits where `MB` is bytes, and `g` and `L` stand alone only in those cases, so these
/// match case-sensitively; every other spelling matches in any case.
const QUANTITY_WORDS: &[(&[&str], &str)] = &[
    (&["k", "thousand"], "thousand"),
    (&["mn", "million", "millions"], "million"),
    (&["b", "bn", "billion", "billions"], "billion"),
    (&["trillion", "trillions"], "trillion"),
    (
        &[
            "km",
            "kms",
            "kilometre",
            "kilometres",
            "kilometer",
            "kilometers",
        ],
        "km",
    ),
    (&["metre", "metres", "meter", "meters"], "m"),
    (&["cm", "centimetre", "centimetres"], "cm"),
    (&["mm", "millimetre", "millimetres"], "mm"),
    (&["mile", "miles"], "mile"),
    (&["kg", "kgs", "kilogram", "kilograms"], "kg"),
    (&["tonne", "tonnes"], "tonne"),
    (&["lb", "lbs", "pound", "pounds"], "lb"),
    (&["mg", "milligram", "milligrams"], "mg"),
    (&["gram", "grams"], "g"),
    (&["ml", "millilitre", "millilitres", "milliliter", "milliliters"], "mL"),
    (&["litre", "litres", "liter", "liters"], "L"),
    (&["°c", "celsius"], "°C"),
    (&["°f", "fahrenheit"], "°F"),
    (&["hour", "hours", "hr", "hrs"], "hour"),
    (&["minute", "minutes", "min", "mins"], "minute"),
    (&["second", "seconds", "sec", "secs"], "second"),
    (&["ms", "millisecond", "milliseconds"], "ms"),
    (&["day", "days"], "day"),
    (&["week", "weeks"], "week"),
    (&["month", "months"], "month"),
    (&["year", "years"], "year"),
    (&["kb", "kilobyte", "kilobytes"], "KB"),
    (&["mb", "megabyte", "megabytes"], "MB"),
    (&["gb", "gigabyte", "gigabytes"], "GB"),
    (&["tb", "terabyte", "terabytes"], "TB"),
    (&["kilobit", "kilobits"], "Kb"),
    (&["megabit", "megabits"], "Mb"),
    (&["gigabit", "gigabits"], "Gb"),
    (&["terabit", "terabits"], "Tb"),
];

const MONTHS: &[(&[&str], &str)] = &[
    (&["January", "Jan"], "January"),
    (&["February", "Feb"], "February"),
    (&["March", "Mar"], "March"),
    (&["April", "Apr"], "April"),
    (&["May"], "May"),
    (&["June", "Jun"], "June"),
    (&["July", "Jul"], "July"),
    (&["August", "Aug"], "August"),
    (&["September", "Sept", "Sep"], "September"),
    (&["October", "Oct"], "October"),
    (&["November", "Nov"], "November"),
    (&["December", "Dec"], "December"),
];

/// Capitalised words that are not part of a name, so 'May I' and 'The Board' read right.
const NOT_NAME_WORDS: &[&str] = &[
    "A", "An", "And", "As", "At", "But", "By", "For", "From", "He", "Her", "His", "I", "If", "In",
    "It", "Its", "My", "No", "Not", "Of", "On", "Or", "Our", "She", "So", "The", "Their", "These",
    "They", "This", "Those", "To", "We", "When", "With", "You", "Your",
];

/// Number words count as their numerals, so 'three' to '3' is not a change.
/// 'one' is left out because it is also a pronoun. shortcut: compounds such as
/// 'twenty-five' read as 20 and 5; spell out a parser if edits hit them.
const NUMBER_WORDS: &[(&str, &str)] = &[
    ("two", "2"),
    ("three", "3"),
    ("four", "4"),
    ("five", "5"),
    ("six", "6"),
    ("seven", "7"),
    ("eight", "8"),
    ("nine", "9"),
    ("ten", "10"),
    ("eleven", "11"),
    ("twelve", "12"),
    ("thirteen", "13"),
    ("fourteen", "14"),
    ("fifteen", "15"),
    ("sixteen", "16"),
    ("seventeen", "17"),
    ("eighteen", "18"),
    ("nineteen", "19"),
    ("twenty", "20"),
    ("thirty", "30"),
    ("forty", "40"),
    ("fifty", "50"),
    ("sixty", "60"),
    ("seventy", "70"),
    ("eighty", "80"),
    ("ninety", "90"),
];

/// Capitalised names in the original that the edit no longer contains. Advisory only:
/// capitals also mark titles, so a hit is a reason to look. Headings and bold labels are
/// skipped, and a run of more than 3 words reads as a title. shortcut: a lone 'Priya' is
/// not seen; catching it needs a sentence splitter.
pub fn names(original: &str, edited: &str) -> Vec<String> {
    let edited = prose(edited).replace('*', "");
    dedup(name_runs(original))
        .into_iter()
        .filter(|name| !edited.contains(name))
        .collect()
}

/// Runs of 2 or 3 capitalised words outside headings and bold labels.
fn name_runs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let is_label = |line: &&str| {
        let line = line.trim_start();
        line.starts_with('#') || line.starts_with("**") || line.starts_with("- **")
    };
    for line in text.lines().filter(|line| !is_label(line)) {
        let line = prose(line);
        let mut run: Vec<&str> = Vec::new();
        let mut tokens = line.split_whitespace().peekable();
        while let Some(token) = tokens.next() {
            let word = token.trim_matches(|c: char| !c.is_alphanumeric());
            let capital = word.starts_with(char::is_uppercase) && !NOT_NAME_WORDS.contains(&word);
            if capital {
                run.push(word);
            }
            let closes = token.ends_with(|c: char| !c.is_alphanumeric() && c != '*');
            if !capital || closes || tokens.peek().is_none() {
                if (2..=3).contains(&run.len()) {
                    out.push(run.join(" "));
                }
                run.clear();
            }
        }
    }
    out
}

/// What `preserve` reports, as the CLI prints it.
pub struct Report {
    /// A line for each item the edit dropped, then each it added and each name to check,
    /// after a clean line when nothing was dropped, and last the change in length.
    pub lines: Vec<String>,
    /// The edit dropped a protected item.
    pub dropped: bool,
    /// The edit added a protected item, which fails only under `--strict`.
    pub added: bool,
}

pub fn report(original: &str, edited: &str) -> Report {
    let missing = missing(original, edited);
    let added = added(original, edited);
    let mut lines = Vec::new();
    if missing.is_empty() {
        lines.push("No tracked items dropped; meaning not checked.".to_string());
    }
    for (kind, items) in &missing {
        lines.extend(items.iter().map(|item| format!("dropped {kind}: {item}")));
    }
    for (kind, items) in &added {
        lines.extend(items.iter().map(|item| format!("added {kind}: {item}")));
    }
    lines.extend(
        names(original, edited)
            .into_iter()
            .map(|name| format!("check name: {name}")),
    );
    // A name only the edit has may be invented; like a lost name, it never fails.
    lines.extend(
        names(edited, original)
            .into_iter()
            .map(|name| format!("check added name: {name}")),
    );
    lines.push(length(original, edited));
    Report {
        lines,
        dropped: !missing.is_empty(),
        added: !added.is_empty(),
    }
}

/// The edit's length in whitespace-separated words against the original's, as a rounded
/// share of it.
fn length(original: &str, edited: &str) -> String {
    let (before, after) = (
        original.split_whitespace().count(),
        edited.split_whitespace().count(),
    );
    if before == 0 {
        return format!("length: 0 to {after} words");
    }
    let share = share(before, after);
    format!("length: {before} to {after} words ({share}% of the original)")
}

/// `after` as a percentage of a non-zero `before`, rounded half up; u128 keeps any count
/// from overflowing, including on 32-bit targets.
fn share(before: usize, after: usize) -> u128 {
    let (before, after) = (before as u128, after as u128);
    (after * 100 + before / 2) / before
}

/// Items the original had and the edit lost.
pub fn missing(original: &str, edited: &str) -> BTreeMap<&'static str, Vec<String>> {
    only_in(original, edited, |had, has| (had, has))
}

/// Items the edit has and the original did not, such as an invented figure or `must`.
/// Numbers, quantities, months, code, URLs and quotations count as added only when the value is
/// new, not when an existing one repeats; a meaning word counts when it appears more often.
pub fn added(original: &str, edited: &str) -> BTreeMap<&'static str, Vec<String>> {
    only_in(edited, original, |had, has| (has, had))
}

/// Items in `from` that `to` lacks; `order` maps the 2 meaning-word counts to
/// (before, after) for the report.
fn only_in(
    from: &str,
    to: &str,
    order: fn(usize, usize) -> (usize, usize),
) -> BTreeMap<&'static str, Vec<String>> {
    let mut out: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    // Unsigned values, as a quantity carries no sign or currency.
    let values_to: Vec<String> = numbers(to)
        .iter()
        .map(|n| {
            number_value(n)
                .trim_start_matches(|c: char| !c.is_ascii_digit())
                .to_string()
        })
        .collect();
    for (kind, extract) in [
        ("number", numbers as fn(&str) -> Vec<String>),
        ("quantity", quantities),
        ("month", months),
        ("code", code_spans),
        ("url", urls),
        ("quotation", quotations),
    ] {
        let key = |x: &String| {
            if kind == "number" {
                number_value(x)
            } else {
                x.clone()
            }
        };
        let kept: Vec<String> = extract(to).iter().map(key).collect();
        let lost: Vec<String> = dedup(extract(from))
            .into_iter()
            .filter(|x| !kept.contains(&key(x)))
            // A lost number already reports itself; a quantity adds only a changed size or unit.
            .filter(|x| {
                kind != "quantity"
                    || x.split(' ')
                        .next()
                        .is_some_and(|n| values_to.iter().any(|v| v == n))
            })
            .collect();
        if !lost.is_empty() {
            out.insert(kind, lost);
        }
    }
    let (in_from, in_to) = (meaning_tokens(from), meaning_tokens(to));
    let lost: Vec<String> = MEANING_WORDS
        .iter()
        .chain(MEANING_PHRASES)
        .filter_map(|w| {
            let (mut f, mut t) = (count(&in_from, w), count(&in_to, w));
            if matches!(*w, "may" | "will") {
                f = f.saturating_sub(not_modal(from, w));
                t = t.saturating_sub(not_modal(to, w));
            }
            let (b, a) = order(f, t);
            (t < f).then(|| format!("'{w}' ({b} before, {a} after)"))
        })
        .collect();
    if !lost.is_empty() {
        out.insert("meaning word", lost);
    }
    out
}

/// Words for meaning counts, with hedges on a number read as one: 'approximately 50' to
/// 'about 50' is not a change, and every negative contraction ('didn't', 'wasn't') reads
/// as 'not', so 'do not' to 'don't' is not one either.
fn meaning_tokens(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = words(&prose(text))
        .into_iter()
        .map(|w| if w.ends_with("n't") { "not".into() } else { w })
        .collect();
    for i in 1..tokens.len() {
        if matches!(
            tokens[i - 1].as_str(),
            "approximately" | "roughly" | "around"
        ) && is_number(&tokens[i])
        {
            tokens[i - 1] = "about".to_string();
        }
    }
    tokens
}

/// Uses of 'may' or 'will' that are a month or part of a name, such as 'Will Smith'.
/// shortcut: 'Will Priya sign?' reads as a name, so its modal is listed as a name to check
/// and not counted; telling them apart needs a parser.
fn not_modal(text: &str, word: &str) -> usize {
    let dated = months(text)
        .iter()
        .filter(|month| month.eq_ignore_ascii_case(word))
        .count();
    let named = name_runs(text)
        .iter()
        .filter(|name| name.split(' ').any(|part| part.eq_ignore_ascii_case(word)))
        .count();
    dated + named
}

fn dedup(mut v: Vec<String>) -> Vec<String> {
    let mut seen = Vec::new();
    v.retain(|x| {
        if seen.contains(x) {
            false
        } else {
            seen.push(x.clone());
            true
        }
    });
    v
}

/// A number's value, so a required edit such as 1000 to 1,000 is not a change. Only
/// commas that group thousands go; '1,2,3' stays a list. A sign or currency stays.
fn number_value(token: &str) -> String {
    let digits = token.find(|c: char| c.is_ascii_digit()).unwrap_or(0);
    let (prefix, token) = token.split_at(digits);
    let whole = token
        .split('.')
        .next()
        .unwrap_or(token)
        .trim_end_matches('%');
    let mut groups = whole.split(',');
    let first_fits = groups.next().is_some_and(|g| (1..=3).contains(&g.len()));
    if whole.contains(',') && first_fits && groups.all(|g| g.len() == 3) {
        format!("{prefix}{}", token.replace(',', ""))
    } else {
        format!("{prefix}{token}")
    }
}

/// The text without ordered list markers, which number the layout and not the content,
/// including a marker inside bold or strong emphasis ('**1. Plan**').
/// shortcut: a hard-wrapped line starting '12. ' reads as a marker, so markers stop at
/// 2 digits; a longer list reports its later markers as numbers.
fn without_list_markers(text: &str) -> String {
    text.lines()
        .map(|line| {
            let body = line.trim_start();
            let body = body
                .strip_prefix("**")
                .or_else(|| body.strip_prefix("__"))
                .unwrap_or(body);
            let digits = body.len() - body.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            let marker = (1..=2).contains(&digits)
                && [". ", ".\t", ") ", ")\t"]
                    .iter()
                    .any(|marker| body[digits..].starts_with(marker));
            if marker { &body[digits + 2..] } else { line }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether a percentage word follows, so '5 per cent' and '5%' are the same value.
fn starts_with_percent(text: &str) -> bool {
    let mut next = text.split_whitespace().map(str::to_lowercase);
    match next.next().as_deref() {
        Some(w) if w.trim_end_matches(['.', ',', ';', ':', ')']) == "percent" => true,
        Some("per") => next
            .next()
            .is_some_and(|w| w.trim_end_matches(['.', ',', ';', ':', ')']) == "cent"),
        _ => false,
    }
}

/// A number with the size or unit it carries, attached or as the next word: '$47.3M',
/// '150 km', 'three days'. Read from prose, so a value in code or a URL is not one.
fn quantities(text: &str) -> Vec<String> {
    let text = prose(text);
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut out = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        let body = token.trim_start_matches(|c: char| !c.is_alphanumeric());
        let end = body
            .find(|c: char| !(c.is_ascii_digit() || matches!(c, ',' | '.')))
            .unwrap_or(body.len());
        let (value, attached) = body.split_at(end);
        let (value, attached) = if value.is_empty() {
            let word = body
                .trim_end_matches(|c: char| !c.is_alphanumeric())
                .split('-')
                .next()
                .unwrap_or_default()
                .to_lowercase();
            match number_word(&word) {
                Some(n) => (n, &body[word.len()..]),
                None => continue,
            }
        } else {
            (value, attached)
        };
        let bare = value.trim_end_matches(['.', ',']);
        if bare.is_empty() {
            continue;
        }
        let unit = attached.trim_start_matches('-');
        let word = if !unit
            .trim_end_matches(|c: char| !c.is_alphanumeric())
            .is_empty()
        {
            unit
        } else if value.len() == bare.len() && attached.is_empty() {
            match tokens.get(i + 1) {
                Some(next) => next,
                None => continue,
            }
        } else {
            continue;
        };
        if let Some(unit) = quantity_word(word.trim_end_matches(|c: char| !c.is_alphanumeric())) {
            out.push(format!("{} {unit}", number_value(bare)));
        }
    }
    out
}

fn quantity_word(word: &str) -> Option<&'static str> {
    match word {
        "M" => return Some("million"),
        "m" => return Some("m"),
        "Kb" => return Some("Kb"),
        "Mb" => return Some("Mb"),
        "Gb" => return Some("Gb"),
        "Tb" => return Some("Tb"),
        "g" => return Some("g"),
        "L" => return Some("L"),
        _ => {}
    }
    let lower = word.to_lowercase();
    QUANTITY_WORDS
        .iter()
        .find(|(spellings, _)| spellings.contains(&lower.as_str()))
        .map(|(_, unit)| *unit)
}

/// Capitalised month names next to a number, so 'May I' and 'march on' do not count.
fn months(text: &str) -> Vec<String> {
    let text = prose(text);
    let tokens: Vec<&str> = text
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
        .collect();
    let numeric = |i: usize| {
        tokens
            .get(i)
            .is_some_and(|t| t.starts_with(|c: char| c.is_ascii_digit()))
    };
    tokens
        .iter()
        .enumerate()
        .filter(|(i, _)| numeric(i + 1) || (*i > 0 && numeric(i - 1)))
        .filter_map(|(_, t)| {
            MONTHS
                .iter()
                .find(|(spellings, _)| spellings.contains(t))
                .map(|(_, name)| name.to_string())
        })
        .collect()
}

fn numbers(text: &str) -> Vec<String> {
    let text = without_list_markers(text);
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let starts = chars[i].is_ascii_digit() && (i == 0 || !chars[i - 1].is_alphanumeric());
        if starts {
            let mut j = i;
            while j < chars.len()
                && (chars[j].is_ascii_digit() || matches!(chars[j], ',' | '.' | '%'))
            {
                j += 1;
            }
            let token: String = chars[i..j]
                .iter()
                .collect::<String>()
                .trim_end_matches(['.', ','])
                .to_string();
            let after: String = chars[j..chars.len().min(j + 12)].iter().collect();
            let percent = !token.ends_with('%') && starts_with_percent(&after);
            let token = if percent { token + "%" } else { token };
            let (start, mut negative, currency) = number_prefix(&chars, i);
            // Accounting brackets mark a negative amount: '(1,200)' or '($250)'. A bare
            // whole number in brackets, such as a note '(3)', is an aside, not an amount.
            let amount = !currency.is_empty() || token.contains([',', '.']);
            if start > 0 && chars[start - 1] == '(' && chars.get(j) == Some(&')') && amount {
                negative = true;
            }
            let sign = if negative { "-" } else { "" };
            out.push(format!("{sign}{currency}{token}"));
            i = j;
        } else {
            i += 1;
        }
    }
    let spelled = words(&prose(&text));
    out.extend(spelled.iter().enumerate().filter_map(|(i, w)| {
        let percent = match spelled.get(i + 1).map(String::as_str) {
            Some("percent") => true,
            Some("per") => spelled.get(i + 2).is_some_and(|w| w == "cent"),
            _ => false,
        };
        number_word(w).map(|n| {
            if percent {
                format!("{n}%")
            } else {
                n.to_string()
            }
        })
    }));
    out.extend(ordinals(&text));
    out
}

/// Ordinal words count as their numerals, so 'first' matches '1st' and a dropped 'first'
/// in 'first-time buyers' reports. 'second' is left out because it is also a unit of
/// time, and an ordinal before a comma ('First, ...') orders the text, not the facts.
const ORDINAL_WORDS: &[(&str, &str)] = &[
    ("first", "1"),
    ("third", "3"),
    ("fourth", "4"),
    ("fifth", "5"),
    ("sixth", "6"),
    ("seventh", "7"),
    ("eighth", "8"),
    ("ninth", "9"),
    ("tenth", "10"),
];

/// Ordinal words in prose, except those that order the text.
fn ordinals(text: &str) -> Vec<String> {
    prose(text)
        .split_whitespace()
        .filter_map(|token| {
            let body = token.trim_start_matches(|c: char| !c.is_alphanumeric());
            let word: String = body
                .chars()
                .take_while(|c| c.is_alphabetic())
                .collect::<String>()
                .to_lowercase();
            let rest = &body[word.len()..];
            if rest.starts_with(',') {
                return None;
            }
            ORDINAL_WORDS
                .iter()
                .find(|(w, _)| *w == word)
                .map(|(_, n)| n.to_string())
        })
        .collect()
}

/// The sign and currency before the digits at `digit`: whether a minus sign makes the
/// number negative, the currency mark with up to 3 capital letters before it ('$', 'US$',
/// '£'), and where that prefix starts. A minus counts only at the start of a word, so
/// '2-3' is a range and 'COVID-19' a name, and a currency mark may stand one space before
/// the digits, as in '$ 27.99'.
fn number_prefix(chars: &[char], digit: usize) -> (usize, bool, String) {
    let minus = |c: char| matches!(c, '-' | '\u{2212}');
    let currency_mark = |c: char| matches!(c, '$' | '£' | '€' | '¥');
    let mut start = digit;
    let mut sign_at = None;
    if start > 0 && minus(chars[start - 1]) {
        start -= 1;
        sign_at = Some(start);
    }
    let mark = if start > 0 && currency_mark(chars[start - 1]) {
        Some(start - 1)
    } else if sign_at.is_none()
        && start > 1
        && chars[start - 1] == ' '
        && currency_mark(chars[start - 2])
    {
        Some(start - 2)
    } else {
        None
    };
    let mut currency = String::new();
    if let Some(mark) = mark {
        let mut from = mark;
        while from > 0 && mark - from < 3 && chars[from - 1].is_ascii_uppercase() {
            from -= 1;
        }
        if from > 0 && chars[from - 1].is_alphanumeric() {
            from = mark;
        }
        currency = chars[from..=mark].iter().collect();
        start = from;
        if sign_at.is_none() && start > 0 && minus(chars[start - 1]) {
            start -= 1;
            sign_at = Some(start);
        }
    }
    match sign_at {
        // A hyphen after a word or figure joins, so it is not a sign. A currency mark
        // after it still counts: '2-$3' keeps '$3'.
        Some(at) if at > 0 && chars[at - 1].is_alphanumeric() => {
            let currency = if mark.is_some_and(|m| m > at) {
                currency
            } else {
                String::new()
            };
            (at + 1, false, currency)
        }
        signed => (start, signed.is_some(), currency),
    }
}

fn number_word(w: &str) -> Option<&'static str> {
    NUMBER_WORDS
        .iter()
        .find(|(word, _)| *word == w)
        .map(|(_, n)| *n)
}

fn code_spans(text: &str) -> Vec<String> {
    split_code(text).0
}

/// Code spans and the text outside them. As in Markdown, a span opens and closes
/// with the same number of backticks, and an unmatched run is literal text.
fn split_code(text: &str) -> (Vec<String>, String) {
    let (mut spans, mut outside) = (Vec::new(), String::new());
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        outside.push_str(&rest[..start]);
        let run = backtick_run(&rest[start..]);
        let after = &rest[start + run..];
        match closing_run(after, run) {
            Some(end) => {
                if !after[..end].is_empty() {
                    spans.push(after[..end].to_string());
                }
                outside.push(' ');
                rest = &after[end + run..];
            }
            None => {
                outside.push_str(&rest[start..start + run]);
                rest = after;
            }
        }
    }
    outside.push_str(rest);
    (spans, outside)
}

fn backtick_run(s: &str) -> usize {
    s.len() - s.trim_start_matches('`').len()
}

fn closing_run(s: &str, run: usize) -> Option<usize> {
    let mut i = 0;
    while let Some(p) = s[i..].find('`') {
        let len = backtick_run(&s[i + p..]);
        if len == run {
            return Some(i + p);
        }
        i += p + len;
    }
    None
}

fn urls(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter_map(|w| w.find("http").map(|p| &w[p..]))
        .filter(|w| w.starts_with("http://") || w.starts_with("https://"))
        .map(|w| {
            w.trim_end_matches(['.', ',', ')', ';', '\'', '"'])
                .to_string()
        })
        .collect()
}

/// Quoted spans outside code, without their marks, so a quotation has to survive word for word
/// but restyling its marks or reflowing its line changes nothing. A mark opens only at the start
/// of a word and closes only at the end of one, so an apostrophe does neither, and a stray
/// opening mark runs no further than its paragraph. One character, as in rock 'n' roll, is
/// an elision or a label rather than a quotation.
fn quotations(text: &str) -> Vec<String> {
    let outside = split_code(text).1.replace("\r\n", "\n");
    let mut found = Vec::new();
    for paragraph in outside.split("\n\n") {
        let chars: Vec<char> = paragraph.chars().collect();
        let mut open = None;
        for (i, &c) in chars.iter().enumerate() {
            let before = i.checked_sub(1).map(|p| chars[p]);
            let after = chars.get(i + 1);
            match open {
                None => {
                    let at_start = before
                        .is_none_or(|b| b.is_whitespace() || "([{*_:\u{2013}\u{2014}".contains(b));
                    if at_start
                        && after.is_some_and(|a| !a.is_whitespace())
                        && !(matches!(c, '\'' | '\u{2018}') && is_decade(&chars[i + 1..]))
                    {
                        open = closing_mark(c).map(|close| (i, close));
                    }
                }
                Some((start, close)) => {
                    let at_end = before.is_some_and(|b| !b.is_whitespace())
                        && !after.is_some_and(|a| a.is_alphanumeric());
                    if c == close && at_end {
                        open = None;
                        found.push(quoted_words(&chars[start + 1..i]));
                    }
                }
            }
        }
    }
    found.retain(|q| q.chars().nth(1).is_some());
    found
}

/// The digits and `s` of a decade, as in '90s, whose apostrophe opens nothing.
fn is_decade(after_mark: &[char]) -> bool {
    let digits = after_mark.iter().take_while(|c| c.is_ascii_digit()).count();
    digits > 0 && after_mark.get(digits) == Some(&'s')
}

fn closing_mark(opening: char) -> Option<char> {
    match opening {
        '"' | '\'' => Some(opening),
        '\u{201C}' => Some('\u{201D}'),
        '\u{2018}' => Some('\u{2019}'),
        _ => None,
    }
}

/// A quotation's words in one form: straight marks inside it, single spaces, and no trailing
/// comma or full stop, which styles put on either side of the closing mark.
fn quoted_words(chars: &[char]) -> String {
    let text: String = chars
        .iter()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            other => *other,
        })
        .collect();
    let spaced = text.split_whitespace().collect::<Vec<_>>().join(" ");
    spaced.trim_end_matches([',', '.', ';', ':']).to_string()
}

/// The text without code spans and URLs, so `--all` or a path is not a meaning word.
/// A link label before its URL, as in `[all](https://...)`, is kept.
fn prose(text: &str) -> String {
    split_code(text)
        .1
        .split_whitespace()
        .map(|w| {
            match [w.find("http://"), w.find("https://")]
                .into_iter()
                .flatten()
                .min()
            {
                Some(p) => &w[..p],
                None => w,
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Marks punctuation between words, so a phrase such as 'up to' cannot span 'up. To'.
const BOUNDARY: &str = "|";

/// Lowercase words, with a boundary after punctuation that ends a phrase. Link brackets
/// do not end one ('up [to](url)'), and nor does a mark before a digit ('over .5', '3,000').
fn words(text: &str) -> Vec<String> {
    let chars: Vec<char> = text
        .to_lowercase()
        .replace('\u{2019}', "'")
        .chars()
        .collect();
    let segmented: String = chars
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let ends_phrase = matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '(' | ')')
                && !chars.get(i + 1).is_some_and(char::is_ascii_digit);
            if ends_phrase { '\n' } else { c }
        })
        .collect();
    let mut out = Vec::new();
    for segment in segmented.split('\n') {
        out.extend(
            segment
                .split(|c: char| !(c.is_alphanumeric() || c == '\''))
                .map(|w| w.trim_matches('\'').to_string())
                .filter(|w| !w.is_empty()),
        );
        out.push(BOUNDARY.to_string());
    }
    out
}

fn is_number(word: &str) -> bool {
    word.starts_with(|c: char| c.is_ascii_digit()) || number_word(word).is_some()
}

/// Occurrences of a word or space-separated phrase, where `N` matches a number.
fn count(words: &[String], phrase: &str) -> usize {
    let parts: Vec<&str> = phrase.split(' ').collect();
    words
        .windows(parts.len())
        .filter(|window| {
            window.iter().zip(&parts).all(|(w, p)| match *p {
                "N" => is_number(w),
                _ => w == p,
            })
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_length_change_last_as_a_rounded_share() {
        let report = report("one two three", "one  two\nthree four");
        assert_eq!(
            report.lines.last().unwrap(),
            "length: 3 to 4 words (133% of the original)"
        );
        assert_eq!(
            length("a b c d e f", "a b c d"),
            "length: 6 to 4 words (67% of the original)"
        );
        assert_eq!(length("", "Some words."), "length: 0 to 2 words");
        assert_eq!(
            length("Keep it.", ""),
            "length: 2 to 0 words (0% of the original)"
        );
        assert_eq!(share(usize::MAX, usize::MAX), 100);
        assert_eq!(share(1, usize::MAX), usize::MAX as u128 * 100);
    }

    #[test]
    fn a_quotation_has_to_survive_word_for_word() {
        let original = "The minister said 'we will not proceed' on Tuesday.";
        let edited = "The minister said 'we will proceed' on Tuesday.";
        assert_eq!(
            missing(original, edited)["quotation"],
            vec!["we will not proceed"]
        );
        assert_eq!(
            added(original, edited)["quotation"],
            vec!["we will proceed"]
        );
    }

    #[test]
    fn restyling_or_reflowing_a_quotation_changes_nothing() {
        let original = "He said \"it isn't over,\" and left.";
        for edited in [
            "He said 'it isn't over', and left.",
            "He said \u{201C}it isn\u{2019}t over\u{201D}, and left.",
            "He said \u{2018}it isn\u{2019}t\nover\u{2019}, and left.",
        ] {
            assert!(
                !missing(original, edited).contains_key("quotation"),
                "{edited}"
            );
        }
    }

    #[test]
    fn an_apostrophe_is_no_quotation_mark() {
        let plain: Vec<String> = Vec::new();
        assert_eq!(quotations("It's the dogs' bowl, and 5\" wide."), plain);
        assert_eq!(quotations("Says 'don't stop' twice."), vec!["don't stop"]);
        assert!(missing("It was rock 'n' roll.", "It was rock and roll.").is_empty());
        let (decade, edit) = (
            "The '90s inspired the artists' work.",
            "The '90s shaped the artists' work.",
        );
        assert!(!missing(decade, edit).contains_key("quotation"));
        assert_eq!(
            quotations("Said \"90s music\" and '90 per cent'."),
            vec!["90s music", "90 per cent"]
        );
        assert_eq!(
            quotations("Said \u{201C}90s music\u{201D}."),
            vec!["90s music"]
        );
    }

    #[test]
    fn a_quotation_can_follow_a_colon_or_dash() {
        assert_eq!(quotations("He said:\u{201C}stop\u{201D}"), vec!["stop"]);
        assert_eq!(
            quotations("Then\u{2014}'stop now'\u{2014}he left."),
            vec!["stop now"]
        );
    }

    #[test]
    fn a_stray_mark_stops_at_its_paragraph_and_code_is_left_out() {
        assert_eq!(
            quotations("He said 'hello\n\nThen 'bye' came."),
            vec!["bye"]
        );
        assert_eq!(
            quotations("He said 'hello\r\n\r\nThen 'bye' came."),
            vec!["bye"]
        );
        assert_eq!(quotations("Use `'x'` here, and ''."), Vec::<String>::new());
    }

    #[test]
    fn reports_dropped_numbers_code_urls_and_negations() {
        let original = "Over 3,000 records, up to 4 at once. Run `make test`. See https://example.com/a. Exports must not include deleted accounts.";
        let edited = "Over 3,000 records. Run tests. Exports should include accounts.";
        let m = missing(original, edited);
        assert_eq!(m["number"], vec!["4"]);
        assert_eq!(m["code"], vec!["make test"]);
        assert_eq!(m["url"], vec!["https://example.com/a"]);
        assert_eq!(
            m["meaning word"],
            vec![
                "'not' (1 before, 0 after)",
                "'must' (1 before, 0 after)",
                "'up to' (1 before, 0 after)"
            ]
        );
    }

    #[test]
    fn reports_changed_quantifiers_modals_and_bounds() {
        let original =
            "Some users may see over 3,000 errors, up to 4 at once, and should retry three times.";
        let edited = "All users must see 3,000 errors, 4 at once, and must retry 3 times.";
        let m = missing(original, edited);
        assert!(!m.contains_key("number"), "three and 3 are the same number");
        assert_eq!(
            m["meaning word"],
            vec![
                "'may' (1 before, 0 after)",
                "'some' (1 before, 0 after)",
                "'should' (1 before, 0 after)",
                "'up to' (1 before, 0 after)",
                "'over N' (1 before, 0 after)"
            ]
        );
        let a = added(original, edited);
        assert!(!a.contains_key("number"));
        assert_eq!(
            a["meaning word"],
            vec!["'must' (0 before, 2 after)", "'all' (0 before, 1 after)"]
        );
    }

    #[test]
    fn link_labels_keep_their_meaning_words() {
        let m = missing(
            "See [all](https://example.com/a) docs. It is [not](https://example.com/b) final.",
            "See [some](https://example.com/a) docs. It is [now](https://example.com/b) final.",
        );
        assert_eq!(
            m["meaning word"],
            vec!["'not' (1 before, 0 after)", "'all' (1 before, 0 after)"]
        );
    }

    #[test]
    fn phrases_do_not_cross_punctuation() {
        let original = "The service is up. To confirm, run a check.";
        let edited = "The service is up. Run a check to confirm.";
        assert!(missing(original, edited).is_empty());
        assert!(added(original, edited).is_empty());
    }

    #[test]
    fn links_and_leading_decimal_points_do_not_split_phrases() {
        let m = missing(
            "Allow up [to](https://example.com) five retries over .5 seconds apart.",
            "Allow five retries .5 seconds apart.",
        );
        assert_eq!(
            m["meaning word"],
            vec![
                "'up to' (1 before, 0 after)",
                "'over N' (1 before, 0 after)"
            ]
        );
    }

    #[test]
    fn double_backtick_code_and_urls_hold_no_meaning_or_number_words() {
        let m = missing(
            "Run ``can --all`` from https://example.com/two-step today.",
            "Run ``may`` from https://example.com/one-step today.",
        );
        assert!(!m.contains_key("meaning word"), "{m:?}");
        assert!(!m.contains_key("number"), "{m:?}");
        assert_eq!(m["code"], vec!["can --all"]);
    }

    #[test]
    fn number_words_match_numerals_and_over_needs_a_number() {
        let original = "Run two checks over the weekend; twelve passed.";
        let edited = "Run 2 checks at the weekend; 12 passed.";
        assert!(missing(original, edited).is_empty());
        assert!(added(original, edited).is_empty());
    }

    #[test]
    fn a_faithful_edit_reports_nothing() {
        let original = "Due on 15 April. It may not run on Windows; see `--force`.";
        let edited = "It may not run on Windows (see `--force`). Due on 15 April.";
        assert!(missing(original, edited).is_empty());
        assert!(added(original, edited).is_empty());
    }

    #[test]
    fn reports_invented_figures_links_code_and_obligations() {
        let original = "Refund the fee. See the policy.";
        let edited = "You must refund the $1,200 fee within 14 days. See https://example.com/p and run `refund --all`.";
        let a = added(original, edited);
        assert_eq!(a["number"], vec!["$1,200", "14"]);
        assert_eq!(a["url"], vec!["https://example.com/p"]);
        assert_eq!(a["code"], vec!["refund --all"]);
        assert_eq!(a["meaning word"], vec!["'must' (0 before, 1 after)"]);
        assert!(missing(original, edited).is_empty());
    }

    #[test]
    fn reports_changed_magnitudes_and_units_once() {
        let m = missing(
            "Revenue was $47.3 million. The site is 150 km away.",
            "Revenue was $47.3 billion. The site is 150 miles away.",
        );
        assert_eq!(m["quantity"], vec!["47.3 million", "150 km"]);
        assert!(!m.contains_key("number"), "{m:?}");
        let a = added(
            "Revenue was $47.3 million. The site is 150 km away.",
            "Revenue was $47.3 billion. The site is 150 miles away.",
        );
        assert_eq!(a["quantity"], vec!["47.3 billion", "150 mile"]);
    }

    #[test]
    fn quantity_spellings_match_and_a_dropped_number_reports_once() {
        for (original, edited) in [
            ("Revenue was $47.3M.", "Revenue was $47.3 million."),
            ("Allow 3 days.", "Allow a 3-day window, or three days."),
            ("It weighs 5kg.", "It weighs 5 kilograms."),
            ("Fees of 2bn.", "Fees of 2 billion."),
        ] {
            assert!(missing(original, edited).is_empty(), "{original}");
            assert!(added(original, edited).is_empty(), "{original}");
        }
        let lower_case_m = missing("A 5m cable.", "A 5M cable.");
        assert_eq!(lower_case_m["quantity"], vec!["5 m"]);
        let m = missing("Revenue was $47.3 million.", "Revenue rose.");
        assert_eq!(m["number"], vec!["$47.3"]);
        assert!(!m.contains_key("quantity"), "{m:?}");
    }

    #[test]
    fn reports_a_changed_month_and_reads_dated_may_as_a_month() {
        let original = "The contract ends on 3 March 2027.";
        let edited = "The contract ends on 3 May 2027.";
        let m = missing(original, edited);
        assert_eq!(m["month"], vec!["March"]);
        assert!(!m.contains_key("meaning word"), "{m:?}");
        let a = added(original, edited);
        assert_eq!(a["month"], vec!["May"]);
        assert!(!a.contains_key("meaning word"), "{a:?}");
        assert_eq!(
            missing("It may run on 3 May.", "It runs on 3 May.")["meaning word"],
            vec!["'may' (1 before, 0 after)"]
        );
        assert!(missing("Due 3 Sept.", "Due 3 September.").is_empty());
        assert!(!missing("March on. May I?", "Go on. Ask.").contains_key("month"));
    }

    #[test]
    fn reports_dropped_hedges_commitments_conditions_and_order() {
        let m = missing(
            "Costs rose about 50%. It will likely pass if tested. Pay before 3 March.",
            "Costs rose 50%. It passes when tested. Pay after 3 March.",
        );
        assert_eq!(
            m["meaning word"],
            vec![
                "'will' (1 before, 0 after)",
                "'if' (1 before, 0 after)",
                "'before' (1 before, 0 after)",
                "'likely' (1 before, 0 after)",
                "'about N' (1 before, 0 after)"
            ]
        );
        assert!(missing("Ask about the fee.", "Ask the fee.").is_empty());
    }

    #[test]
    fn required_style_edits_are_not_drops_or_additions() {
        for (original, edited) in [
            (
                "We processed 1000 invoices.",
                "We processed 1,000 invoices.",
            ),
            ("Costs rose 5 per cent.", "Costs rose 5%."),
            ("Costs rose 5 percent.", "Costs rose 5%."),
            ("Costs rose five per cent.", "Costs rose 5%."),
            (
                "1. Open the file.\n2) Save it.",
                "Open the file, then save it.",
            ),
        ] {
            assert!(missing(original, edited).is_empty(), "{original}");
            assert!(added(original, edited).is_empty(), "{original}");
        }
        assert_eq!(
            missing("Costs rose 5%.", "Costs rose 5.")["number"],
            vec!["5%"]
        );
        assert_eq!(
            missing("Items 1,2,3 failed.", "Items 123 failed.")["number"],
            vec!["1,2,3"]
        );
        assert_eq!(
            missing("In 2026. It ran.", "It ran.")["number"],
            vec!["2026"]
        );
    }

    #[test]
    fn a_hyphenated_number_word_matches_its_numeral() {
        for (original, edited) in [
            ("A five-day window.", "A 5-day window."),
            ("A 5-day window.", "A five-day window."),
        ] {
            assert!(missing(original, edited).is_empty(), "{original}");
            assert!(added(original, edited).is_empty(), "{original}");
        }
        assert_eq!(
            missing("A five-day window.", "A 5-week window.")["quantity"],
            vec!["5 day"]
        );
    }

    #[test]
    fn bytes_and_bits_are_different_units() {
        assert_eq!(
            missing("Send 100 MB.", "Send 100 Mb.")["quantity"],
            vec!["100 MB"]
        );
        assert!(missing("Send 100 MB.", "Send 100 megabytes.").is_empty());
        assert!(missing("Send 100 Mb.", "Send 100 megabits.").is_empty());
    }

    #[test]
    fn a_name_is_not_a_modal() {
        for original in ["Will Smith signed the plan.", "May Parker signed the plan."] {
            let m = missing(original, "The plan was signed.");
            assert!(!m.contains_key("meaning word"), "{m:?}");
        }
        assert_eq!(
            missing("It will pass.", "It passes.")["meaning word"],
            vec!["'will' (1 before, 0 after)"]
        );
        for (original, word) in [("May I leave?", "may"), ("Will you attend?", "will")] {
            assert_eq!(
                missing(original, "Please confirm.")["meaning word"],
                vec![format!("'{word}' (1 before, 0 after)")],
                "{original}"
            );
        }
        assert!(names("May I leave?", "Please confirm.").is_empty());
        assert_eq!(
            missing("Will Smith will sign.", "Will Smith signs.")["meaning word"],
            vec!["'will' (1 before, 0 after)"]
        );
    }

    #[test]
    fn equivalent_hedges_match_and_a_dropped_hedge_reports() {
        for hedge in ["approximately", "roughly", "around"] {
            let hedged = format!("Costs rose {hedge} 50%.");
            assert!(
                missing(&hedged, "Costs rose about 50%.").is_empty(),
                "{hedge}"
            );
            assert!(
                added(&hedged, "Costs rose about 50%.").is_empty(),
                "{hedge}"
            );
            assert_eq!(
                missing(&hedged, "Costs rose 50%.")["meaning word"],
                vec!["'about N' (1 before, 0 after)"],
                "{hedge}"
            );
        }
        assert!(missing("Walk around 3 sites.", "Walk 3 sites.").contains_key("meaning word"));
        assert_eq!(
            missing("Roughly half left.", "Half left.")["meaning word"],
            vec!["'roughly' (1 before, 0 after)"]
        );
    }

    #[test]
    fn a_tab_after_a_list_marker_is_still_a_marker() {
        assert!(missing("1.\tOpen it.\n2)\tSave it.", "Open it, then save it.").is_empty());
    }

    #[test]
    fn names_are_advisory_and_ignore_a_leading_common_word() {
        assert_eq!(
            names(
                "Priya Nair approved it. The Audit Committee met.",
                "It was approved."
            ),
            vec!["Priya Nair", "Audit Committee"]
        );
        assert!(names("Priya Nair approved it.", "**Priya Nair** signed off.").is_empty());
        assert!(names("The file is here. Run it.", "Here it is.").is_empty());
        assert!(
            names(
                "## Key Changes
**Improved Reliability** added.",
                "Added."
            )
            .is_empty()
        );
        assert!(
            names(
                "Ended
Priya said.",
                "Priya said."
            )
            .is_empty()
        );
        assert!(names("Enhanced Caching Now Implemented Here.", "Cached.").is_empty());
    }

    #[test]
    fn a_bold_list_marker_numbers_the_layout_but_a_changed_figure_still_reports() {
        let original = "**1. Start early**\n\n**2. Keep it quiet**";
        let edited = "1. Start early\n2. Keep it quiet";
        assert!(missing(original, edited).is_empty());
        assert!(added(original, edited).is_empty());
        assert_eq!(
            missing("**1. Pay $500 now**", "1. Pay $400 now")["number"],
            vec!["$500"]
        );
        assert_eq!(
            missing("**3 of 4** passed.", "**2 of 4** passed.")["number"],
            vec!["3"]
        );
    }

    #[test]
    fn ordinal_words_are_numbers_unless_they_order_the_text() {
        assert_eq!(
            missing("Coaching for first-time triathletes.", "Coaching for triathletes.")["number"],
            vec!["1"]
        );
        assert!(missing("It came first.", "It came 1st.").is_empty());
        assert!(added("It came first.", "It came 1st.").is_empty());
        assert!(missing("First, run it.", "Run it.").is_empty());
        assert_eq!(
            missing("Pay the first instalment.", "Pay the third instalment.")["number"],
            vec!["1"]
        );
        assert_eq!(
            added("Pay the first instalment.", "Pay the third instalment.")["number"],
            vec!["3"]
        );
    }

    #[test]
    fn every_negative_contraction_counts_as_not() {
        for original in ["The build didn't pass.", "The fee wasn't paid.", "It won't ship."] {
            assert_eq!(
                missing(original, &original.replace("n't", "").replace("wo ", "will "))
                    ["meaning word"],
                vec!["'not' (1 before, 0 after)"],
                "{original}"
            );
        }
        assert!(missing("Do not deploy.", "Don't deploy.").is_empty());
        assert!(added("Do not deploy.", "Don't deploy.").is_empty());
        assert!(missing("It can\u{2019}t run.", "It can not run.").is_empty());
        assert_eq!(
            added("The build passed.", "The build didn't pass.")["meaning word"],
            vec!["'not' (0 before, 1 after)"]
        );
    }

    #[test]
    fn sign_brackets_and_currency_belong_to_the_value() {
        for (original, edited, dropped) in [
            ("It costs $500.", "It costs £500.", "$500"),
            ("Pay US$5.", "Pay A$5.", "US$5"),
            ("Growth was -3%.", "Growth was 3%.", "-3%"),
            ("A loss of (1,200).", "A loss of 1,200.", "-1,200"),
            ("A loss of ($250).", "A loss of $250.", "-$250"),
            ("A refund of $250.", "A refund of ($250).", "$250"),
        ] {
            assert_eq!(missing(original, edited)["number"], vec![dropped], "{original}");
        }
        assert_eq!(added("It costs $500.", "It costs £500.")["number"], vec!["£500"]);
        for (original, edited) in [
            ("Growth was \u{2212}3%.", "Growth was -3%."),
            ("A loss of (1,200).", "A loss of -1,200."),
            ("It costs $ 27.99.", "It costs $27.99."),
            ("A loss of -$12,000.", "A loss of -$12000."),
            ("Read pages 2-3 on COVID-19.", "Read pages 2 to 3 on COVID 19."),
            ("See note (3).", "See note 3."),
        ] {
            assert!(missing(original, edited).is_empty(), "{original}");
            assert!(added(original, edited).is_empty(), "{original}");
        }
    }

    #[test]
    fn time_temperature_mass_and_volume_units_are_quantities() {
        for (original, edited, dropped) in [
            ("Latency is 200 ms.", "Latency is 200 s.", "200 ms"),
            ("Store below 25 °C.", "Store below 25 °F.", "25 °C"),
            ("Store below 25°C.", "Store below 25°F.", "25 °C"),
            ("Take 5 mg daily.", "Take 5 g daily.", "5 mg"),
            ("Add 2 L of water.", "Add 2 mL of water.", "2 L"),
        ] {
            assert_eq!(missing(original, edited)["quantity"], vec![dropped], "{original}");
        }
        for (original, edited) in [
            ("Latency is 200 ms.", "Latency is 200 milliseconds."),
            ("Take 5 g daily.", "Take 5 grams daily."),
            ("Add 2 L of water.", "Add 2 litres of water."),
        ] {
            assert!(missing(original, edited).is_empty(), "{original}");
        }
    }

    #[test]
    fn a_name_only_the_edit_has_is_listed_and_never_fails() {
        let report = report("The plan was approved.", "The plan was approved by Jane Smith.");
        assert!(report.lines.contains(&"check added name: Jane Smith".to_string()));
        assert!(!report.dropped && !report.added);
    }
}
