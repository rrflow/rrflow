//! The recursive-descent parser and its helpers.

use super::*;

pub(super) struct Parser {
    pub(super) tokens: Vec<Token>,
    pub(super) cursor: usize,
}

impl Parser {
    pub(super) fn query(&mut self) -> Result<Query, ParseError> {
        // SELECT is a second spelling of the existing relational projection,
        // not a separate binder, executor, or PostgreSQL compatibility claim.
        let selected_projection = if self.peek().is_some_and(|token| token.is_keyword("SELECT")) {
            self.cursor += 1;
            Some(self.projection()?)
        } else {
            None
        };
        self.keyword("FROM")?;
        let source = self.source()?;
        let join = if self.peek().is_some_and(|token| token.is_keyword("JOIN")) {
            self.cursor += 1;
            let source = self.source()?;
            self.keyword("ON")?;
            let left_field = self.word("left join field")?;
            validate_field(&left_field, self.previous_offset())?;
            self.punctuation(TokenKind::Equals, "'=' in join condition")?;
            let right_field = self.word("right join field")?;
            validate_field(&right_field, self.previous_offset())?;
            Some(Join {
                source,
                left_field,
                right_field,
            })
        } else {
            None
        };
        if selected_projection.is_some()
            && (matches!(source, Source::Traversal { .. })
                || join
                    .as_ref()
                    .is_some_and(|join| matches!(join.source, Source::Traversal { .. })))
        {
            return Err(ParseError::new(
                0,
                "SELECT requires relational sources; traversal is not a table source",
            ));
        }
        self.keyword("AT")?;
        self.keyword("VALID")?;
        let valid_at = self.time_expr()?;
        self.keyword("KNOWN")?;
        let known_at = self.cursor_expr()?;
        let mut query = Query::new(source, TemporalSelector { valid_at, known_at });
        query.join = join;
        let mut saw_where = false;
        let mut saw_project = selected_projection.is_some();
        if let Some(projection) = selected_projection {
            query.projection = projection;
        }
        let mut saw_limit = false;
        let mut saw_explain = false;
        while let Some(token) = self.peek() {
            if token.is_keyword("WHERE") {
                if saw_where {
                    return Err(ParseError::new(token.offset, "duplicate WHERE clause"));
                }
                saw_where = true;
                self.cursor += 1;
                query.filters = self.filters()?;
            } else if token.is_keyword("PROJECT") {
                if saw_project {
                    return Err(ParseError::new(token.offset, "duplicate PROJECT clause"));
                }
                saw_project = true;
                self.cursor += 1;
                query.projection = self.projection()?;
            } else if token.is_keyword("LIMIT") {
                if saw_limit {
                    return Err(ParseError::new(token.offset, "duplicate LIMIT clause"));
                }
                saw_limit = true;
                let clause_offset = token.offset;
                self.cursor += 1;
                let value = self.number("row limit")?;
                query.limit = Some(usize::try_from(value).map_err(|_| {
                    ParseError::new(clause_offset, "row limit exceeds this platform's capacity")
                })?);
            } else if token.is_keyword("EXPLAIN") {
                if saw_explain {
                    return Err(ParseError::new(token.offset, "duplicate EXPLAIN clause"));
                }
                saw_explain = true;
                self.cursor += 1;
                if self
                    .peek()
                    .is_some_and(|token| token.is_keyword("CONTRACT"))
                {
                    self.cursor += 1;
                    query.explain_contract = true;
                } else if self.peek().is_some_and(|token| token.is_keyword("ANALYZE")) {
                    self.cursor += 1;
                    query.explain_analyze = true;
                } else {
                    return Err(ParseError::new(
                        self.peek().map_or_else(
                            || self.tokens.last().map_or(0, |token| token.offset + 1),
                            |token| token.offset,
                        ),
                        "EXPLAIN requires CONTRACT or ANALYZE",
                    ));
                }
            } else {
                break;
            }
        }
        Ok(query)
    }

