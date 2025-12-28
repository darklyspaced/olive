use std::ops::{ControlFlow, FromResidual, Try};

use super::{Parser, State, action::Tree};

use crate::{
    error::{
        self,
        parse_err::{ParseError, ParseErrorKind as PEKind},
    },
    syntax::SyntaxKind,
    token::{Token, TokenKind},
};

type Error = error::Error<ParseError>;

macro_rules! error {
    ($self:ident, $kind:expr, $tok:expr) => {{
        let context = Some(
            $self
                .source_map
                .ctxt_from_tok($tok)
                .with(line!(), column!()),
        );
        ParseError {
            kind: $kind,
            ctxt: context,
        }
        .into();
    }};
    // if it's something to do with an EOF
    ($self:ident, $kind:expr) => {
        ParseError {
            kind: $kind,
            ctxt: Some($self.source_map.ctxt_from_end().with(line!(), column!())),
        }
        .into()
    };
}

impl Parser<'_> {
    /// Peeks the next token and handles error cases. `err_kind` is for the EOF case
    // TODO: make this a try_peek macro so that line and column information can be captured
    // correctly
    pub(super) fn peek(&mut self, eof_err: impl FnOnce(String) -> PEKind) -> Option<&Token<'_>> {
        match self.toks.peek() {
            Some(tok) => Some(tok),
            None => {
                self.errors.push(error!(self, eof_err(String::from("EOF"))));
                None
            }
        }
    }

    pub(super) fn at_eof(&mut self) -> bool {
        self.toks.peek().is_none()
    }

    /// Should only be used the case that the next token **exists** but isn't what it should be and
    /// cooks up error.
    // TODO: make a wrapper for this so that the line and column are captured correctly
    pub fn make_err(&mut self, make_kind: impl FnOnce(String) -> PEKind) -> Error {
        let erroneous_tok = self.toks.next().expect("shouldn't be EOF");
        let kind = make_kind(String::from(erroneous_tok.lexeme));
        let context = Some(
            self.source_map
                .ctxt_from_tok(&erroneous_tok)
                .with(line!(), column!()),
        );
        ParseError {
            kind,
            ctxt: context,
        }
        .into()
    }

    /// Attempts to parse an ident and interns its symbol, returning any `err` if it fails
    pub(super) fn ident(&mut self, err: impl FnOnce(String) -> PEKind) -> Option<Result<(), ()>> {
        let next = self.peek(PEKind::ExpIdentFound)?; // None -> found eof
        let ident = match next.kind {
            TokenKind::Ident => self.toks.next().unwrap(),
            _ => {
                let err = self.make_err(err);
                self.errors.push(err);
                return Some(Err(())); // found token that wasn't ident
            }
        };

        self.actions.push(Tree::Ident {
            ident: crate::ast::Ident {
                sym: self.interner.intern(ident.lexeme),
                width: ident.lexeme.len(),
            },
        });

        Some(Ok(())) // parsed ident!
    }

    /// Eats `kind` otherwise throws `err`
    pub(super) fn eat<G>(&mut self, kind: TokenKind, err: G) -> Option<Result<(), ()>>
    where
        G: FnOnce(String) -> PEKind + Clone,
    {
        let equals = self.peek(err.clone())?;
        if equals.kind == kind {
            self.consume();
        } else {
            let err = self.make_err(err);
            self.errors.push(err);
            return Some(Err(()));
        }
        Some(Ok(()))
    }

    pub(super) fn at_any(&mut self, set: &[TokenKind]) -> bool {
        match self.toks.peek() {
            Some(tok) => set.contains(&tok.kind),
            None => false,
        }
    }

    /// Eats the next token no matter what it is granted that it isn't a lex_error or EOF. Need
    /// those two invariants (via peek for example) to call this
    pub(super) fn consume(&mut self) {
        let tok = self.toks.next().unwrap();
        self.actions.push(Tree::Token { tok })
    }

    pub(super) fn consume_err(&mut self, make_kind: impl FnOnce(String) -> PEKind) {
        let err = self.make_err(make_kind); // make error before consuming token to get token info
        self.errors.push(err);
        let error_tree = self.start();
        self.consume();
        error_tree.end(SyntaxKind::Error, self);
    }

    /// Conforms the parser to to a state that is satisfactory based on the input set of tokens
    pub(super) fn conform(&mut self, res: Result<(), ()>, set: &[TokenKind]) -> Option<()> {
        if let Err(_) = res {
            let error_tree = self.start();

            while !self.at_any(set) && !self.at_eof() {
                self.consume();
            }

            error_tree.end(SyntaxKind::Error, self);

            return Some(());
        } else {
            Some(())
        }
    }
}
