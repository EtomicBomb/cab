use crate::restrictions::{CourseCode, ExamScore, Operator, PrerequisiteTree, Qualification};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

/// # Grammar
/// Class | Rules
/// ---|---
/// top      | any_expr Eoi
/// any_expr | and_expr (Any and_expr)*
/// and_expr | base (All base)*
/// base     | Course \| ExamScore \| LeftParen any_expr RightParen

impl<'a> TryFrom<&'a str> for PrerequisiteTree {
    type Error = PrerequisiteStringError<'a>;
    fn try_from(string: &'a str) -> Result<Self, Self::Error> {
        let mut tokens = TokenStream::try_from(string)?;
        let ret = parse_any_expr(&mut tokens);
        tokens.consume_token(&TokenKind::Eoi)?;
        ret
    }
}

fn parse_any_expr<'a, 'b>(
    tokens: &'b mut TokenStream<'a>,
) -> Result<PrerequisiteTree, PrerequisiteStringError<'a>> {
    let mut ret = Vec::new();
    let token = parse_all_expr(tokens)?;
    ret.extend(token);

    while tokens.peek_token()?.kind == TokenKind::Operator(Operator::Any) {
        tokens.consume_token(&TokenKind::Operator(Operator::Any))?;
        let token = parse_all_expr(tokens)?;
        ret.extend(token);
    }

    if ret.len() < 2 {
        Ok(ret.pop().unwrap())
    } else {
        Ok(PrerequisiteTree::Operator(Operator::Any, ret))
    }
}

fn parse_all_expr<'a, 'b>(
    tokens: &'b mut TokenStream<'a>,
) -> Result<Option<PrerequisiteTree>, PrerequisiteStringError<'a>> {
    let mut ret = Vec::new();
    let token = parse_bottom(tokens)?;
    ret.extend(token);

    while tokens.peek_token()?.kind == TokenKind::Operator(Operator::All) {
        tokens.consume_token(&TokenKind::Operator(Operator::All))?;
        let token = parse_bottom(tokens)?;
        ret.extend(token);
    }

    if ret.len() < 2 {
        Ok(ret.pop())
    } else {
        Ok(Some(PrerequisiteTree::Operator(Operator::All, ret)))
    }
}

fn parse_bottom<'a, 'b>(
    tokens: &'b mut TokenStream<'a>,
) -> Result<Option<PrerequisiteTree>, PrerequisiteStringError<'a>> {
    let token = tokens.peek_token()?;
    tokens.consume_token(&token.kind)?;

    match token.kind {
        TokenKind::Qualification(qual) => Ok(Some(PrerequisiteTree::Qualification(qual))),
        TokenKind::GraduateStudentWaive => Ok(None),
        TokenKind::LeftParen => {
            let ret = parse_any_expr(tokens)?;
            tokens.consume_token(&TokenKind::RightParen)?;
            Ok(Some(ret))
        }
        _ => Err(PrerequisiteStringError::ExpectedLeftParenOrQualification { found: token }),
    }
}

struct TokenStream<'a> {
    tokens: Vec<Token<'a>>,
    index: usize,
}

impl<'a> TokenStream<'a> {
    fn peek_token(&self) -> Result<Token<'a>, PrerequisiteStringError<'a>> {
        self.tokens
            .get(self.index)
            .cloned()
            .ok_or(PrerequisiteStringError::EarlyEoi)
    }

    fn consume_token(&mut self, token: &TokenKind) -> Result<(), PrerequisiteStringError<'a>> {
        let found = &self.tokens[self.index];
        if &found.kind == token {
            self.index += 1;
            Ok(())
        } else {
            Err(PrerequisiteStringError::ExpectedToken {
                expected: token.clone(),
                found: found.clone(),
            })
        }
    }
}

impl<'a> TryFrom<&'a str> for TokenStream<'a> {
    type Error = PrerequisiteStringError<'a>;
    fn try_from(string: &'a str) -> Result<Self, Self::Error> {
        /// Replaces Token::Comma in `tokens` with the right conjunctive.
        fn de_comma<'a>(tokens: &mut [Token<'a>]) -> Result<(), PrerequisiteStringError<'a>> {
            // each paren level needs its own conjunctive token stored
            let mut conjunctives: HashMap<i32, Operator> = HashMap::new();
            let mut paren_level = 0;

            for token in tokens.iter_mut().rev() {
                let matching_token = &token.kind;

                match matching_token {
                    TokenKind::Operator(conj) => {
                        conjunctives.insert(paren_level, *conj);
                    }
                    TokenKind::LeftParen => paren_level += 1,
                    TokenKind::RightParen => paren_level -= 1,
                    TokenKind::Comma => {
                        token.kind = match conjunctives.get(&paren_level) {
                            Some(&conj) => TokenKind::Operator(conj),
                            None => TokenKind::Operator(Operator::Any),
                        }
                    }
                    _ => {}
                }
            }

            Ok(())
        }

        let mut tokens = tokenize(string)?;
        de_comma(&mut tokens)?;
        Ok(TokenStream { tokens, index: 0 })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token<'a> {
    kind: TokenKind,
    span: Span<'a>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Span<'a> {
    input: &'a str,
    start: usize,
    end: usize,
}

