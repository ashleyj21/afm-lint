// AFM CharMetrics lines look like:
//   C 32 ; WX 278 ; N space ;
// i.e. semicolon-separated "key value" pairs. This splits a line into
// those pairs and, critically, tracks the byte column each value starts
// at, so callers can point a diagnostic straight at it.

pub struct Field<'a> {
    pub key: &'a str,
    pub value: &'a str,
    pub value_col: usize,
}

pub fn parse_fields(line: &str) -> Option<Vec<Field>> {
    let mut fields = Vec::new();
    let mut byte_offset = 0usize;

    for raw_segment in line.split(';') {
        let segment_start = byte_offset;
        byte_offset += raw_segment.len() + 1; // +1 accounts for the ';' split() consumed

        let after_leading_ws = raw_segment.trim_start();
        if after_leading_ws.is_empty() {
            continue;
        }
        let leading_ws_len = raw_segment.len() - after_leading_ws.len();

        let key_end = after_leading_ws.find(char::is_whitespace)?;
        let key = &after_leading_ws[..key_end];

        let rest = &after_leading_ws[key_end..];
        let value_trimmed = rest.trim_start();
        let value_leading_ws = rest.len() - value_trimmed.len();
        let value = value_trimmed.trim_end();
        if value.is_empty() {
            return None;
        }

        let value_col = segment_start + leading_ws_len + key_end + value_leading_ws + 1;
        fields.push(Field { key, value, value_col });
    }

    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

// KernPairs lines look like:
//   KPX A Aacute -20
// i.e. plain whitespace-separated tokens, unlike the semicolon-joined
// CharMetrics rows above. This splits a line into tokens and records the
// 1-indexed byte column each one starts at.
pub struct Token<'a> {
    pub text: &'a str,
    pub col: usize,
}

pub fn parse_tokens(line: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = line.char_indices().peekable();

    while let Some(&(idx, ch)) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        let start = idx;
        let mut end = idx + ch.len_utf8();
        chars.next();

        while let Some(&(next_idx, next_ch)) = chars.peek() {
            if next_ch.is_whitespace() {
                break;
            }
            end = next_idx + next_ch.len_utf8();
            chars.next();
        }

        tokens.push(Token { text: &line[start..end], col: start + 1 });
    }

    tokens
}

// Composites lines look like:
//   CC Aacute 2 ; PCC A 0 0 ; PCC acute 130 0 ;
// i.e. semicolon-separated segments like CharMetrics, but each segment is
// itself a group of whitespace-separated tokens rather than a single key
// and value. This splits a line into those segments and tokenizes each
// one, keeping token columns absolute within the original line so callers
// can point a diagnostic at any individual piece name or number.
pub fn parse_semicolon_segments(line: &str) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut byte_offset = 0usize;

    for raw_segment in line.split(';') {
        let tokens: Vec<Token> = parse_tokens(raw_segment)
            .into_iter()
            .map(|t| Token { text: t.text, col: t.col + byte_offset })
            .collect();
        if !tokens.is_empty() {
            segments.push(tokens);
        }
        byte_offset += raw_segment.len() + 1; // +1 accounts for the ';' split() consumed
    }

    segments
}
