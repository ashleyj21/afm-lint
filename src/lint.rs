use crate::diagnostic::Diagnostic;
use crate::parser::parse_fields;
use std::collections::HashMap;

pub fn run(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut in_char_metrics = false;
    let mut expected_count: Option<(usize, usize, usize)> = None; // (line, col, declared count)
    let mut actual_count = 0usize;
    let mut seen_codes: HashMap<i64, usize> = HashMap::new();

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

        if !in_char_metrics || content.trim().is_empty() {
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
