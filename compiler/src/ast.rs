use std::{
    collections::VecDeque,
    fmt::{Debug, Display},
};

use strum::EnumIter;

use crate::{error::span::Span, interner::Symbol, token::TokenKind, ty::TypeId, value::Value};

pub trait Pass {
    type XArg: Debug;
    type XRet: Debug;
    type XVar: Debug;
    type XTy: Debug;
    type XId: Debug;
}

#[derive(Debug)]
pub struct Untyped;
#[derive(Debug)]
pub struct Annotated;
#[derive(Debug)]
pub struct Typed;

/// Initial state of the AST
impl Pass for Untyped {
    type XArg = (Ident, Ident);
    type XRet = Option<Ident>;
    type XVar = (Ident, Option<Ident>);
    type XTy = Ident;
    type XId = AstId;
}

/// All the provided types have been resolved (name resolution) + defined in the type system and
/// they aren't `TyIdent`s but `Ty`s now.
impl Pass for Annotated {
    type XArg = (Ident, TypeId);
    type XRet = Option<TypeId>;
    type XVar = (Ident, Option<TypeId>);
    type XTy = TypeId;
    type XId = ResolvedAstId;
}

impl Pass for Typed {
    type XArg = (Ident, TypeId);
    type XRet = TypeId;
    type XVar = (Ident, TypeId);
    type XTy = TypeId;
    type XId = ResolvedAstId;
}

pub type Itself<P> = Box<InnerAst<P>>;

/// The spans of higher level things are the sums of the spans of their components
#[derive(Debug)]
pub enum Ast<P: Pass> {
    /// The name of the function, the parameters to the function, block, and return type
    FunDeclaration {
        name: Ident,
        // params: Vec<(BindIdent, Option<TyIdent>)>,
        params: Vec<P::XArg>,
        // ret: Option<TyIdent>,
        ret: P::XRet,
        block: Itself<P>,
    },
    Struct {
        name: Ident,
        fields: Vec<(Ident, P::XTy)>,
    },
    ForLoop {
        decl: Itself<P>,
        predicate: Itself<P>,
        assignment: Itself<P>,
        block: Itself<P>,
    },
    If {
        predicate: Itself<P>,
        then: Itself<P>,
        otherwise: Option<Itself<P>>, // this is `(Block || If)`
    },
    Application {
        name: Ident,
        params: Vec<Itself<P>>,
    },
    ImplBlock(VecDeque<InnerAst<P>>), // only allowed to be function definitions essentially
    Block(VecDeque<InnerAst<P>>),
    Declaration(P::XVar, Option<Itself<P>>),
    Assignment(Ident, Itself<P>),
    BinOp(Op, Itself<P>, Itself<P>),
    UnaryOp(Op, Itself<P>),
    FnInvoc(Ident, Option<Vec<InnerAst<P>>>),
    Ident(Ident),
    Atom(Value),
}

#[derive(Debug, Clone, Copy)]
/// A unique identifier for each item in the Ast
pub struct AstId(pub usize);

#[derive(Debug)]
/// Essentially an interned path to the namespace in which this item can be located
pub struct OwnerId(pub usize);

#[derive(Debug)]
pub struct ResolvedAstId {
    owner_id: OwnerId,
    ast_id: AstId,
}

#[derive(Debug)]
pub struct InnerAst<P: Pass> {
    pub inner: Ast<P>,
    pub id: P::XId,
}

// impl<P: Pass> Iterator for Ast<P> {
//     /// Whether it is something that needs to be matched on or not
//     type Item = Option<Ast<P>>;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         match self {
//             Ast::Block(vec) => Some(vec.pop_front()),
//             _ => Some(None),
//         }
//     }
// }

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
/// An identifier to the function, type, or variable
pub struct Ident {
    pub sym: Symbol,
    pub width: usize,
}

#[derive(Debug)]
pub struct Op {
    pub kind: OpKind,
    pub ty: OpType,
    pub span: Span,
}

#[derive(Debug, strum::Display, EnumIter)]
pub enum OpKind {
    Add,
    Mult,
    Div,
    Sub,
    Greater,
    Less,
    Equal,
    GreaterEqual,
    LessEqual,
    Or,
    And,
}

#[derive(Debug, strum::Display, EnumIter)]
pub enum OpType {
    /// 2 + 1
    IntOp,
    /// 2 +. 1
    FloatOp,
    /// 2 > 1
    IntCmp,
    /// 2 >. 1
    FloatCmp,
    /// Generic == and !=
    AnyCmp,
}

impl TryFrom<TokenKind> for OpKind {
    type Error = ();

    fn try_from(value: TokenKind) -> Result<Self, Self::Error> {
        match value {
            TokenKind::Plus => Ok(OpKind::Add),
            TokenKind::Minus => Ok(OpKind::Sub),
            TokenKind::Star => Ok(OpKind::Mult),
            TokenKind::Slash => Ok(OpKind::Div),
            TokenKind::Greater => Ok(OpKind::Greater),
            TokenKind::Less => Ok(OpKind::Less),
            TokenKind::EqualEqual => Ok(OpKind::Equal),
            TokenKind::LessEqual => Ok(OpKind::LessEqual),
            TokenKind::GreaterEqual => Ok(OpKind::GreaterEqual),
            TokenKind::DoubleAmpersand => Ok(OpKind::And),
            TokenKind::DoublePipe => Ok(OpKind::Or),
            _ => Err(()),
        }
    }
}

impl From<TokenKind> for OpType {
    fn from(value: TokenKind) -> Self {
        use TokenKind::*;
        match value {
            Plus => Self::IntOp,
            Minus => Self::IntOp,
            Greater => Self::IntCmp,
            GreaterEqual => Self::IntCmp,
            Less => Self::IntCmp,
            LessEqual => Self::IntCmp,
            x => unreachable!(
                "{} should have been filtered out by pratt parsing algo before this via try_from",
                x
            ),
        }
    }
}

// impl Display for Expr {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Expr::BinOp(token_kind, er, e1) => write!(f, "({} {} {})", token_kind, er, e1),
//             Expr::UnaryOp(token_kind, e) => write!(f, "({}{})", token_kind, e),
//             Expr::Atom(ty) => write!(f, "{:?}", ty),
//             Expr::FnInvoc(ident, x) => write!(f, "{:?}({:?})", ident, x),
//             Expr::Ident(ident) => write!(f, "{:?}", ident),
//         }
//     }
// }

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}
