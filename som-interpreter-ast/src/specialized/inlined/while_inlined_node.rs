use std::fmt::Write;
use std::fmt::{Display, Formatter};

use crate::ast::AstBody;
use crate::evaluate::Evaluate;
use crate::invokable::Return;
use crate::universe::Universe;
use crate::value::Value;
use indenter::indented;

#[derive(Debug, Clone, PartialEq)]
pub struct WhileInlinedNode {
    pub expected_bool: bool,
    pub cond_instrs: AstBody,
    pub body_instrs: AstBody,
}

impl Display for WhileInlinedNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "WhileInlinedNode (expected bool: {}):", self.expected_bool)?;
        writeln!(indented(f), "condition block:")?;
        write!(indented(&mut indented(f)), "{}", self.cond_instrs)?;
        writeln!(indented(f), "body block:")?;
        write!(indented(&mut indented(f)), "{}", self.body_instrs)
    }
}

impl Evaluate for WhileInlinedNode {
    fn evaluate(&mut self, universe: &mut Universe) -> Return {
        loop {
            let cond_result = propagate!(self.cond_instrs.evaluate(universe));
            debug_assert!(cond_result.is_boolean());
            if cond_result.as_boolean_unchecked() != self.expected_bool {
                break;
            } else {
                propagate!(self.body_instrs.evaluate(universe));
            }
        }
        Return::Local(Value::NIL)
    }
}
