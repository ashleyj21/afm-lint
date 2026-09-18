use crate::diagnostic::Diagnostic;
use crate::parser::{parse_fields, parse_tokens};
use std::collections::HashMap;

const REQUIRED_HEADER_KEYS: &[&str] = &["FontName", "FontBBox", "Ascender", "Descender"];

pub fn run(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = check_header(source);
    let mut in_char_metrics = false;
    let mut expected_count: Option<(usize, usize, usize)> = None; // (line, col, declared count)
    let mut actual_count = 0usize;
    let mut seen_codes: HashMap<i64, usize> = HashMap::new();
    let mut known_glyph_names: HashMap<String, usize> = HashMap::new();

    let mut in_kern_pairs = false;
    let mut expected_kern_count: Option<(usize, usize, usize)> = None;
    let mut actual_kern_count = 0usize;
    let mut seen_kern_pairs: HashMap<(String, String), usize> = HashMap::new();

    for (idx, line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let leading_ws = line.len() - line.trim_start().len();
        let content = &line[leading_ws..];

        if content.starts_with("StartCharMetrics") {
            in_char_metrics = true;
            actual_count = 0;
            seen_codes.clear();
            expected_count = locate_value(content, "StartCharMetrics", leading_ws)
                .and_then(|(col, value)| value.parse::<usize>().ok().map(|count| (line_no, col, count)));
            continue;
        }

        if content.starts_with("EndCharMetrics") {
            in_char_metrics = false;
            if let Some((exp_line, exp_col, expected)) = expected_count.take() {
                if expected != actual_count {
                    diagnostics.push(Diagnostic::new(
                        "char-count-mismatch",
                        format!(
                            "StartCharMetrics declares {} characters but {} were found before EndCharMetrics",
                            expected, actual_count
                        ),
                        exp_line,
                        exp_col,
                        expected.to_string().len(),
                    ));
                }
            }
            continue;
        }

        // Only matches the direction-agnostic StartKernPairs/EndKernPairs
        // pair, not the StartKernPairs0/StartKernPairs1 variants used for
        // per-direction kerning; those fall through unrecognized for now.
        if is_keyword_line(content, "StartKernPairs") {
            in_kern_pairs = true;
            actual_kern_count = 0;
            seen_kern_pairs.clear();
            expected_kern_count = locate_value(content, "StartKernPairs", leading_ws)
                .and_then(|(col, value)| value.parse::<usize>().ok().map(|count| (line_no, col, count)));
            continue;
        }

        if is_keyword_line(content, "EndKernPairs") {
            in_kern_pairs = false;
            if let Some((exp_line, exp_col, expected)) = expected_kern_count.take() {
                if expected != actual_kern_count {
                    diagnostics.push(Diagnostic::new(
                        "kern-count-mismatch",
                        format!(
                            "StartKernPairs declares {} pairs but {} were found before EndKernPairs",
                            expected, actual_kern_count
                        ),
                        exp_line,
                        exp_col,
                        expected.to_string().len(),
                    ));
                }
            }
            continue;
        }

        if in_char_metrics {
            if content.trim().is_empty() {
                continue;
            }

            let fields = match parse_fields(line) {
                Some(fields) => fields,
                None => {
                    diagnostics.push(Diagnostic::new(
                        "malformed-char-line",
                        "expected 'key value ;' pairs inside the CharMetrics table".to_string(),
                        line_no,
                        1,
                        line.trim_end().len().max(1),
                    ));
                    continue;
                }
            };

            actual_count += 1;

            if let Some(field) = fields.iter().find(|f| f.key == "C") {
                if let Ok(code) = field.value.parse::<i64>() {
                    if let Some(&first_line) = seen_codes.get(&code) {
                        diagnostics.push(Diagnostic::new(
                            "duplicate-code",
                            format!("character code {} already defined on line {}", code, first_line),
                            line_no,
                            field.value_col,
                            field.value.len(),
                        ));
                    } else {
                        seen_codes.insert(code, line_no);
                    }
                }
            }

            if let Some(field) = fields.iter().find(|f| f.key == "WX") {
                if let Ok(width) = field.value.parse::<f64>() {
                    if width < 0.0 {
                        diagnostics.push(
                            Diagnostic::new(
                                "negative-width",
                                format!("advance width WX {} is negative", field.value),
                                line_no,
                                field.value_col,
                                field.value.len(),
                            )
                            .with_note("width must be >= 0".to_string()),
                        );
                    }
                }
            }

            if let Some(field) = fields.iter().find(|f| f.key == "N") {
                known_glyph_names.entry(field.value.to_string()).or_insert(line_no);
            }

            continue;
        }

        if in_kern_pairs {
            if content.trim().is_empty() {
                continue;
            }

            let tokens = parse_tokens(line);
            if tokens.len() != 4 || tokens[0].text != "KPX" {
                diagnostics.push(Diagnostic::new(
                    "malformed-kern-pair",
                    "expected 'KPX name1 name2 amount' inside the KernPairs table".to_string(),
                    line_no,
                    1,
                    line.trim_end().len().max(1),
                ));
                continue;
            }

            actual_kern_count += 1;

            let name1 = tokens[1].text;
            let name2 = tokens[2].text;
            let amount = &tokens[3];

            if amount.text.parse::<f64>().is_err() {
                diagnostics.push(Diagnostic::new(
                    "malformed-kern-value",
                    format!("kerning amount '{}' is not a number", amount.text),
                    line_no,
                    amount.col,
                    amount.text.len(),
                ));
            }

            let pair_key = (name1.to_string(), name2.to_string());
            if let Some(&first_line) = seen_kern_pairs.get(&pair_key) {
                diagnostics.push(Diagnostic::new(
                    "duplicate-kern-pair",
                    format!("kerning pair {} {} already defined on line {}", name1, name2, first_line),
                    line_no,
                    tokens[1].col,
                    name1.len(),
                ));
            } else {
                seen_kern_pairs.insert(pair_key, line_no);
            }

            // known_glyph_names is only populated if CharMetrics came first,
            // which is the order the AFM spec requires; skip the check
            // rather than flag every pair when we have nothing to compare.
            if !known_glyph_names.is_empty() {
                if !known_glyph_names.contains_key(name1) {
                    diagnostics.push(Diagnostic::new(
                        "undefined-kern-glyph",
                        format!("kerning pair references undefined glyph name '{}'", name1),
                        line_no,
                        tokens[1].col,
                        name1.len(),
                    ));
                }
                if !known_glyph_names.contains_key(name2) {
                    diagnostics.push(Diagnostic::new(
                        "undefined-kern-glyph",
                        format!("kerning pair references undefined glyph name '{}'", name2),
                        line_no,
                        tokens[2].col,
                        name2.len(),
                    ));
                }
            }
        }
    }

    diagnostics
}

