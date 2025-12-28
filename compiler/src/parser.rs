mod action;
pub mod utils;

use std::iter::Peekable;

use crate::{
    ast::OpKind,
    error::{
        self,
        parse_err::{ParseError, ParseErrorKind as PEKind},
        source_map::SourceMap,
    },
    interner::Interner,
    lexer::Lexer,
    parser::action::{CompletedTreeIdx, Tree},
    red_node::SyntaxTree,
    syntax::SyntaxKind,
    token::{Token, TokenKind},
};

type Error = error::Error<ParseError>;

/// The state of the parser
pub enum State {
    /// Parse
    Parse,
    /// Recover until we see a statement boundary
    Recover,
    /// Return `None` forever
    Abort,
    /// Return `None` forever
    Finished,
}

pub struct Parser<'de> {
    interner: &'de mut Interner,
    source_map: &'de SourceMap,
    toks: Peekable<Lexer<'de>>,
    actions: Vec<Tree<'de>>,
    errors: Vec<Error>,
}

macro_rules! check {
    ($expr:expr) => {
        if ($expr).is_err() {
            return Some(Err(()));
        }
    };
}

macro_rules! doo {
    ($($e:expr);* $(;)?) => {
        $(check!($e);)*
    };
}

// TODO: need to add support for . notation for field access
// TODO: add support for use statements and paths to fully qualify names
///
/// Return Option<()> from all functions because we just want to shortcircuit if EOF is found
impl<'de> Parser<'de> {
    pub fn new(iter: Lexer<'de>, source_map: &'de SourceMap, interner: &'de mut Interner) -> Self {
        Self {
            toks: iter.peekable(),
            actions: vec![],
            errors: vec![],
            source_map,
            interner,
        }
    }

    pub fn parse(&mut self) -> (SyntaxTree<'_>, Vec<Error>) {
        while self.toks.peek().is_some() {
            if self.stmt().is_none() {
                break;
            }
        }

        (
            SyntaxTree::new_root(action::build(std::mem::take(&mut self.actions))),
            std::mem::take(&mut self.errors),
        )
    }

    /// func | var | assignment | for | if | struct
    fn stmt(&mut self) -> Option<()> {
        const STMT_RECOVERY: &[TokenKind] = &[
            TokenKind::Let,
            TokenKind::Fn,
            TokenKind::If,
            TokenKind::Impl,
            TokenKind::Struct,
        ];
        match self
            .peek(|x| PEKind::ExpFound(vec![TokenKind::Fn, TokenKind::Let], x))?
            .kind
        {
            TokenKind::Let => {
                let decl = self.declaration()?;
                self.conform(decl, STMT_RECOVERY)
            }
            TokenKind::Fn => {
                let fn_decl = self.fn_decl()?;
                self.conform(fn_decl, STMT_RECOVERY)
            }
            TokenKind::For => {
                let for_loop = self.for_loop()?;
                self.conform(for_loop, STMT_RECOVERY)
            }
            TokenKind::If => {
                let if_stmt = self.if_stmt()?;
                self.conform(if_stmt, STMT_RECOVERY)
            }
            TokenKind::Impl => {
                let impl_stmt = self.impl_block()?;
                self.conform(impl_stmt, STMT_RECOVERY)
            }
            TokenKind::Struct => {
                let strct = self.structure()?;
                self.conform(strct, STMT_RECOVERY)
            }
            _ => {
                let assignment = self.assignment(true)?;
                self.conform(assignment, STMT_RECOVERY)
            }
        }
    }

    fn impl_block(&mut self) -> Option<Result<(), ()>> {
        let t = self.start();

        self.consume(); // impl
        doo! {
            self.ident(PEKind::ExpImplStructTargetFound)?; // target struct
            self.eat(TokenKind::LeftBrace, PEKind::ExpLBraceFound)?;
        }

        let mut next = self.peek(PEKind::ExpRBraceFound)?.kind;

        while next != TokenKind::RightBrace {
            let fn_decl = self.fn_decl()?;
            self.conform(fn_decl, &[TokenKind::Fn]);
            next = self.peek(PEKind::ExpRBraceFound)?.kind;
        }

        self.consume(); // r_brace

        t.end(SyntaxKind::Impl, self);
        Some(Ok(()))
    }

