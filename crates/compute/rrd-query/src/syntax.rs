//! `rrflowQL`: a small, explicit query grammar that lowers into `RRD query engine`.
//!
//! Parsing is intentionally separate from binding and execution. The parser
//! validates syntax and identifiers, but only a catalog can decide whether a
//! type or property exists. Every query must state valid time and known state;
//! there is no ambient "latest" hidden in the language contract.

use rrd_core::{Predicate, RuntimeRef, RuntimeType, RuntimeValue};
use serde::{Deserialize, Serialize};
use std::fmt;

mod ast;
mod parse;
mod parser;

pub use ast::*;
pub use parse::*;
use parser::*;