    fn source(&mut self) -> Result<Source, ParseError> {
        let token = self.next("source family")?;
        let TokenKind::Word(family) = &token.kind else {
            return Err(ParseError::new(token.offset, "expected source family"));
        };
        let family = family.to_ascii_lowercase();
        if family == "claim" && !self.peek_is(TokenKind::Colon) {
            return Ok(Source::Claim { predicate: None });
        }
        self.punctuation(TokenKind::Colon, "':' after source family")?;
        let name = self.word("source type")?;
        match family.as_str() {
            "record" => RuntimeType::new(name)
                .map(|kind| Source::Record { kind })
                .map_err(|error| ParseError::new(token.offset, error.to_string())),
            "relation" => RuntimeType::new(name)
                .map(|kind| Source::Relation { kind })
                .map_err(|error| ParseError::new(token.offset, error.to_string())),
            "event" => RuntimeType::new(name)
                .map(|kind| Source::Event { kind })
                .map_err(|error| ParseError::new(token.offset, error.to_string())),
            "series" => RuntimeType::new(name)
                .map(|kind| Source::Series { kind })
                .map_err(|error| ParseError::new(token.offset, error.to_string())),
            "geo" => RuntimeType::new(name)
                .map(|kind| Source::Geo { kind })
                .map_err(|error| ParseError::new(token.offset, error.to_string())),
            "traverse" => self.traversal_source(name, token.offset),
            "claim" => Predicate::new(name)
                .map(|predicate| Source::Claim {
                    predicate: Some(predicate),
                })
                .map_err(|error| ParseError::new(token.offset, error.to_string())),
            _ => Err(ParseError::new(
                token.offset,
                format!("unknown source family {family:?}"),
            )),
        }
    }

    fn traversal_source(&mut self, relation: String, offset: usize) -> Result<Source, ParseError> {
        let relation = RuntimeType::new(relation)
            .map_err(|error| ParseError::new(offset, error.to_string()))?;
        self.keyword("START")?;
        let start_kind = self.word("traversal start kind")?;
        self.punctuation(TokenKind::Colon, "':' in traversal start")?;
        let start_id = self.traversal_id()?;
        let start = RuntimeRef::parse(start_kind, start_id)
            .map_err(|error| ParseError::new(offset, error.to_string()))?;
        self.keyword("DIRECTION")?;
        let direction_token = self.next("traversal direction")?;
        let TokenKind::Word(direction) = &direction_token.kind else {
            return Err(ParseError::new(
                direction_token.offset,
                "expected OUTGOING, INCOMING, or BOTH",
            ));
        };
        let direction = match direction.to_ascii_lowercase().as_str() {
            "outgoing" => TraversalDirection::Outgoing,
            "incoming" => TraversalDirection::Incoming,
            "both" => TraversalDirection::Both,
            _ => {
                return Err(ParseError::new(
                    direction_token.offset,
                    "expected OUTGOING, INCOMING, or BOTH",
                ))
            }
        };
        self.keyword("DEPTH")?;
        let depth_offset = self.peek().map_or(offset, |token| token.offset);
        let depth = self.number("traversal depth")?;
        let max_depth = u16::try_from(depth).map_err(|_| {
            ParseError::new(
                depth_offset,
                format!("traversal depth must be in 1..={MAX_TRAVERSAL_DEPTH}"),
            )
        })?;
        if !(1..=MAX_TRAVERSAL_DEPTH).contains(&max_depth) {
            return Err(ParseError::new(
                depth_offset,
                format!("traversal depth must be in 1..={MAX_TRAVERSAL_DEPTH}"),
            ));
        }
        Ok(Source::Traversal {
            relation,
            start,
            direction,
            max_depth,
        })
    }