    fn structure(&mut self) -> Option<Result<(), ()>> {
        // assume the fields are comma separated
        const STRUCT_RECOVERY: &[TokenKind] = &[TokenKind::Comma, TokenKind::RightBrace];
        let t = self.start();

        self.consume(); // struct
        doo! {
            self.ident(PEKind::ExpIdentFound)?;
            self.eat(TokenKind::LeftBrace, PEKind::ExpLBraceFound)?;
        }

        let mut next = self
            .peek(|x| PEKind::ExpFound(vec![TokenKind::Comma, TokenKind::RightBrace], x))?
            .kind;
        while next != TokenKind::RightBrace {
            if next == TokenKind::Ident {
                self.param()?;
            } else {
                if self.at_any(STRUCT_RECOVERY) {
                    if next == TokenKind::Comma {
                        self.consume_err(PEKind::ExpParamFound);
                        // attempt to parse another field now
                    } else {
                        break;
                    }
                }
                self.consume_err(PEKind::ExpParamFound);
            }

            next = self
                .peek(|x| PEKind::ExpFound(vec![TokenKind::Ident, TokenKind::RightParen], x))?
                .kind;
        }

        self.consume(); // r_brace

        t.end(SyntaxKind::Struct, self);
        Some(Ok(()))
    }

    fn if_stmt(&mut self) -> Option<Result<(), ()>> {
        let t = self.start();

        self.consume(); // if

        doo! {
            self.eat(TokenKind::LeftParen, PEKind::ExpLParenFound)?;
            self.expression()?; // predicate
            self.eat(TokenKind::RightParen, PEKind::ExpRParenFound)?;
        }

        check!(self.block()?); // then

        if let Some(Token {
            kind: TokenKind::Else,
            ..
        }) = self.toks.peek()
        {
            self.consume();
            if let Some(Token {
                kind: TokenKind::If,
                ..
            }) = self.toks.peek()
            {
                check!(self.if_stmt()?);
            } else {
                check!(self.block()?);
            }
        }

        t.end(SyntaxKind::If, self);

        Some(Ok(()))
    }

    fn for_loop(&mut self) -> Option<Result<(), ()>> {
        self.eat(TokenKind::LeftParen, PEKind::ExpLParenFound)?
            .unwrap();
        check!(self.ident(PEKind::ExpIdentFound)?);

        if let Some(Token {
            kind: TokenKind::Colon,
            ..
        }) = self.toks.peek()
        {
            self.consume();
            check!(self.ident(PEKind::ExpTyFound)?);
        }

        doo! {
            self.eat(TokenKind::Equal, |_| PEKind::IdxNotInitialised)?;

            self.expression()?; // value
            self.eat(TokenKind::Semicolon, PEKind::ExpSemicolonFound)?;

            self.expression()?; // predicate
            self.eat(TokenKind::Semicolon, PEKind::ExpSemicolonFound)?;

            self.assignment(false)?;

            self.eat(TokenKind::RightParen, PEKind::ExpRParenFound)?;
            self.block()?;
        }

        Some(Ok(()))
    }

    // Param = Ident ':' TypeExpr ','?
    fn param(&mut self) -> Option<()> {
        let t = self.start();

        self.ident(PEKind::Unreachable).unwrap().unwrap();
        let _ = self.eat(TokenKind::Colon, PEKind::ExpTyAnnotationFound)?;
        let _ = self.ident(PEKind::ExpTyFound)?;
        if self.peek(PEKind::ExpRParenFound)?.kind != TokenKind::RightParen {
            let _ = self.eat(TokenKind::Comma, PEKind::ExpTyAnnotationFound)?;
        }
        t.end(SyntaxKind::Param, self);
        Some(())
    }

