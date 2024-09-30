use std::fmt::{Display, Formatter};
use std::fmt::Write;
use indenter::indented;
use crate::ast::{AstBody, AstExpression};
use crate::evaluate::Evaluate;
use crate::invokable::Return;
use crate::universe::UniverseAST;

#[derive(Debug, Clone, PartialEq)]
pub struct AndInlinedNode {
    pub first: AstExpression,
    pub second: AstBody
}

impl Display for AndInlinedNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "AndInlinedNode:")?;
        writeln!(indented(f), "first block:")?;
        write!(indented(&mut indented(f)), "{}", self.first)?;
        writeln!(indented(f), "second block:")?;
        write!(indented(&mut indented(f)), "{}", self.second)
    }
}

impl Evaluate for AndInlinedNode {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        let first_result = propagate!(self.first.evaluate(universe));
        match first_result.as_boolean() {
            Some(b) => {
                match b {
                    false => Return::Local(first_result),
                    true => {
                        match self.second.evaluate(universe) {
                            Return::Local(a) if a.is_boolean() => Return::Local(a),
                            invalid => panic!("and:'s second part didn't evaluate to a returnlocal + boolean, but {:?}?", invalid)
                        }
                    }
                }
            }
            _ => panic!("and:'s first part didn't evaluate to a boolean?")
        }
    }
}