    fn time_expr(&mut self) -> Result<TimeExpr, ParseError> {
        let token = self.next("valid-time literal or parameter")?;
        match &token.kind {
            TokenKind::Number(value) => Ok(TimeExpr::Literal(*value)),
            TokenKind::Word(value) => parameter(value, token.offset).map(TimeExpr::Parameter),
            _ => Err(ParseError::new(
                token.offset,
                "expected valid-time literal or $parameter",
            )),
        }
    }

    fn cursor_expr(&mut self) -> Result<CursorExpr, ParseError> {
        let token = self.next("known cursor")?;
        match &token.kind {
            TokenKind::Number(value) => Ok(CursorExpr::Literal(*value)),
            TokenKind::Word(value) if value.eq_ignore_ascii_case("HEAD") => Ok(CursorExpr::Head),
            TokenKind::Word(value) => parameter(value, token.offset).map(CursorExpr::Parameter),
            _ => Err(ParseError::new(
                token.offset,
                "expected cursor, HEAD, or $parameter",
            )),
        }
    }

    fn filters(&mut self) -> Result<Vec<Filter>, ParseError> {
        let mut filters = Vec::new();
        loop {
            let field = self.word("filter field")?;
            validate_field(&field, self.previous_offset())?;
            let comparison = self.comparison()?;
            let value = self.value_expr()?;
            filters.push(Filter {
                field,
                comparison,
                value,
            });
            if self.peek().is_some_and(|token| token.is_keyword("AND")) {
                self.cursor += 1;
            } else {
                break;
            }
        }
        Ok(filters)
    }

    fn comparison(&mut self) -> Result<ComparisonOperator, ParseError> {
        let token = self.next("comparison operator")?;
        match token.kind {
            TokenKind::Equals => Ok(ComparisonOperator::Equal),
            TokenKind::NotEquals => Ok(ComparisonOperator::NotEqual),
            TokenKind::LessThan => Ok(ComparisonOperator::LessThan),
            TokenKind::LessThanOrEqual => Ok(ComparisonOperator::LessThanOrEqual),
            TokenKind::GreaterThan => Ok(ComparisonOperator::GreaterThan),
            TokenKind::GreaterThanOrEqual => Ok(ComparisonOperator::GreaterThanOrEqual),
            TokenKind::Word(value) if value.eq_ignore_ascii_case("MATCH") => {
                Ok(ComparisonOperator::Match)
            }
            _ => Err(ParseError::new(
                token.offset,
                format!("expected comparison operator, found {}", token.describe()),
            )),
        }
    }

    fn value_expr(&mut self) -> Result<ValueExpr, ParseError> {
        let token = self.next("filter value")?;
        match &token.kind {
            TokenKind::String(value) => Ok(ValueExpr::Literal(RuntimeValue::String(value.clone()))),
            TokenKind::Number(value) => Ok(ValueExpr::Literal(RuntimeValue::Unsigned(*value))),
            TokenKind::Word(value) if value.starts_with('$') => {
                parameter(value, token.offset).map(ValueExpr::Parameter)
            }
            TokenKind::Word(value) if value.eq_ignore_ascii_case("true") => {
                Ok(ValueExpr::Literal(RuntimeValue::Bool(true)))
            }
            TokenKind::Word(value) if value.eq_ignore_ascii_case("false") => {
                Ok(ValueExpr::Literal(RuntimeValue::Bool(false)))
            }
            TokenKind::Word(value) if value.eq_ignore_ascii_case("null") => {
                Ok(ValueExpr::Literal(RuntimeValue::Null))
            }
            _ => Err(ParseError::new(
                token.offset,
                "expected string, unsigned integer, boolean, null, or $parameter",
            )),
        }
    }

    fn projection(&mut self) -> Result<Projection, ParseError> {
        if self.peek_is(TokenKind::Star) {
            self.cursor += 1;
            return Ok(Projection::All);
        }
        let mut fields = Vec::new();
        loop {
            let field = self.word("projection field")?;
            validate_field(&field, self.previous_offset())?;
            fields.push(field);
            if self.peek_is(TokenKind::Comma) {
                self.cursor += 1;
            } else {
                break;
            }
        }
        Ok(Projection::Fields(fields))
    }

