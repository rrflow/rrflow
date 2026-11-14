//! Parsing entry points, parse errors, tokens and the tokenizer.

use super::*;

pub fn parse_transaction_program(input: &str) -> Result<TransactionProgram, ParseError> {
    if input.is_empty() || input.len() > MAX_TRANSACTION_PROGRAM_BYTES {
        return Err(ParseError::new(
            0,
            format!(
                "transaction program length must be in 1..={MAX_TRANSACTION_PROGRAM_BYTES} bytes"
            ),
        ));
    }
    let mut statements = input.split(';').map(str::trim).collect::<Vec<_>>();
    if statements.last() == Some(&"") {
        statements.pop();
    }
    if statements.iter().any(|statement| statement.is_empty()) || statements.len() < 3 {
        return Err(ParseError::new(
            0,
            "transaction program requires BEGIN, mutation statements, and COMMIT or CANCEL",
        ));
    }
    if !statements[0].eq_ignore_ascii_case("BEGIN") {
        return Err(ParseError::new(
            0,
            "transaction program must start with BEGIN",
        ));
    }
    let disposition = if statements
        .last()
        .is_some_and(|statement| statement.eq_ignore_ascii_case("COMMIT"))
    {
        TransactionDisposition::Commit
    } else if statements
        .last()
        .is_some_and(|statement| statement.eq_ignore_ascii_case("CANCEL"))
    {
        TransactionDisposition::Cancel
    } else {
        return Err(ParseError::new(
            input.len(),
            "transaction program must end with COMMIT or CANCEL",
        ));
    };
    let mut mutation_bindings = Vec::with_capacity(statements.len() - 2);
    for statement in &statements[1..statements.len() - 1] {
        let words = statement.split_whitespace().collect::<Vec<_>>();
        if words.len() != 2 || !words[0].eq_ignore_ascii_case("MUTATE") {
            return Err(ParseError::new(
                input.find(statement).unwrap_or(0),
                "transaction statements must be MUTATE $binding",
            ));
        }
        mutation_bindings.push(parameter(words[1], input.find(words[1]).unwrap_or(0))?);
    }
    let program = TransactionProgram {
        contract_version: QUERY_CONTRACT_VERSION,
        mutation_bindings,
        disposition,
    };
    program.validate()?;
    Ok(program)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl ParseError {
    pub(super) fn new(offset: usize, message: impl Into<String>) -> Self {
        Self {
            offset,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "query parse error at byte {}: {}",
            self.offset, self.message
        )
    }
}

impl std::error::Error for ParseError {}

pub fn parse(input: &str) -> Result<Query, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens, cursor: 0 };
    let query = parser.query()?;
    if let Some(token) = parser.peek() {
        return Err(ParseError::new(
            token.offset,
            format!("unexpected token {}", token.describe()),
        ));
    }
    query.validate()?;
    Ok(query)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Token {
    pub(super) offset: usize,
    pub(super) kind: TokenKind,
}

impl Token {
    pub(super) fn describe(&self) -> String {
        match &self.kind {
            TokenKind::Word(value) => format!("{value:?}"),
            TokenKind::String(value) => format!("string {value:?}"),
            TokenKind::Number(value) => value.to_string(),
            TokenKind::Colon => "':'".into(),
            TokenKind::Comma => "','".into(),
            TokenKind::Equals => "'='".into(),
            TokenKind::NotEquals => "'!='".into(),
            TokenKind::LessThan => "'<'".into(),
            TokenKind::LessThanOrEqual => "'<='".into(),
            TokenKind::GreaterThan => "'>'".into(),
            TokenKind::GreaterThanOrEqual => "'>='".into(),
            TokenKind::Star => "'*'".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TokenKind {
    Word(String),
    String(String),
    Number(u64),
    Colon,
    Comma,
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Star,
}

pub(super) fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        let offset = cursor;
        let kind = match bytes[cursor] {
            b':' => {
                cursor += 1;
                TokenKind::Colon
            }
            b',' => {
                cursor += 1;
                TokenKind::Comma
            }
            b'=' => {
                cursor += 1;
                TokenKind::Equals
            }
            b'!' if bytes.get(cursor + 1) == Some(&b'=') => {
                cursor += 2;
                TokenKind::NotEquals
            }
            b'<' => {
                cursor += 1;
                if bytes.get(cursor) == Some(&b'=') {
                    cursor += 1;
                    TokenKind::LessThanOrEqual
                } else {
                    TokenKind::LessThan
                }
            }
            b'>' => {
                cursor += 1;
                if bytes.get(cursor) == Some(&b'=') {
                    cursor += 1;
                    TokenKind::GreaterThanOrEqual
                } else {
                    TokenKind::GreaterThan
                }
            }
            b'*' => {
                cursor += 1;
                TokenKind::Star
            }
            b'"' => {
                cursor += 1;
                let mut value = String::new();
                let mut closed = false;
                while cursor < bytes.len() {
                    match bytes[cursor] {
                        b'"' => {
                            cursor += 1;
                            closed = true;
                            break;
                        }
                        b'\\' => {
                            cursor += 1;
                            let escaped = *bytes.get(cursor).ok_or_else(|| {
                                ParseError::new(offset, "unterminated string escape")
                            })?;
                            value.push(match escaped {
                                b'"' => '"',
                                b'\\' => '\\',
                                b'n' => '\n',
                                b'r' => '\r',
                                b't' => '\t',
                                _ => {
                                    return Err(ParseError::new(
                                        cursor,
                                        "unsupported string escape",
                                    ))
                                }
                            });
                            cursor += 1;
                        }
                        byte if byte.is_ascii() => {
                            value.push(byte as char);
                            cursor += 1;
                        }
                        _ => {
                            let rest = &input[cursor..];
                            let character = rest.chars().next().expect("non-empty UTF-8 tail");
                            value.push(character);
                            cursor += character.len_utf8();
                        }
                    }
                }
                if !closed {
                    return Err(ParseError::new(offset, "unterminated string literal"));
                }
                TokenKind::String(value)
            }
            byte if byte.is_ascii_digit() => {
                while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                    cursor += 1;
                }
                let value = input[offset..cursor]
                    .parse()
                    .map_err(|_| ParseError::new(offset, "integer is outside u64 range"))?;
                TokenKind::Number(value)
            }
            _ => {
                while cursor < bytes.len()
                    && !bytes[cursor].is_ascii_whitespace()
                    && !matches!(
                        bytes[cursor],
                        b':' | b',' | b'=' | b'!' | b'<' | b'>' | b'*' | b'"'
                    )
                {
                    cursor += 1;
                }
                if cursor == offset {
                    return Err(ParseError::new(offset, "unsupported query character"));
                }
                TokenKind::Word(input[offset..cursor].to_owned())
            }
        };
        tokens.push(Token { offset, kind });
    }
    Ok(tokens)
}
