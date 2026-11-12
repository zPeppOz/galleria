#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Directive {
        line: usize,
        name: String,
        args: Vec<String>,
    },
    Block {
        line: usize,
        tag: String,
        body: String,
    },
    ConnectionOpen {
        line: usize,
    },
    ConnectionClose {
        line: usize,
    },
}
impl Item {
    pub fn line(&self) -> usize {
        match self {
            Self::Directive { line, .. }
            | Self::Block { line, .. }
            | Self::ConnectionOpen { line }
            | Self::ConnectionClose { line } => *line,
        }
    }
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("Unterminated inline block at line {line}")]
    UnterminatedBlock { line: usize },
    #[error("Invalid configuration syntax at line {line}")]
    Syntax { line: usize },
}
/// OpenVPN quoting: single quotes are literal; double quotes and bare words use backslash escaping.
pub fn words(text: &str) -> Result<Vec<String>, ParseError> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut started = false;
    for c in text.chars() {
        if c == '\0' || c == '\r' || c == '\n' {
            return Err(ParseError::Syntax { line: 1 });
        }
        if escaped {
            word.push(c);
            escaped = false;
            started = true;
            continue;
        }
        if c == '\\' && quote != Some('\'') {
            escaped = true;
            started = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                word.push(c);
            }
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            started = true;
        } else if c.is_whitespace() {
            if started {
                words.push(std::mem::take(&mut word));
                started = false;
            }
        } else if (c == '#' || c == ';') && !started {
            break;
        } else {
            word.push(c);
            started = true;
        }
    }
    if escaped || quote.is_some() {
        return Err(ParseError::Syntax { line: 1 });
    }
    if started {
        words.push(word);
    }
    Ok(words)
}
pub fn parse(text: &str) -> Result<Vec<Item>, ParseError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n').enumerate();
    let mut items = Vec::new();
    let mut connection = false;
    while let Some((i, raw)) = lines.next() {
        let line = i + 1;
        if raw.len() > 255 || raw.contains('\0') {
            return Err(ParseError::Syntax { line });
        }
        let t = raw.trim();
        if t.is_empty() || t.starts_with(['#', ';']) {
            continue;
        }
        if t == "<connection>" {
            if connection {
                return Err(ParseError::Syntax { line });
            }
            connection = true;
            items.push(Item::ConnectionOpen { line });
            continue;
        }
        if t == "</connection>" {
            if !connection {
                return Err(ParseError::Syntax { line });
            }
            connection = false;
            items.push(Item::ConnectionClose { line });
            continue;
        }
        if let Some(tag) = t.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
            if tag.is_empty() || !tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err(ParseError::Syntax { line });
            }
            let close = format!("</{tag}>");
            let mut body = String::new();
            let mut closed = false;
            for (_, raw) in lines.by_ref() {
                if raw.len() > 255 || raw.contains('\0') {
                    return Err(ParseError::Syntax {
                        line: line + body.bytes().filter(|b| *b == b'\n').count() + 1,
                    });
                }
                // OpenVPN read_inline_file recognizes the closing tag as a prefix.
                if raw.trim_start().starts_with(&close) {
                    closed = true;
                    break;
                }
                body.push_str(raw);
            }
            if !closed {
                return Err(ParseError::UnterminatedBlock { line });
            }
            items.push(Item::Block {
                line,
                tag: tag.into(),
                body,
            });
            continue;
        }
        let mut tokens = words(raw.trim_end_matches(['\n', '\r']))
            .map_err(|_| ParseError::Syntax { line })?
            .into_iter();
        if let Some(name) = tokens.next() {
            let name = name.strip_prefix("--").unwrap_or(&name).to_owned();
            items.push(Item::Directive {
                line,
                name,
                args: tokens.collect(),
            });
        }
    }
    if connection {
        return Err(ParseError::UnterminatedBlock {
            line: items
                .iter()
                .find(|i| matches!(i, Item::ConnectionOpen { .. }))
                .map_or(1, Item::line),
        });
    }
    Ok(items)
}
pub fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
pub fn render(items: &[Item]) -> String {
    let mut out = String::new();
    let mut line = 1;
    for item in items {
        while line < item.line() {
            out.push('\n');
            line += 1;
        }
        let text = match item {
            Item::Directive { name, args, .. } => format!(
                "{}{}\n",
                name,
                args.iter()
                    .map(|a| format!(" {}", quote(a)))
                    .collect::<String>()
            ),
            Item::Block { tag, body, .. } => format!("<{tag}>\n{body}</{tag}>\n"),
            Item::ConnectionOpen { .. } => "<connection>\n".into(),
            Item::ConnectionClose { .. } => "</connection>\n".into(),
        };
        line += text.bytes().filter(|b| *b == b'\n').count();
        out.push_str(&text);
    }
    out
}
