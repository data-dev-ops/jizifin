//! RFC 4180 tokenizer: quotes, escaped quotes, embedded delimiters and newlines, BOM.

/// Splits CSV text into rows of fields.
pub struct Tokenizer {
    delimiter: char,
    rows: Vec<Vec<String>>,
    row: Vec<String>,
    field: String,
    quoted: bool,
}

impl Tokenizer {
    /// Tokenizer for `delimiter`.
    pub fn new(delimiter: char) -> Self {
        Self { delimiter, rows: Vec::new(), row: Vec::new(), field: String::new(), quoted: false }
    }

    /// Tokenizes `text`, skipping blank lines.
    pub fn rows(mut self, text: &str) -> Vec<Vec<String>> {
        let mut chars = strip_bom(text).chars().peekable();
        while let Some(c) = chars.next() {
            let escaped = self.quoted && c == '"' && chars.next_if_eq(&'"').is_some();
            let crlf = !self.quoted && c == '\r' && chars.next_if_eq(&'\n').is_some();
            self.feed(if crlf { '\n' } else { c }, escaped);
        }
        self.end_row();
        self.rows
    }

    fn feed(&mut self, c: char, escaped: bool) {
        match (self.quoted, c) {
            (_, '"') if escaped => self.field.push('"'),
            (_, '"') => self.quoted = !self.quoted,
            (false, '\n' | '\r') => self.end_row(),
            (false, d) if d == self.delimiter => self.end_field(),
            _ => self.field.push(c),
        }
    }

    fn end_field(&mut self) {
        self.row.push(std::mem::take(&mut self.field));
    }

    fn end_row(&mut self) {
        self.end_field();
        let row = std::mem::take(&mut self.row);
        if row.len() > 1 || !row[0].is_empty() {
            self.rows.push(row);
        }
    }
}

fn strip_bom(text: &str) -> &str {
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    text.strip_prefix("ï»¿").unwrap_or(text)
}