    fn keyword(&mut self, expected: &str) -> Result<(), ParseError> {
        let token = self.next(expected)?;
        if token.is_keyword(expected) {
            Ok(())
        } else {
            Err(ParseError::new(
                token.offset,
                format!("expected {expected}, found {}", token.describe()),
            ))
        }
    }

    fn punctuation(&mut self, expected: TokenKind, description: &str) -> Result<(), ParseError> {
        let token = self.next(description)?;
        if token.kind == expected {
            Ok(())
        } else {
            Err(ParseError::new(
                token.offset,
                format!("expected {description}, found {}", token.describe()),
            ))
        }
    }

    fn word(&mut self, description: &str) -> Result<String, ParseError> {
        let token = self.next(description)?;
        match &token.kind {
            TokenKind::Word(value) => Ok(value.clone()),
            _ => Err(ParseError::new(
                token.offset,
                format!("expected {description}, found {}", token.describe()),
            )),
        }
    }

    fn traversal_id(&mut self) -> Result<String, ParseError> {
        let token = self.next("traversal start ID")?;
        match &token.kind {
            TokenKind::Word(value) | TokenKind::String(value) => Ok(value.clone()),
            _ => Err(ParseError::new(
                token.offset,
                format!("expected traversal start ID, found {}", token.describe()),
            )),
        }
    }

    fn number(&mut self, description: &str) -> Result<u64, ParseError> {
        let token = self.next(description)?;
        match token.kind {
            TokenKind::Number(value) => Ok(value),
            _ => Err(ParseError::new(
                token.offset,
                format!("expected {description}, found {}", token.describe()),
            )),
        }
    }

    fn next(&mut self, expected: &str) -> Result<Token, ParseError> {
        let token = self.tokens.get(self.cursor).cloned().ok_or_else(|| {
            ParseError::new(
                self.tokens.last().map_or(0, |token| token.offset + 1),
                format!("expected {expected}, found end of query"),
            )
        })?;
        self.cursor += 1;
        Ok(token)
    }

    pub(super) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn peek_is(&self, expected: TokenKind) -> bool {
        self.peek().is_some_and(|token| token.kind == expected)
    }

    fn previous_offset(&self) -> usize {
        self.tokens
            .get(self.cursor.saturating_sub(1))
            .map_or(0, |token| token.offset)
    }
}

pub(super) fn is_false(value: &bool) -> bool {
    !*value
}

impl Token {
    fn is_keyword(&self, expected: &str) -> bool {
        matches!(&self.kind, TokenKind::Word(value) if value.eq_ignore_ascii_case(expected))
    }
}

pub(super) fn parameter(value: &str, offset: usize) -> Result<String, ParseError> {
    let Some(name) = value.strip_prefix('$') else {
        return Err(ParseError::new(offset, "parameter must begin with '$'"));
    };
    validate_field(name, offset)?;
    Ok(name.to_owned())
}

pub(super) fn validate_field(value: &str, offset: usize) -> Result<(), ParseError> {
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '.'))
    {
        return Err(ParseError::new(
            offset,
            format!("invalid field or parameter name {value:?}"),
        ));
    }
    Ok(())
}

pub(super) fn traversal_id(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.first().is_none_or(u8::is_ascii_digit)
        || bytes.iter().any(|byte| {
            byte.is_ascii_whitespace()
                || matches!(byte, b':' | b',' | b'=' | b'!' | b'<' | b'>' | b'*' | b'"')
        })
    {
        quote(value)
    } else {
        value.to_owned()
    }
}

pub(super) fn quote(value: &str) -> String {
    let mut out = String::from("\"");
    for character in value.chars() {
        match character {
            '\"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            value => out.push(value),
        }
    }
    out.push('\"');
    out
}
