use crate::token::TokenKind;

use super::{
    lex_err::LexErrorKind,
    reportable::{Ctxt, RawCtxt, Reportable},
};

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    /// The line number, line, and offset of annotation
    pub ctxt: Option<RawCtxt>,
}

#[derive(Debug)]
pub enum ParseErrorKind {
    Lexing(LexErrorKind),
    ExpExprFound(String),
    ExpOpFound(String),
    ExpOperandFound(String),
    ExpIdentFound(String),
    ExpImplStructTargetFound(String),
    ExpTyFound(String),
    ExpSemicolonFound(String),
    ExpEqualFound(String),
    ExpLParenFound(String),
    ExpRParenFound(String),
    ExpLBraceFound(String),
    ExpRBraceFound(String),
    ExpBlockFound(String),
    ExpIfOrBlockFound(String),
    ExpColonFound(String),
    ExpCommaFound(String),
    ExpVarDefFound(String),
    ExpTyAnnotationFound(String),
    ExpParamFound(String),
    ExpStructFieldFound(String),
    ExpFound(Vec<TokenKind>, String),
    IdxNotInitialised,
    SolelyAssDecl,
    /// Places where an error kind is expected but it will never be produced
    Unreachable(String),
}

impl Reportable for ParseError {
    fn ctxt(&mut self) -> Vec<Ctxt> {
        let annotation = self.kind.annotation();

        vec![Ctxt {
            inner: self.ctxt.take().expect("should only call ctxt once"), // should function fine
            // if only called once
            annotation,
        }]
    }

    fn msg(&self) -> String {
        self.kind.message()
    }

    fn code(&self) -> usize {
        1
    }
}

impl ParseErrorKind {
    /// The annotation that goes below the upticks in the context of an error message
    pub fn annotation(&self) -> String {
        match self {
            ParseErrorKind::Lexing(lex_error_kind) => lex_error_kind.annotation(),
            ParseErrorKind::ExpExprFound(_) => String::from("expected expr"),
            ParseErrorKind::ExpOpFound(_) => String::from("expected op"),
            ParseErrorKind::ExpOperandFound(_) => String::from("expected operand"),
            ParseErrorKind::ExpIdentFound(_) => String::from("expected ident"),
            ParseErrorKind::ExpTyFound(_) => String::from("expected ty"),
            ParseErrorKind::ExpSemicolonFound(_) => String::from("expected `;`"),
            ParseErrorKind::ExpFound(vec, _) => {
                let list = vec
                    .iter()
                    .map(|kind| format!("`{}`", kind.to_string()))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("expected one of {list}")
            }
            ParseErrorKind::Unreachable(_) => unreachable!(),
            ParseErrorKind::SolelyAssDecl => String::from("not allowed here"),
            ParseErrorKind::ExpEqualFound(_) => String::from("expected `=`"),
            ParseErrorKind::ExpLParenFound(_) => String::from("expected `(`"),
            ParseErrorKind::ExpRParenFound(_) => String::from("expected `)`"),
            ParseErrorKind::ExpLBraceFound(_) => String::from("expected `{`"),
            ParseErrorKind::ExpRBraceFound(_) => String::from("expected `}`"),
            ParseErrorKind::ExpBlockFound(_) => String::from("expected block, `{}`"),
            ParseErrorKind::ExpCommaFound(_) => String::from("expected `,`"),
            ParseErrorKind::ExpColonFound(_) => String::from("expected `:`"),
            ParseErrorKind::ExpTyAnnotationFound(_) => String::from("expected type annotation"),
            ParseErrorKind::IdxNotInitialised => String::from("must be initialised"),
            ParseErrorKind::ExpVarDefFound(_) => String::from("expected a variable definition"),
            ParseErrorKind::ExpStructFieldFound(_) => {
                String::from("expected a struct field or `}`")
            }
            ParseErrorKind::ExpParamFound(_) => String::from("expected a function parameter"),
            ParseErrorKind::ExpImplStructTargetFound(_) => {
                String::from("expected a struct for the impl block")
            }
            ParseErrorKind::ExpIfOrBlockFound(_) => String::from("expected `if` or `{}`"),
        }
    }

    /// What goes in the title of an error message
    pub fn message(&self) -> String {
        match self {
            ParseErrorKind::Lexing(lex_error_kind) => lex_error_kind.message(),
            ParseErrorKind::ExpExprFound(x) => {
                format!("found `{x}` in place of an expression")
            }
            ParseErrorKind::ExpOpFound(x) => {
                format!("found `{x}` where operator was expected")
            }
            ParseErrorKind::ExpOperandFound(x) => {
                format!("found `{x}` where operand was expected")
            }
            ParseErrorKind::ExpIdentFound(x) => {
                format!("found `{x}` where an ident was expected")
            }
            ParseErrorKind::ExpTyFound(x) => {
                format!("found `{x}` where a type was expected")
            }
            ParseErrorKind::ExpEqualFound(x) => {
                format!("found `{x}` where `=` was expected")
            }
            ParseErrorKind::ExpSemicolonFound(x) => {
                format!("found `{x}` where `;` was expected")
            }
            ParseErrorKind::ExpLParenFound(x) => {
                format!("found `{x}` where `(` was expected")
            }
            ParseErrorKind::ExpRParenFound(x) => {
                format!("found `{x}` where `)` was expected")
            }
            ParseErrorKind::ExpLBraceFound(x) => {
                format!("found `{x}` where `{{` was expected")
            }
            ParseErrorKind::ExpRBraceFound(x) => {
                format!("found `{x}` where `}}` was expected")
            }
            ParseErrorKind::ExpTyAnnotationFound(x) => {
                format!("found `{x}` where a `: T` was expected")
            }
            ParseErrorKind::ExpCommaFound(x) => {
                format!("found `{x}` where a `,` was expected")
            }
            ParseErrorKind::ExpColonFound(x) => {
                format!("found `{x}` where a `:` was expected")
            }
            ParseErrorKind::ExpBlockFound(x) => {
                format!("found `{x}` where a block was expected")
            }
            ParseErrorKind::ExpFound(vec, x) => {
                let list = vec
                    .iter()
                    .map(|kind| format!("`{}`", kind.to_string()))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("expected one of {list}, found {x}")
            }
            ParseErrorKind::SolelyAssDecl => {
                String::from("only assignment and declarations are supported as of now")
            }
            ParseErrorKind::IdxNotInitialised => {
                String::from("loop index must be initialised, try adding a `=`")
            }
            ParseErrorKind::ExpImplStructTargetFound(x) => {
                format!("found {x} where a struct name was expected")
            }
            ParseErrorKind::ExpVarDefFound(x) => {
                format!("found {x} where a variable definition was expected")
            }
            ParseErrorKind::ExpStructFieldFound(x) => {
                format!("found {x} where a struct field or `}}` was expected")
            }
            ParseErrorKind::ExpParamFound(x) => {
                format!("found {x} where param or `)` was expected")
            }
            ParseErrorKind::ExpIfOrBlockFound(x) => {
                format!("found {x} where if or a block was expected")
            }
            ParseErrorKind::Unreachable(_) => unreachable!(),
        }
    }
}
