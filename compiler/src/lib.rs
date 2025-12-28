#![allow(clippy::let_unit_value)]
#![feature(assert_matches)]
#![feature(try_trait_v2)]
#![feature(never_type)]

pub mod ast;
pub mod disjoint_set;
pub mod env;
pub mod error;
pub mod green_builder;
pub mod green_node;
pub mod interner;
pub mod lexer;
pub mod name_resolution;
pub mod parser;
pub mod red_node;
pub mod syntax;
pub mod token;
pub mod ty;
pub mod type_ck;
pub mod value;
pub mod visitor;