// True if `content` is exactly `keyword`, or starts with `keyword` followed
// by whitespace (its argument) - e.g. matches "StartKernPairs" against
// "StartKernPairs 315" but not against "StartKernPairs0 12".
fn is_keyword_line(content: &str, keyword: &str) -> bool {
    match content.strip_prefix(keyword) {
        Some(rest) => rest.is_empty() || rest.starts_with(char::is_whitespace),
        None => false,
    }
}

// Header keys (FontName, FontBBox, Ascender, Descender, ...) sit one per
// line before StartCharMetrics as "Key value", unlike the semicolon-joined
// CharMetrics rows, so they get their own pass with their own column math.
fn check_header(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut found: HashMap<&str, (usize, usize, String)> = HashMap::new();
    let mut start_line = 1usize;

    for (idx, line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let leading_ws = line.len() - line.trim_start().len();
        let content = &line[leading_ws..];

        if content.starts_with("StartFontMetrics") {
            start_line = line_no;
        }

        if content.starts_with("StartCharMetrics") {
            break;
        }

        for &key in REQUIRED_HEADER_KEYS {
            let starts_with_key = content
                .strip_prefix(key)
                .is_some_and(|rest| rest.starts_with(char::is_whitespace));
            if starts_with_key {
                if let Some((col, value)) = locate_value(content, key, leading_ws) {
                    found.entry(key).or_insert((line_no, col, value.to_string()));
                }
            }
        }
    }

    for &key in REQUIRED_HEADER_KEYS {
        if !found.contains_key(key) {
            diagnostics.push(Diagnostic::new(
                "missing-header-key",
                format!("required header key '{}' was not found before StartCharMetrics", key),
                start_line,
                1,
                1,
            ));
        }
    }

    if let Some((line_no, col, value)) = found.get("FontBBox") {
        let numbers: Vec<&str> = value.split_whitespace().collect();
        let span = value.len().max(1);
        if numbers.len() != 4 {
            diagnostics.push(Diagnostic::new(
                "malformed-font-bbox",
                format!(
                    "FontBBox expects 4 numbers (llx lly urx ury), found {}",
                    numbers.len()
                ),
                *line_no,
                *col,
                span,
            ));
        } else {
            match numbers
                .iter()
                .map(|n| n.parse::<f64>())
                .collect::<Result<Vec<f64>, _>>()
            {
                Err(_) => diagnostics.push(Diagnostic::new(
                    "malformed-font-bbox",
                    "FontBBox values must all be numbers".to_string(),
                    *line_no,
                    *col,
                    span,
                )),
                Ok(nums) => {
                    let (llx, lly, urx, ury) = (nums[0], nums[1], nums[2], nums[3]);
                    if llx > urx || lly > ury {
                        diagnostics.push(Diagnostic::new(
                            "inverted-font-bbox",
                            format!(
                                "FontBBox lower-left ({}, {}) is not below/left of upper-right ({}, {})",
                                llx, lly, urx, ury
                            ),
                            *line_no,
                            *col,
                            span,
                        ));
                    }
                }
            }
        }
    }

    if let Some((line_no, col, value)) = found.get("Ascender") {
        let span = value.len().max(1);
        match value.parse::<f64>() {
            Err(_) => diagnostics.push(Diagnostic::new(
                "malformed-ascender",
                format!("Ascender value '{}' is not a number", value),
                *line_no,
                *col,
                span,
            )),
            Ok(ascender) if ascender < 0.0 => diagnostics.push(
                Diagnostic::new(
                    "ascender-sign",
                    format!("Ascender {} is negative", value),
                    *line_no,
                    *col,
                    span,
                )
                .with_note("ascender is conventionally >= 0".to_string()),
            ),
            Ok(_) => {}
        }
    }

    if let Some((line_no, col, value)) = found.get("Descender") {
        let span = value.len().max(1);
        match value.parse::<f64>() {
            Err(_) => diagnostics.push(Diagnostic::new(
                "malformed-descender",
                format!("Descender value '{}' is not a number", value),
                *line_no,
                *col,
                span,
            )),
            Ok(descender) if descender > 0.0 => diagnostics.push(
                Diagnostic::new(
                    "descender-sign",
                    format!("Descender {} is positive", value),
                    *line_no,
                    *col,
                    span,
                )
                .with_note("descender is conventionally <= 0".to_string()),
            ),
            Ok(_) => {}
        }
    }

    diagnostics
}

// Finds `key`'s value inside a line that starts with `key`, e.g. pulls
// "315" out of "StartCharMetrics 315", and returns its 1-indexed column
// in the original (untrimmed) line.
fn locate_value<'a>(content: &'a str, key: &str, leading_ws: usize) -> Option<(usize, &'a str)> {
    let rest = content.strip_prefix(key)?;
    let trimmed = rest.trim_start();
    let ws_len = rest.len() - trimmed.len();
    let col = leading_ws + key.len() + ws_len + 1;
    let value = trimmed.trim_end();
    if value.is_empty() {
        None
    } else {
        Some((col, value))
    }
}