    fn fn_decl(&mut self) -> Option<Result<(), ()>> {
        const PARAM_LIST_RECOVERY: &[TokenKind] = &[
            TokenKind::Arrow,
            TokenKind::LeftBrace,
            TokenKind::RightParen,
        ];
        let t = self.start();

        self.consume(); // fn
        check!(self.ident(PEKind::ExpIdentFound)?);
        check!(self.eat(TokenKind::LeftParen, PEKind::ExpLParenFound)?);

        let mut next = self
            .peek(|x| PEKind::ExpFound(vec![TokenKind::Ident, TokenKind::RightParen], x))?
            .kind;

        while next != TokenKind::RightParen {
            if next == TokenKind::Ident {
                self.param()?;
                next = self
                    .peek(|x| PEKind::ExpFound(vec![TokenKind::Ident, TokenKind::RightParen], x))?
                    .kind;
            } else {
                let err = self.make_err(PEKind::ExpParamFound);
                self.errors.push(err);

                let error_tree = self.start();

                while !self.at_any(PARAM_LIST_RECOVERY) && !self.at_eof() {
                    self.consume();
                }

                error_tree.end(SyntaxKind::Error, self);
                break;
            }
        }

        self.consume(); // r_paren

        let err = |x| PEKind::ExpFound(vec![TokenKind::LeftBrace, TokenKind::Arrow], x);
        let branch = self.peek(err)?;
        match branch.kind {
            TokenKind::Arrow => {
                self.consume();
                let _ = self.ident(PEKind::ExpIdentFound)?; // ignore failure and continue
            }
            TokenKind::LeftBrace => (),
            _ => {
                let err = self.make_err(err);
                self.errors.push(err);
                let error_tree = self.start();
                while !self.at_any(&[TokenKind::LeftBrace]) && !self.at_eof() {
                    self.consume();
                }
                error_tree.end(SyntaxKind::Error, self);
            }
        };

        check!(self.block()?);

        t.end(SyntaxKind::FnDecl, self);

        Some(Ok(()))
    }

    fn block(&mut self) -> Option<Result<(), ()>> {
        let t = self.start();
        check!(self.eat(TokenKind::LeftBrace, PEKind::ExpLParenFound)?);

        let mut next = self.peek(PEKind::ExpLBraceFound)?;
        while next.kind != TokenKind::RightBrace {
            self.stmt()?; // the recovery is done by statement
            next = self.peek(PEKind::ExpRBraceFound)?;
        }

        self.consume(); // r_brace

        t.end(SyntaxKind::Block, self);
        Some(Ok(()))
    }

    /// Parse an assignment of a value to an ident
    fn assignment(&mut self, semi: bool) -> Option<Result<(), ()>> {
        let t = self.start();
        doo! { // checks all, returning early (with Err()) if any one of them fails
            self.ident(PEKind::ExpIdentFound)?;
            self.eat(TokenKind::Equal, PEKind::ExpEqualFound)?;
            self.expression()?;
        }
        if semi {
            check!(self.eat(TokenKind::Semicolon, PEKind::ExpSemicolonFound)?);
        }

        t.end(SyntaxKind::Assignment, self);
        Some(Ok(()))
    }

    /// Parse a declaration
    fn declaration(&mut self) -> Option<Result<(), ()>> {
        let t = self.start();
        self.consume(); // let
        check!(self.ident(PEKind::ExpIdentFound)?);

        let branch = self
            .peek(|x| PEKind::ExpFound(vec![TokenKind::Semicolon, TokenKind::Equal], x))?
            .kind;

        if branch == TokenKind::Colon {
            self.consume();
            check!(self.ident(PEKind::ExpTyFound)?);
        }

        match branch {
            TokenKind::Semicolon => {
                self.consume();
            }
            TokenKind::Equal => {
                self.consume();
                check!(self.expression()?);
                check!(self.eat(TokenKind::Semicolon, PEKind::ExpSemicolonFound)?);
            }
            _ => {
                let err = self.make_err(|tok| {
                    PEKind::ExpFound(vec![TokenKind::Semicolon, TokenKind::Equal], tok)
                });
                self.errors.push(err);
                return Some(Err(()));
            }
        };

        t.end(SyntaxKind::Declaration, self);

        Some(Ok(()))
    }

