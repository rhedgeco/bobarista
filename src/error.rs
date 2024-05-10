use ariadne::{Report, ReportKind, Source};

use crate::lexer::token::Span;

pub type Color = ariadne::Color;

#[derive(Debug, Clone)]
pub struct Label {
    pub message: String,
    pub color: Color,
    pub span: Span,
}

impl Label {
    pub fn new(message: impl Into<String>, color: Color, span: Span) -> Self {
        Self {
            message: message.into(),
            color,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BobaError {
    pub offset: usize,
    pub message: String,
    pub labels: Vec<Label>,
}

impl BobaError {
    pub fn new(message: impl Into<String>) -> Self {
        Self::new_offset(0, message)
    }

    pub fn new_offset(offset: usize, message: impl Into<String>) -> Self {
        Self {
            offset,
            message: message.into(),
            labels: Vec::new(),
        }
    }

    pub fn label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub fn report(self, id: impl AsRef<str>, source: impl Into<Source>) {
        let id = id.as_ref();
        let source = source.into();
        let mut report = Report::build(ReportKind::Error, id, 0).with_message(self.message);

        for label in self.labels {
            report.add_label(
                ariadne::Label::new((id, label.span))
                    .with_color(label.color)
                    .with_message(label.message),
            )
        }

        report.finish().eprint((id, source)).unwrap();
    }
}