impl<'a> fmt::Display for Span<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}[{}]{}",
            &self.input[..self.start],
            &self.input[self.start..self.end],
            &self.input[self.end..]
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Qualification(Qualification),
    Operator(Operator),
    Comma,
    LeftParen,
    RightParen,
    GraduateStudentWaive,
    Eoi,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Qualification(qual) => fmt::Display::fmt(qual, f),
            TokenKind::Operator(conj) => fmt::Display::fmt(conj, f),
            TokenKind::Comma => f.write_str(","),
            TokenKind::LeftParen => f.write_str("("),
            TokenKind::RightParen => f.write_str(")"),
            TokenKind::GraduateStudentWaive => f.write_str("graduate student waive"),
            TokenKind::Eoi => f.write_str("end of input"),
        }
    }
}

fn tokenize(string: &str) -> Result<Vec<Token>, PrerequisiteStringError<'_>> {
    static TOKEN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^( |and|or|,|\(|\)|minimum score of WAIVE in 'Graduate Student PreReq'|minimum score of (?P<score>\d*?) in '(?P<exam>.*?)'|((?P<subj>[A-Z]{3,4}) )?(?P<num>\d{4}[A-Z]?)\*?)").unwrap()
    });

    let mut last_subject = None;

    let mut ret = Vec::with_capacity(string.len());

    let mut i = 0;

    while i < string.len() {
        let captures = match TOKEN.captures(&string[i..]) {
            Some(captures) => captures,
            None => return Err(PrerequisiteStringError::InvalidToken { string, start: i }),
        };
        let entire_match = &captures[0];

        let span = Span {
            start: i,
            end: i + entire_match.len(),
            input: string,
        };
        i += entire_match.len();

        let kind = match entire_match {
            " " => continue,
            "minimum score of WAIVE in 'Graduate Student PreReq'" => {
                TokenKind::GraduateStudentWaive
            }
            "and" => TokenKind::Operator(Operator::All),
            "or" => TokenKind::Operator(Operator::Any),
            "," => TokenKind::Comma,
            "(" => TokenKind::LeftParen,
            ")" => TokenKind::RightParen,
            _ if captures.name("score").is_some() => {
                TokenKind::Qualification(Qualification::ExamScore(ExamScore {
                    exam: captures["exam"].to_string(),
                    score: captures["score"].parse().unwrap(),
                }))
            }
            _ if captures.name("num").is_some() => {
                if let Some(subject) = captures.name("subj") {
                    let subject = subject.as_str().parse().unwrap();
                    last_subject = Some(subject);
                }

                TokenKind::Qualification(Qualification::Course(
                    CourseCode::new(
                        last_subject
                            .clone()
                            .ok_or(PrerequisiteStringError::NoSubjectContext { span })?,
                        captures["num"].parse().unwrap(),
                    )
                    .unwrap(),
                ))
            }
            _ => unreachable!(),
        };

        ret.push(Token { kind, span });
    }

    ret.push(Token {
        kind: TokenKind::Eoi,
        span: Span {
            input: string,
            start: string.len() - 1,
            end: string.len(),
        },
    });

    Ok(ret)
}

#[derive(Clone)]
pub enum PrerequisiteStringError<'a> {
    InvalidToken {
        string: &'a str,
        start: usize,
    },
    ExpectedToken {
        expected: TokenKind,
        found: Token<'a>,
    },
    NoSubjectContext {
        span: Span<'a>,
    },
    ExpectedLeftParenOrQualification {
        found: Token<'a>,
    },
    EarlyEoi,
}

impl<'a> fmt::Debug for PrerequisiteStringError<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            PrerequisiteStringError::InvalidToken { string, start } => write!(
                f,
                "'{} [{}]': invalid token",
                &string[..*start],
                &string[*start..]
            ),
            PrerequisiteStringError::ExpectedToken { expected, found } => {
                write!(f, "'{}': expected {}", found.span, expected)
            }
            PrerequisiteStringError::NoSubjectContext { span: location } => {
                write!(f, "'{}': no subject found for course number", location)
            }
            PrerequisiteStringError::ExpectedLeftParenOrQualification { found } => write!(
                f,
                "'{}': expected qualification or '(', found {}",
                found.span, found.kind
            ),
            PrerequisiteStringError::EarlyEoi => {
                write!(f, "Reached the end of the input too early")
            }
        }
    }
}