    fn parse_params(&mut self) -> Option<()> {
        const PARAM_LIST_RECOVERY: &[TokenKind] = &[TokenKind::Semicolon];

        let t = self.start();
        self.consume(); // l_paren

        let mut next = self.peek(PEKind::ExpExprFound)?;
        if next.kind != TokenKind::RightParen {
            loop {
                // NOTE: loop here so we do error recovery local and return Option<()>
                if self.expression()?.is_err() {
                    // TODO: make sure recovery works
                    let error_tree = self.start();
                    while !self.at_any(PARAM_LIST_RECOVERY) && !self.at_eof() {
                        self.consume();
                    }
                    error_tree.end(SyntaxKind::Error, self);
                }

                let err = |x| PEKind::ExpFound(vec![TokenKind::Comma, TokenKind::RightParen], x);

                next = self.peek(err)?;
                match next.kind {
                    TokenKind::RightParen => {
                        break;
                    }
                    TokenKind::Comma => {
                        let _comma = self.consume();
                        continue;
                    }
                    _ => {
                        let error_tree = self.start();

                        while !self.at_any(PARAM_LIST_RECOVERY) && !self.at_eof() {
                            self.consume();
                        }

                        error_tree.end(SyntaxKind::Error, self);
                        let error = self.make_err(err);
                        self.errors.push(error);
                    }
                };
            }
        }

        let _r_paren = self.consume();
        t.end(SyntaxKind::ParamList, self);
        Some(())
    }

    /// Extract an expression, handling any errors that were raised
    fn expression(&mut self) -> Option<Result<(), ()>> {
        Some(self.expr(0)?.and(Ok(())))
    }

    /// An implementation of Pratt Parsing to deal with mathematical operations. All calls to this
    /// function from outside of itself must have `min_bp` = 0.
    fn expr(&mut self, min_bp: u8) -> Option<Result<CompletedTreeIdx, ()>> {
        let t = self.start();
        // BUG: parenthesises??

        let next = self.peek(PEKind::ExpExprFound)?;
        let mut lhs = match next.kind {
            TokenKind::Number | TokenKind::Float => {
                self.consume();
                t.end(SyntaxKind::AtomExpr, self)
            }
            TokenKind::Ident => {
                self.ident(PEKind::ExpIdentFound).unwrap().unwrap();

                let next = self.peek(PEKind::ExpSemicolonFound)?;
                match next.kind {
                    TokenKind::LeftParen => {
                        self.parse_params()?;
                        t.end(SyntaxKind::FnApp, self)
                    }
                    _ => t.end(SyntaxKind::AtomExpr, self),
                }
            }
            _ => {
                let err = self.make_err(PEKind::ExpOperandFound);
                self.errors.push(err);

                return Some(Err(()));
            }
        };

        while self.toks.peek().is_some() {
            let tok = self.toks.peek().unwrap();

            let Ok(op_kind) = OpKind::try_from(tok.kind) else {
                break;
            };

            let (l, r) = infix_binding_power(&op_kind);
            if l < min_bp {
                break; // fold to left
            }

            // only want to consume once we know that we're folding so that after
            // folding we can resume on the operator that had a lower BP to the left of
            // it
            let bin_expr = lhs.precede(self);
            self.consume(); // op

            check!(self.expr(r)?); // rhs
            lhs = bin_expr.end(SyntaxKind::BinExpr, self);
        }

        Some(Ok(lhs))
    }
}

fn infix_binding_power(op: &OpKind) -> (u8, u8) {
    match op {
        OpKind::Equal => (1, 2),
        OpKind::Or => (3, 4),
        OpKind::And => (5, 6),
        OpKind::Greater | OpKind::GreaterEqual | OpKind::Less | OpKind::LessEqual => (7, 8),
        OpKind::Add | OpKind::Sub => (9, 10),
        OpKind::Mult | OpKind::Div => (11, 12),
    }
}
