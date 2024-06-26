use std::fmt::Debug;

use ariadne::{Color, Label, Report, ReportKind, Span};
use dashu::base::ParseError;

use crate::cache::CacheSpan;

pub type PResult<Data, T> = Result<T, PError<Data>>;

#[derive(Debug, Clone)]
#[repr(u8)]
pub enum PError<Data> {
    UnexpectedEnd {
        expected: String,
        data: Data,
    },
    InvalidToken {
        part: String,
        data: Data,
    },
    UnclosedString {
        data: Data,
    },
    ParseNumError {
        error: ParseError,
        data: Data,
    },
    UnexpectedToken {
        expected: String,
        found: String,
        data: Data,
    },
    UnclosedBrace {
        data: Data,
    },
    InvalidAssignment {
        data: Data,
    },
    MixedTabsAndSpaces {
        data: Data,
        tab: bool,
    },
}

impl PError<CacheSpan> {
    pub fn report(&self) -> Report<CacheSpan> {
        match self {
            PError::UnexpectedEnd { expected, data } => {
                Report::build(ReportKind::Error, data.source().clone(), data.start())
                    .with_code(format!("C-001"))
                    .with_message("Unexpected End of Input")
                    .with_label(
                        Label::new(data.clone())
                            .with_color(Color::Red)
                            .with_message(format!("expected {expected}, found end of input")),
                    )
                    .finish()
            }
            PError::InvalidToken { part, data } => {
                Report::build(ReportKind::Error, data.source().clone(), data.start())
                    .with_code(format!("C-002"))
                    .with_message("Invalid Token")
                    .with_label(
                        Label::new(data.clone())
                            .with_color(Color::Red)
                            .with_message(format!("invalid token {part}")),
                    )
                    .finish()
            }
            PError::UnclosedString { data } => {
                Report::build(ReportKind::Error, data.source().clone(), data.start())
                    .with_code(format!("C-003"))
                    .with_message("Unclosed String")
                    .with_label(
                        Label::new(data.clone())
                            .with_color(Color::Red)
                            .with_message(format!("string has no closing quote")),
                    )
                    .finish()
            }
            PError::ParseNumError { error, data } => {
                Report::build(ReportKind::Error, data.source().clone(), data.start())
                    .with_code(format!("C-004"))
                    .with_message("Invalid Integer")
                    .with_label(
                        Label::new(data.clone())
                            .with_color(Color::Red)
                            .with_message(format!("error parsing number: {error}")),
                    )
                    .finish()
            }
            PError::UnexpectedToken {
                expected,
                found,
                data,
            } => Report::build(ReportKind::Error, data.source().clone(), data.start())
                .with_code(format!("C-006"))
                .with_message("Unexpected Token")
                .with_label(
                    Label::new(data.clone())
                        .with_color(Color::Red)
                        .with_message(format!("expected {expected}, found {found}")),
                )
                .finish(),
            PError::UnclosedBrace { data } => {
                Report::build(ReportKind::Error, data.source().clone(), data.start())
                    .with_code(format!("C-007"))
                    .with_message("Unclosed Brace")
                    .with_label(
                        Label::new(data.clone())
                            .with_color(Color::Red)
                            .with_message(format!("opening brace has no closing brace")),
                    )
                    .finish()
            }
            PError::InvalidAssignment { data } => {
                Report::build(ReportKind::Error, data.source().clone(), data.start())
                    .with_code(format!("C-008"))
                    .with_message("Invalid Assignment")
                    .with_label(
                        Label::new(data.clone())
                            .with_color(Color::Red)
                            .with_message(format!(
                                "cannot assign expression to another expression"
                            )),
                    )
                    .finish()
            }
            PError::MixedTabsAndSpaces { data, tab } => {
                Report::build(ReportKind::Error, data.source().clone(), data.start())
                    .with_code(format!("C-009"))
                    .with_message("Mixed Tabs and Spaces")
                    .with_label(
                        Label::new(data.clone())
                            .with_color(Color::Red)
                            .with_message(match tab {
                                true => format!("tab found here when a space was expected"),
                                false => format!("space found here when a tab was expected"),
                            }),
                    )
                    .finish()
            }
        }
    }
}
