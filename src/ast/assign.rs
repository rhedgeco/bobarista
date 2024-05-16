use crate::{
    parser::{node::NodeBuilderExt, report::PError, Node, TokenSource},
    Token,
};

use super::{Expr, Ident};

#[derive(Debug)]
pub struct Assign {
    pub ident: Node<Ident>,
    pub expr: Node<Expr>,
}

impl Assign {
    pub fn parse<'a>(source: &mut impl TokenSource<'a>) -> Result<Node<Self>, PError> {
        let mut builder = source.node_builder();

        // check for let token
        match builder.take() {
            Some((Token::Let, _)) => (),
            Some((token, span)) => {
                return Err(PError::UnexpectedToken {
                    expect: "'let'".into(),
                    found: format!("'{token}'"),
                    span: span.clone(),
                })
            }
            None => {
                return Err(PError::UnexpectedEnd {
                    expect: "'let'".into(),
                    pos: builder.pos(),
                })
            }
        }

        // parse ident
        let ident = Ident::parse(&mut builder)?;

        // check for assign token
        match builder.take() {
            Some((Token::Assign, _)) => (),
            Some((token, span)) => {
                return Err(PError::UnexpectedToken {
                    expect: "'='".into(),
                    found: format!("'{token}'"),
                    span: span.clone(),
                })
            }
            None => {
                return Err(PError::UnexpectedEnd {
                    expect: "'='".into(),
                    pos: builder.pos(),
                })
            }
        }

        // parse expression
        let expr = Expr::parse(&mut builder)?;

        // build assignment
        Ok(builder.build(Self { ident, expr }))
    }
}