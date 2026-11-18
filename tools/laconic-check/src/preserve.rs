// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Protected items an edit must keep: numbers, quantities, dated months, day names, inline code,
//! URLs, quotations and the words that carry negation, modality, certainty, condition, order and
//! scope. Reports what the original had and the edit lost, and what the edit added. A
//! clean result does not prove the meaning survived; it only rules out these losses. An
//! addition is a reason to check the edit invented nothing, not proof that it did.

use std::collections::{BTreeMap, HashSet};

mod reversal;

const MEANING_WORDS: &[&str] = &[
    "not",
    "never",
    "no",
    "none",
    "nothing",
    "nor",
    "neither",
    "without",
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

/// Meaning words whose loss `report` lists to check under `--loose`, without failing: frequency,
/// quantifiers, ability, 'should', prediction, cause and hedges. A heavy paraphrase often drops
/// one without changing a fact, but a lost hedge can turn a qualified claim into a categorical
/// one, so the default fails on them. Negation, conditions, 'only', 'every', order, 'must',
/// 'shall', 'may', 'might', 'could' and bounds fail either way.
const CHECK_WORDS: &[&str] = &[
    "some",
    "all",
    "any",
    "most",
    "should",
    "can",
    "always",
    "will",
    "because",
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
    (
        &[
            "ml",
            "millilitre",
            "millilitres",
            "milliliter",
            "milliliters",
        ],
        "mL",
    ),
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

/// Day names, capitalised as written, with their plurals: 'on Sundays' keeps Sunday.
const WEEKDAYS: &[&str] = &[
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
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
    let kept = word_runs(&prose(edited).replace('*', ""));
    dedup(name_runs(original))
        .into_iter()
        .filter(|name| !kept.contains(name))
        .collect()
}

/// Every run of 2 or 3 words in `text` as a name in it would read, so looking up a name
/// takes constant time: the first word without opening marks, as in '(Priya', and the last
/// without closing marks or what follows a mark inside it, as in 'Nair's' and 'Nair-Smith'.
/// A run does not span a word that ends in a mark, such as 'Priya,'.
fn word_runs(text: &str) -> HashSet<String> {
    let open = |word: &str| {
        word.trim_start_matches(|c: char| !c.is_alphanumeric())
            .to_owned()
    };
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut runs = HashSet::new();
    for (i, last) in words.iter().enumerate() {
        let whole = last.trim_end_matches(|c: char| !c.is_alphanumeric());
        let stem = &whole[..whole
            .find(|c: char| !c.is_alphanumeric())
            .unwrap_or(whole.len())];
        let possessive = whole
            .strip_suffix("'s")
            .or_else(|| whole.strip_suffix("\u{2019}s"))
            .unwrap_or(whole);
        let lasts = [whole, stem, possessive];
        if i >= 1 {
            let first = open(words[i - 1]);
            runs.extend(lasts.map(|end| format!("{first} {end}")));
        }
        if i >= 2 {
            let first = format!("{} {}", open(words[i - 2]), words[i - 1]);
            runs.extend(lasts.map(|end| format!("{first} {end}")));
        }
    }
    runs
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

/// `loose` is `--loose`: a lost word of `CHECK_WORDS` is listed to check, as a lost name is, and
/// does not fail. A swap such as 'all' to 'some' also adds a word, which `--strict` fails on.
pub fn report(original: &str, edited: &str, loose: bool) -> Report {
    let mut missing = missing(original, edited);
    let mut loosened = Vec::new();
    if loose && let Some(words) = missing.remove("meaning word") {
        let (check, fail): (Vec<String>, Vec<String>) =
            words.into_iter().partition(|item| to_check(item));
        loosened = check;
        if !fail.is_empty() {
            missing.insert("meaning word", fail);
        }
    }
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
        loosened
            .iter()
            .map(|item| format!("check meaning word: {item}")),
    );
    // Like a lost name, a possible reversal is advice and never fails.
    lines.extend(
        reversal::reversals(&split_code(original).1, &split_code(edited).1)
            .into_iter()
            .map(|item| format!("check reversal: {item}")),
    );
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

/// Whether a meaning-word item, such as `'can' (5 before, 4 after)`, names a word of `CHECK_WORDS`.
fn to_check(item: &str) -> bool {
    item.strip_prefix('\'')
        .and_then(|rest| rest.split_once("' ("))
        .is_some_and(|(word, _)| CHECK_WORDS.contains(&word))
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

/// A report kind and the function that lists its items in a text.
type Extractor<'a> = (&'static str, &'a dyn Fn(&str) -> Vec<String>);

/// Items in `from` that `to` lacks; `order` maps the 2 meaning-word counts to
/// (before, after) for the report.
fn only_in(
    from: &str,
    to: &str,
    order: fn(usize, usize) -> (usize, usize),
) -> BTreeMap<&'static str, Vec<String>> {
    let mut out: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    // 'pound' names money or a weight: 'N pounds' is money when either text writes '£N'.
    // shortcut: a weight of 5 pounds beside a '£5' amount then reads as money; telling them
    // apart needs each sentence's sense.
    let pounds: HashSet<String> = [from, to]
        .iter()
        .flat_map(|t| numbers(t, &HashSet::new()))
        .filter(|n| n.contains('£'))
        .map(|n| unsigned(&n))
        .collect();
    // Unsigned values, as a quantity carries no sign or currency.
    let values_to: HashSet<String> = numbers(to, &pounds).iter().map(|n| unsigned(n)).collect();
    let extractors: [Extractor; 7] = [
        ("number", &|t| numbers(t, &pounds)),
        ("quantity", &|t| quantities(t, &pounds)),
        ("month", &months),
        ("weekday", &weekdays),
        ("code", &code_spans),
        ("url", &urls),
        ("quotation", &quotations),
    ];
    for (kind, extract) in extractors {
        let key = |x: &String| match kind {
            "number" => number_value(x),
            "quotation" => quotation_key(x),
            _ => x.clone(),
        };
        let kept: HashSet<String> = extract(to).iter().map(key).collect();
        let lost: Vec<String> = dedup(extract(from))
            .into_iter()
            .filter(|x| !kept.contains(&key(x)))
            // A lost number already reports itself; a quantity adds only a changed size or unit.
            .filter(|x| {
                kind != "quantity" || x.split(' ').next().is_some_and(|n| values_to.contains(n))
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
/// 'about 50' is not a change.
fn meaning_tokens(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = words(&prose(text))
        .into_iter()
        .flat_map(uncontracted)
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

/// A negative contraction or 'cannot' as its auxiliary and 'not', so 'don't' and 'do not'
/// match, as do 'won't' and 'will not', but 'can't' to 'won't' drops 'can'.
fn uncontracted(word: String) -> Vec<String> {
    let auxiliary = match word.strip_suffix("n't") {
        Some("wo") => "will",
        Some("ca") => "can",
        Some("sha") => "shall",
        Some(stem) => stem,
        None if word == "cannot" => "can",
        None => return vec![word],
    };
    [auxiliary, "not"]
        .into_iter()
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
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

/// The items in their first order, each once.
fn dedup(mut v: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    v.retain(|x| seen.insert(x.clone()));
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

/// A number's value without its sign or currency.
fn unsigned(token: &str) -> String {
    number_value(token)
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .to_string()
}

/// The text without ordered list markers, which number the layout and not the content,
/// including a marker inside bold or strong emphasis ('**1. Plan**', '**1.** Plan').
/// shortcut: a hard-wrapped line starting '12. ' reads as a marker, so markers stop at
/// 2 digits; a longer list reports its later markers as numbers.
fn without_list_markers(text: &str) -> String {
    text.lines()
        .map(|line| {
            let body = line.trim_start();
            let (body, close) = ["**", "__"]
                .iter()
                .find_map(|mark| Some((body.strip_prefix(mark)?, *mark)))
                .unwrap_or((body, ""));
            let digits = body.len() - body.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            let text = body[digits..]
                .strip_prefix(['.', ')'])
                .map(|rest| rest.strip_prefix(close).unwrap_or(rest))
                .and_then(|rest| rest.strip_prefix([' ', '\t']));
            match text {
                Some(text) if (1..=2).contains(&digits) => text,
                _ => line,
            }
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
/// '150 km', 'three days', '25 degrees Celsius'. Both bounds of a range carry its unit, so
/// '45 to 60 minutes', '45-60 minutes', '45 – 60 minutes' and 'a 45- to 60-minute ride' are each
/// 45 and 60 minutes.
/// Read from prose, so a value in code or a URL is not one. 'pounds' is not a weight after a
/// value in `pounds`, the amounts written with '£'.
fn quantities(text: &str, pounds: &HashSet<String>) -> Vec<String> {
    let text = dash_ranges(&prose(text));
    let tokens: Vec<&str> = text.split_whitespace().collect();
    (0..tokens.len())
        .filter_map(|i| quantity_at(&tokens, i, pounds, false))
        .map(|(value, unit)| format!("{value} {unit}"))
        .collect()
}

/// The text with a hyphen or en dash between 2 numbers written as ' to ', so '45-60 minutes'
/// reads as '45 to 60 minutes'. A word with more than one, such as the date '2026-11-14' or
/// the phone number '0400-123-456', is no range and stays as it is.
fn dash_ranges(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let chars: Vec<char> = word.chars().collect();
            let dashes: Vec<usize> = (1..chars.len().saturating_sub(1))
                .filter(|&i| {
                    matches!(chars[i], '-' | '\u{2013}')
                        && chars[i - 1].is_ascii_digit()
                        && chars[i + 1].is_ascii_digit()
                })
                .collect();
            match dashes[..] {
                [at] => {
                    let (low, high) = (&chars[..at], &chars[at + 1..]);
                    format!("{} to {}", String::from_iter(low), String::from_iter(high))
                }
                _ => word.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The value of the number at token `i` and its unit, if it has one. `upper` marks a range's
/// upper bound, which does not look for a further range.
fn quantity_at(
    tokens: &[&str],
    i: usize,
    pounds: &HashSet<String>,
    upper: bool,
) -> Option<(String, &'static str)> {
    let body = tokens
        .get(i)?
        .trim_start_matches(|c: char| !c.is_alphanumeric());
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
        (number_word(&word)?, &body[word.len()..])
    } else {
        (value, attached)
    };
    let bare = value.trim_end_matches(['.', ',']);
    if bare.is_empty() {
        return None;
    }
    let unit = attached.trim_start_matches('-');
    let word = if !unit
        .trim_end_matches(|c: char| !c.is_alphanumeric())
        .is_empty()
    {
        unit
    } else if value.len() == bare.len() && matches!(attached, "" | "-") {
        let next = tokens.get(i + 1)?;
        // A range's lower bound takes the unit of its upper bound: '45 to 60 minutes',
        // 'between 2 and 3 hours', '2 or 3 days', '30 – 60 grams', 'a 45- to 60-minute ride'.
        if matches!(*next, "to" | "and" | "or" | "-" | "\u{2013}" | "\u{2014}") {
            if upper {
                return None;
            }
            let (_, unit) = quantity_at(tokens, i + 2, pounds, true)?;
            // A price takes a size across a range, never a unit: 'a fee of $5 and 10 kg'.
            let priced = tokens[i][..tokens[i].len() - body.len()].contains(currency_mark);
            let size = matches!(unit, "thousand" | "million" | "billion" | "trillion");
            return (!priced || size).then(|| (number_value(bare), unit));
        }
        // '25 degrees Celsius' names its unit after 'degrees'.
        let degrees = matches!(next.to_lowercase().as_str(), "degree" | "degrees");
        tokens.get(i + 1 + usize::from(degrees))?
    } else {
        return None;
    };
    let word = word.trim_end_matches(|c: char| !c.is_alphanumeric());
    let pound = matches!(word.to_lowercase().as_str(), "pound" | "pounds");
    let money = pound && pounds.contains(&number_value(bare));
    let unit = quantity_word(word).filter(|_| !money)?;
    Some((number_value(bare), unit))
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

/// Day names in prose, as whole words between any non-letters ('Thursday's',
/// 'Monday-Friday'), a plural read as its day.
fn weekdays(text: &str) -> Vec<String> {
    prose(text)
        .split(|c: char| !c.is_alphabetic())
        .filter_map(|t| {
            WEEKDAYS
                .iter()
                .find(|day| t == **day || t.strip_suffix('s') == Some(day))
                .map(|day| day.to_string())
        })
        .collect()
}

/// Words before 'H:MM' that make it a time: 'at 5:30', 'between 6:00'.
const TIME_BEFORE: &[&str] = &[
    "at", "by", "after", "before", "from", "until", "till", "between", "around", "about",
];

/// Words that join a time to the next one in a range: 'from 6:00 to 8:00', 'between 6:00 PM
/// and 8:00'. After any other number they join 2 ratios or scores, so they make no time.
const TIME_RANGE: &[&str] = &["to", "and"];

/// Whether the 'H:MM' at `chars[start..end]` reads as a time: 'am', 'pm', 'a.m.' or 'p.m.'
/// follows it, a word in TIME_BEFORE comes before it, or, when `after_time` says a time ends
/// just before it, a word in TIME_RANGE does.
fn time_context(chars: &[char], start: usize, end: usize, after_time: bool) -> bool {
    let after: String = chars[end..chars.len().min(end + 5)]
        .iter()
        .collect::<String>()
        .to_lowercase();
    let after = after.strip_prefix(' ').unwrap_or(&after);
    let meridiem = ["am", "pm", "a.m", "p.m"].iter().any(|m| {
        after
            .strip_prefix(m)
            .is_some_and(|rest| !rest.starts_with(char::is_alphanumeric))
    });
    let mut before: Vec<char> = chars[..start]
        .iter()
        .rev()
        .skip_while(|c| **c == ' ')
        .take_while(|c| c.is_alphabetic())
        .copied()
        .collect();
    before.reverse();
    let before = before.into_iter().collect::<String>().to_lowercase();
    meridiem
        || TIME_BEFORE.contains(&before.as_str())
        || (after_time && TIME_RANGE.contains(&before.as_str()))
}

/// The most characters between a time and the next one in a range, as in ' PM and '.
const RANGE_GAP: usize = 10;

/// Numbers with their sign and currency; `pounds` lists the values that 'pounds' after a
/// number names as money.
fn numbers(text: &str, pounds: &HashSet<String>) -> Vec<String> {
    let text = without_list_markers(text);
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    // Where the last number started and ended, and its currency.
    let mut last: Option<(usize, usize, String)> = None;
    // Where the last time ended.
    let mut last_time: Option<usize> = None;
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
            let mut token: String = chars[i..j]
                .iter()
                .collect::<String>()
                .trim_end_matches(['.', ','])
                .to_string();
            // A time is one number, its hour and minutes, and ':00' adds nothing to the hour:
            // '5:00 p.m.' and '5pm' name the same time, but '5:30' is not 5. Out of a time's
            // context, as in the ratio '5:30' or 'John 3:16', each side is its own number.
            let minutes = chars
                .get(j + 1..j + 3)
                .filter(|m| m.iter().all(char::is_ascii_digit));
            if let Some(minutes) = minutes
                && chars[j] == ':'
                && token.chars().all(|c| c.is_ascii_digit())
                && !chars.get(j + 3).is_some_and(char::is_ascii_digit)
                && time_context(
                    &chars,
                    i,
                    j + 3,
                    last_time.is_some_and(|end| i - end <= RANGE_GAP),
                )
            {
                if minutes != ['0', '0'] {
                    token = format!("{token}:{}", minutes.iter().collect::<String>());
                }
                j += 3;
                last_time = Some(j);
            }
            let after: String = chars[j..chars.len().min(j + 12)].iter().collect();
            let percent = !token.ends_with('%') && starts_with_percent(&after);
            let token = if percent { token + "%" } else { token };
            let (start, mut negative, mut currency) = number_prefix(&chars, i);
            // A full stop or comma after the digits ends the clause: 'Pay 30. Dollars ...'.
            if currency.is_empty() && !matches!(chars[j - 1], '.' | ',') {
                let pound = pounds.contains(&number_value(&token));
                currency = spelled_currency(&chars[j..], pound).to_string();
            }
            // A range's upper bound without its own currency shares its lower bound's: '$5-10'
            // is $5 to $10, but 'from $5 to 10 euros' ends in euros.
            if let Some((from, end, mark)) = &last
                && currency.is_empty()
                && chars
                    .get(*end..start)
                    .is_some_and(|between| joins_range(&chars[..*from], between))
            {
                currency = mark.clone();
            }
            last = Some((start, j, currency.clone()));
            // Accounting brackets mark a negative amount of money or a grouped figure: '($250)'
            // or '(1,200)'. Any other number in brackets, such as a note '(3)', a share '(62%)'
            // or '(1,200%)', or a ratio '(2.5)', is an aside, not an amount.
            let amount = !currency.is_empty() || (token.contains(',') && !token.ends_with('%'));
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
    out
}

/// The sign and currency before the digits at `digit`: whether a minus sign makes the
/// number negative, the currency mark with up to 3 capital letters before it ('$', 'US$',
/// '£', '₹'), and where that prefix starts. A currency mark is '$', '£', '¥' or one from the
/// Unicode Currency Symbols block, such as '€', '₹' or '₩'. A minus counts only at the start
/// of a word, so '2-3' is a range and 'COVID-19' a name, and a currency mark may stand one
/// space before the digits, as in '$ 27.99'.
fn number_prefix(chars: &[char], digit: usize) -> (usize, bool, String) {
    let minus = |c: char| matches!(c, '-' | '\u{2212}');
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

/// Whether 2 numbers form a range, given the text `before` the first and `between` them: a
/// hyphen or en dash, or 'to', 'and' or 'or' after 'from' or 'between' ('from $5 to 10'), so
/// 'donated $50 to 3 charities' is no range.
fn joins_range(before: &[char], between: &[char]) -> bool {
    match String::from_iter(between).trim() {
        "-" | "\u{2013}" => true,
        "to" | "and" | "or" => {
            let before = String::from_iter(&before[before.len().saturating_sub(16)..]);
            let word = before.trim_end().rsplit(char::is_whitespace).next();
            word.is_some_and(|w| matches!(w.to_lowercase().as_str(), "from" | "between"))
        }
        _ => false,
    }
}

/// The currency mark a word after a number names, at once or after a size, so '30 dollars'
/// and '30 billion dollars' are '$30'. A range's lower bound takes the word after its upper
/// bound: '5-10 dollars' and '5 to 10 dollars' are '$5' to '$10'. 'pound' is also a weight,
/// so it names '£' only when `money` is set.
fn spelled_currency(after: &[char], money: bool) -> &'static str {
    let after: String = after.iter().take(40).collect();
    let mut words = after
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()));
    let numeral = |w: &str| w.starts_with(|c: char| c.is_ascii_digit());
    let mut word = words.next().unwrap_or_default();
    if after.starts_with(['-', '\u{2013}']) && numeral(word) {
        word = words.next().unwrap_or_default();
    } else if matches!(word, "to" | "and" | "or") && words.clone().next().is_some_and(numeral) {
        words.next();
        word = words.next().unwrap_or_default();
    }
    if matches!(
        quantity_word(word),
        Some("thousand" | "million" | "billion" | "trillion")
    ) {
        word = words.next().unwrap_or_default();
    }
    match word.to_lowercase().as_str() {
        "dollar" | "dollars" => "$",
        "euro" | "euros" => "€",
        "yen" => "¥",
        "pound" | "pounds" if money => "£",
        _ => "",
    }
}

/// A currency mark: '$', '£', '¥' or one from the Unicode Currency Symbols block.
fn currency_mark(c: char) -> bool {
    matches!(c, '$' | '£' | '¥' | '\u{20A0}'..='\u{20CF}')
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

/// A quotation as compared: a quotation set as its own sentence takes a capital and a question
/// or exclamation mark ('can I afford it' as "Can I afford it?"), so neither counts.
fn quotation_key(quotation: &str) -> String {
    let words = quotation.trim_end_matches(['?', '!']);
    let mut chars = words.chars();
    chars
        .next()
        .map(|first| first.to_lowercase().chain(chars).collect())
        .unwrap_or_default()
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
        let report = report("one two three", "one  two\nthree four", false);
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
    fn a_quotation_set_as_its_own_sentence_keeps_its_words() {
        let original = "It shifted from \"can I afford to publish this book\" to \"will anyone buy it,\" lowering the stakes.";
        let edited = "It changed from \"Can I afford to publish this book?\" to \"Will anyone buy it?\" This lowered the stakes.";
        assert!(!missing(original, edited).contains_key("quotation"));
        assert!(!added(original, edited).contains_key("quotation"));
        // The words still have to match.
        assert_eq!(
            missing("Asked 'can we stop'.", "Asked 'Can we start?'.")["quotation"],
            vec!["can we stop"]
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
    fn reports_a_dropped_weekday_and_reads_plurals_as_the_day() {
        let original = "The trackers close on Thursday. Backups run on Sundays.";
        let m = missing(
            original,
            "The trackers close soon. Backups run every Sunday.",
        );
        assert_eq!(m["weekday"], vec!["Thursday"]);
        let a = added(
            original,
            "The trackers close on Friday. Backups run on Sundays.",
        );
        assert_eq!(a["weekday"], vec!["Friday"]);
        // Only the capitalised day names count, as whole words.
        assert!(!missing("A sunday drive.", "A drive.").contains_key("weekday"));
        assert!(!missing("Sundayland opens.", "It opens.").contains_key("weekday"));
        assert_eq!(
            missing(
                "Thursday's draft covers Monday-Friday.",
                "The draft covers it."
            )["weekday"],
            vec!["Thursday", "Monday", "Friday"]
        );
    }

    #[test]
    fn the_report_lists_a_possible_reversal_to_check_outside_code() {
        // A negation inside code is not prose.
        let lines = report(
            "Pass `--not cached` to the tool.",
            "Pass `--cached` to the tool.",
            false,
        )
        .lines;
        assert!(
            !lines.iter().any(|l| l.starts_with("check reversal:")),
            "{lines:?}"
        );
        let lines = report("Do not delete the cache.", "Delete the cache.", true).lines;
        assert!(
            lines.contains(
                &"check reversal: 'delete' is no longer negated in \"Delete the cache.\""
                    .to_owned()
            ),
            "{lines:?}"
        );
    }

    #[test]
    fn a_time_on_the_hour_matches_the_hour_alone() {
        let m = missing(
            "Stop after 5:00 p.m. on Friday.",
            "Stop after 5pm on Friday.",
        );
        assert!(!m.contains_key("number"), "{m:?}");
        // A changed hour or minutes is a dropped time, whichever text has the minutes.
        for (original, edited, dropped) in [
            ("Meet at 5:30 pm.", "Meet at 5 pm.", "5:30"),
            ("Meet at 5:00.", "Meet at 5:30.", "5"),
            ("Meet at 5:00.", "Meet at 6:00.", "5"),
        ] {
            assert_eq!(missing(original, edited)["number"], vec![dropped]);
        }
        // Without 2 digits after the colon, it is not a time.
        assert_eq!(
            missing("A 16:9 screen.", "A screen.")["number"],
            vec!["16", "9"]
        );
    }

    #[test]
    fn a_colon_number_is_a_time_only_in_a_time_context() {
        for (original, edited) in [
            ("Stop by 5:00 p.m.", "Stop by 5 p.m."),
            ("Call between 6:00 and 8:00 AM.", "Call between 6 and 8 am."),
            ("Open from 6:00 PM to 8:00.", "Open from 6 PM to 8."),
            ("The ratio is 5:30.", "The ratio is 5 to 30."),
        ] {
            assert!(
                !missing(original, edited).contains_key("number"),
                "{original} -> {edited}"
            );
        }
        for (original, edited, dropped) in [
            ("The ratio is 5:00.", "The ratio is 5.", "00"),
            ("Read John 5:30.", "Read John 5:31.", "30"),
            ("Start at 5:30:15.", "Start at 5:30:16.", "15"),
            // 'and' or 'to' after a number that is not a time makes no time.
            (
                "We offer a 3:15 and 4:00 slot.",
                "We offer a 3:15 and 4 slot.",
                "00",
            ),
            (
                "The ratio 2:30 to 3:15 held.",
                "The ratio 2:30 to 3:14 held.",
                "15",
            ),
        ] {
            assert_eq!(missing(original, edited)["number"], vec![dropped]);
        }
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
    fn a_name_is_kept_inside_marks_a_possessive_or_a_longer_name() {
        let kept = |edited: &str| names("We asked Priya Nair and Sean O'Brien.", edited);
        for edited in [
            "(Priya Nair) asked Sean O'Brien.",
            "Priya Nair\u{2019}s and Sean O'Brien's notes.",
            "Dr Priya Nair-Smith met Sean O'Brien.",
            "Ask Priya\n  Nair, then Mr Sean O'Brien.",
        ] {
            assert!(kept(edited).is_empty(), "{edited}: {:?}", kept(edited));
        }
        // A name split by a mark or grown into another word is not kept.
        assert_eq!(
            kept("Priya, Nair met Sean O'Briens."),
            vec!["Priya Nair", "Sean O'Brien"]
        );
        assert!(names("Met The Audit Committee.", "The Audit Committee met.").is_empty());
        assert_eq!(
            names("Met The Audit Committee.", "The Audit Committees met."),
            vec!["Audit Committee"]
        );
    }

    #[test]
    fn checks_many_names_in_linear_time() {
        // Each name was looked for in the whole edit, so 16 times the names took about 256
        // times as long; a set of the edit's word runs gives about 16 to 20 times.
        let fastest = |items: usize| {
            let text: String = (0..items).map(|n| format!("Alpha{n} Beta{n}. ")).collect();
            (0..3)
                .map(|_| {
                    let start = std::time::Instant::now();
                    assert!(names(&text, &text).is_empty());
                    start.elapsed()
                })
                .min()
                .unwrap()
        };
        let (small, large) = (fastest(2_000), fastest(32_000));
        assert!(
            large < small * 32,
            "16 times the names took {:.1} times as long ({small:?} to {large:?})",
            large.as_secs_f64() / small.as_secs_f64()
        );
    }

    #[test]
    fn a_bold_list_marker_numbers_the_layout_but_a_changed_figure_still_reports() {
        let original = "**1. Start early**\n\n**2. Keep it quiet**";
        let edited = "1. Start early\n2. Keep it quiet";
        assert!(missing(original, edited).is_empty());
        assert!(added(original, edited).is_empty());
        let closed = "**1.** Start early\n\n__2)__ Keep it quiet";
        assert!(missing(closed, edited).is_empty());
        assert!(added(closed, edited).is_empty());
        assert_eq!(
            missing("**1.** Pay $500 now", "1. Pay $400 now")["number"],
            vec!["$500"]
        );
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
    fn every_negative_contraction_counts_as_not() {
        for original in [
            "The build didn't pass.",
            "The fee wasn't paid.",
            "It won't ship.",
        ] {
            assert_eq!(
                missing(
                    original,
                    &original.replace("n't", "").replace("wo ", "will ")
                )["meaning word"],
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
    fn a_negative_contraction_keeps_its_modal() {
        for (original, edited) in [
            ("We can't pay.", "We cannot pay."),
            ("We can't pay.", "We can not pay."),
            ("We won't pay.", "We will not pay."),
            ("You shouldn't pay.", "You should not pay."),
        ] {
            for (a, b) in [(original, edited), (edited, original)] {
                assert!(missing(a, b).is_empty(), "{a} to {b}");
                assert!(added(a, b).is_empty(), "{a} to {b}");
            }
        }
        for (original, edited, lost, gained) in [
            ("We can't pay.", "We won't pay.", "can", "will"),
            ("We won't pay.", "We can't pay.", "will", "can"),
            ("We can't pay.", "We will not pay.", "can", "will"),
            ("We won't pay.", "We cannot pay.", "will", "can"),
        ] {
            assert_eq!(
                missing(original, edited)["meaning word"],
                vec![format!("'{lost}' (1 before, 0 after)")],
                "{original} to {edited}"
            );
            assert_eq!(
                added(original, edited)["meaning word"],
                vec![format!("'{gained}' (0 before, 1 after)")],
                "{original} to {edited}"
            );
        }
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
            ("Margin was (5%).", "Margin was -5%.", "5%"),
        ] {
            assert_eq!(
                missing(original, edited)["number"],
                vec![dropped],
                "{original}"
            );
        }
        assert_eq!(
            added("It costs $500.", "It costs £500.")["number"],
            vec!["£500"]
        );
        for (original, edited) in [
            ("Growth was \u{2212}3%.", "Growth was -3%."),
            ("A loss of (1,200).", "A loss of -1,200."),
            ("A loss of ($250).", "A loss of -$250."),
            ("It costs $ 27.99.", "It costs $27.99."),
            ("A loss of -$12,000.", "A loss of -$12000."),
            (
                "Read pages 2-3 on COVID-19.",
                "Read pages 2 to 3 on COVID 19.",
            ),
            ("See note (3).", "See note 3."),
            ("Margin was (5%).", "Margin was 5%."),
            (
                "Turnout was high (62%) this year.",
                "Turnout was high at 62% this year.",
            ),
            ("The ratio (2.5) held.", "The ratio 2.5 held."),
            ("Growth was (1,200%).", "Growth was 1,200%."),
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
            (
                "Store below 25 degrees Celsius.",
                "Store below 25 degrees Fahrenheit.",
                "25 °C",
            ),
            ("Take 5 mg daily.", "Take 5 g daily.", "5 mg"),
            ("Add 2 L of water.", "Add 2 mL of water.", "2 L"),
        ] {
            assert_eq!(
                missing(original, edited)["quantity"],
                vec![dropped],
                "{original}"
            );
        }
        for (original, edited) in [
            ("Latency is 200 ms.", "Latency is 200 milliseconds."),
            ("Take 5 g daily.", "Take 5 grams daily."),
            ("Store below 25 degrees Celsius.", "Store below 25 °C."),
            ("Add 2 L of water.", "Add 2 litres of water."),
        ] {
            assert!(missing(original, edited).is_empty(), "{original}");
        }
    }

    #[test]
    fn a_currency_word_matches_its_mark_and_every_mark_is_its_own() {
        for (original, edited) in [
            ("Revenue was $30 billion.", "Revenue was 30 billion dollar."),
            ("It costs $30.", "It costs 30 dollars."),
            ("It costs €5.", "It costs 5 euros."),
            ("It costs ¥500.", "It costs 500 yen."),
            ("It costs £5.", "It costs 5 pounds."),
            ("It weighs 10 lb.", "It weighs 10 pounds."),
            (
                "It costs £5 and needs 2 pounds of flour.",
                "It costs £5 and needs 2 lb of flour.",
            ),
            ("It costs $5-10.", "It costs 5-10 dollars."),
            ("It costs $5 to $10.", "It costs 5 to 10 dollars."),
            ("Prices run from $5 to 10.", "Prices run from $5 to $10."),
            (
                "Prices are between $5 and 10.",
                "Prices are between $5 and $10.",
            ),
            (
                "I donated $50 to 3 charities.",
                "I donated $50 each to 3 charities.",
            ),
            (
                "The fee is $5 and 2 forms are needed.",
                "The fee is $5; 2 forms are needed.",
            ),
            (
                "Prices run from $5 to 10 euros.",
                "Prices run from $5 to €10.",
            ),
            (
                "The fee is $5 and 10 kg of luggage.",
                "The fee is $5; 10 kg of luggage.",
            ),
            (
                "Revenue was $5 to 10 billion.",
                "Revenue was $5 billion to 10 billion.",
            ),
        ] {
            assert!(missing(original, edited).is_empty(), "{original}");
            assert!(added(original, edited).is_empty(), "{original}");
        }
        // A currency word in the next sentence names no currency.
        let next_sentence = missing(
            "Pay 30. Dollars are scarce.",
            "Pay 30 now. Dollars are scarce.",
        );
        assert!(next_sentence.is_empty(), "{next_sentence:?}");
        for (original, edited, dropped) in [
            ("It costs ₹500.", "It costs ₩500.", "₹500"),
            ("It costs $30.", "It costs 30 euros.", "$30"),
            ("It costs £5.", "It costs 5 lb.", "£5"),
            (
                "Prices run from $5 to 10 euros.",
                "Prices run from $5 to $10.",
                "€10",
            ),
        ] {
            assert_eq!(
                missing(original, edited)["number"],
                vec![dropped],
                "{original}"
            );
        }
    }

    #[test]
    fn both_bounds_of_a_range_carry_its_unit() {
        for (original, edited) in [
            ("Bake for 45 to 60 minutes.", "Bake for 45-60 minutes."),
            (
                "Bake for 45 to 60 minutes.",
                "Bake for 45\u{2013}60 minutes.",
            ),
            ("Add 30 to 60 grams of salt.", "Add 30-60 grams of salt."),
            ("Allow 2-3 hours.", "Allow between 2 and 3 hours."),
            ("Allow 2-3 days.", "Allow 2 or 3 days."),
            // A spaced dash, and a lower bound whose hyphen waits for the upper's unit.
            ("Add 30 to 60 grams.", "Add 30 \u{2014} 60 grams."),
            ("Add 30 to 60 grams.", "Add 30 \u{2013} 60 grams."),
            ("Add 30 to 60 grams.", "Add 30 - 60 grams."),
            ("Rides of 45 to 60 minutes.", "A 45- to 60-minute ride."),
        ] {
            for (from, to) in [(original, edited), (edited, original)] {
                assert!(missing(from, to).is_empty(), "{from}");
                assert!(added(from, to).is_empty(), "{from}");
            }
        }
        assert_eq!(
            missing("Bake for 45-60 minutes.", "Bake for 45-60 hours.")["quantity"],
            vec!["45 minute", "60 minute"]
        );
        assert_eq!(
            missing("Add 5-10 kg.", "Add 5 to 10 g.")["quantity"],
            vec!["5 kg", "10 kg"]
        );
        // A date or phone number is no range, so the word after it is no unit of its parts.
        let dated = added(
            "Due 2026-11-14. Call 0400-123-456.",
            "Due 2026-11-14 hours. Call 0400-123-456 minutes.",
        );
        assert!(dated.is_empty(), "{dated:?}");
    }

    #[test]
    fn a_long_chain_of_ranges_reads_one_step_ahead() {
        let chain = "1 to ".repeat(50_000) + "1 km";
        assert!(missing(&chain, &chain).is_empty());
    }

    #[test]
    fn compares_many_numbers_in_linear_time() {
        // Each number was looked for in a list of every kept one, so 16 times the numbers took
        // about 60 times as long in a debug build and 145 in a release build. Hash sets give
        // about 16 to 19 times; the fastest of 3 runs keeps a busy machine from deciding the ratio.
        let fastest = |items: usize| {
            let text: String = (0..items).map(|n| format!("{n} ")).collect();
            (0..3)
                .map(|_| {
                    let start = std::time::Instant::now();
                    assert_eq!(report(&text, &text, false).lines.len(), 2);
                    start.elapsed()
                })
                .min()
                .unwrap()
        };
        let (small, large) = (fastest(2_000), fastest(32_000));
        assert!(
            large < small * 32,
            "16 times the numbers took {:.1} times as long ({small:?} to {large:?})",
            large.as_secs_f64() / small.as_secs_f64()
        );
    }

    #[test]
    fn loose_lists_a_lost_frequency_quantifier_or_hedge_to_check_without_failing() {
        let (original, edited) = (
            "Users can often export some reports, and it will likely work.",
            "Users export reports, and it works.",
        );
        // By default each loss fails; under `--loose` each is listed to check.
        let strict = report(original, edited, false);
        assert!(strict.dropped, "{:?}", strict.lines);
        let loosened = report(original, edited, true);
        assert!(!loosened.dropped, "{:?}", loosened.lines);
        for word in ["some", "can", "will", "likely", "often"] {
            let line = format!("check meaning word: '{word}' (1 before, 0 after)");
            assert!(
                loosened.lines.contains(&line),
                "{line}: {:?}",
                loosened.lines
            );
        }
        // A swap passes, but its added word fails `--strict`.
        for (original, edited) in [
            ("All users qualify.", "Some users qualify."),
            ("We can't pay.", "We won't pay."),
        ] {
            let swapped = report(original, edited, true);
            assert!(!swapped.dropped && swapped.added, "{:?}", swapped.lines);
        }
        // Negation, 'every', 'only', order, obligation, possibility and bounds still fail.
        for (original, edited) in [
            ("Do not deploy.", "Deploy."),
            ("Check every file.", "Check files."),
            ("Only admins edit.", "Admins edit."),
            ("Pay before Friday.", "Pay Friday."),
            ("Staff must apply.", "Staff apply."),
            ("It may rain.", "It rains."),
            ("Up to 4 run at once.", "4 run at once."),
            ("It can often fail if not tested.", "It fails if tested."),
        ] {
            let failed = report(original, edited, true);
            assert!(failed.dropped, "{original} to {edited}: {:?}", failed.lines);
        }
        let mixed = report(
            "It can often fail if not tested.",
            "It fails if tested.",
            true,
        );
        assert_eq!(
            mixed.lines[..3],
            [
                "dropped meaning word: 'not' (1 before, 0 after)",
                "check meaning word: 'can' (1 before, 0 after)",
                "check meaning word: 'often' (1 before, 0 after)",
            ]
        );
    }

    #[test]
    fn a_name_only_the_edit_has_is_listed_and_never_fails() {
        let report = report(
            "The plan was approved.",
            "The plan was approved by Jane Smith.",
            false,
        );
        assert!(
            report
                .lines
                .contains(&"check added name: Jane Smith".to_string())
        );
        assert!(!report.dropped && !report.added);
    }
}
