// One diagnostic, rendered the way a compiler would: file:line:col, the
// offending source line, and a caret under the exact span that's wrong.
// The goal is that you never have to open the file to know what to fix.

pub struct Diagnostic {
    pub rule: &'static str,
    pub message: String,
    pub line: usize,
    pub col: usize,
    pub span_len: usize,
    pub note: Option<String>,
}

impl Diagnostic {
    pub fn new(rule: &'static str, message: String, line: usize, col: usize, span_len: usize) -> Self {
        Diagnostic {
            rule,
            message,
            line,
            col,
            span_len,
            note: None,
        }
    }

    pub fn with_note(mut self, note: String) -> Self {
        self.note = Some(note);
        self
    }

    pub fn render(&self, path: &str, source_line: &str) -> String {
        let line_label = self.line.to_string();
        let gutter = line_label.len();
        let blank_gutter = " ".repeat(gutter);

        let mut out = String::new();
        out.push_str(&format!("error[{}]: {}\n", self.rule, self.message));
        out.push_str(&format!(
            "{}--> {}:{}:{}\n",
            " ".repeat(gutter + 1),
            path,
            self.line,
            self.col
        ));
        out.push_str(&format!("{} |\n", blank_gutter));
        out.push_str(&format!("{} | {}\n", line_label, source_line));

        let pad = " ".repeat(self.col.saturating_sub(1));
        let caret = "^".repeat(self.span_len.max(1));
        out.push_str(&format!("{} | {}{}", blank_gutter, pad, caret));
        if let Some(note) = &self.note {
            out.push(' ');
            out.push_str(note);
        }
        out.push('\n');
        out
    }
}
