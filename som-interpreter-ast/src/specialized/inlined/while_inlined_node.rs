use std::fmt::Write;
use std::fmt::{Display, Formatter};

use crate::ast::AstBody;
use crate::evaluate::Evaluate;
use crate::invokable::Return;
use crate::universe::{GlobalValueStack, Universe};
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
    fn evaluate(&mut self, universe: &mut Universe, value_stack: &mut GlobalValueStack) -> Return {
        loop {
            let cond_result = propagate!(self.cond_instrs.evaluate(universe, value_stack));
            debug_assert!(cond_result.is_boolean()); // and since it's not a pointer, we don't need to push it to the stack to keep it reachable for GC
            if cond_result.as_boolean_unchecked() != self.expected_bool {
                break;
            } else {
                propagate!(self.body_instrs.evaluate(universe, value_stack));
            }
        }
        Return::Local(Value::NIL)
    }
}
