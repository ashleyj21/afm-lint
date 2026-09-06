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
