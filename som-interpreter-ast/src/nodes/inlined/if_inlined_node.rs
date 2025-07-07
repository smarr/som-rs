use crate::ast::{AstBody, AstExpression};
use crate::evaluate::Evaluate;
use crate::invokable::Return;
use crate::universe::{GlobalValueStack, Universe};
use crate::value::Value;
use indenter::indented;
use std::fmt::Write;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq)]
pub struct IfInlinedNode {
    pub expected_bool: bool,
    pub cond_expr: AstExpression,
    pub body_instrs: AstBody,
}

impl Display for IfInlinedNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "IfInlinedNode (expected bool: {}):", self.expected_bool)?;
        writeln!(indented(f), "condition expr:")?;
        write!(indented(&mut indented(f)), "{}", self.cond_expr)?;
        writeln!(indented(f), "body block:")?;
        write!(indented(&mut indented(f)), "{}", self.body_instrs)
    }
}

impl Evaluate for IfInlinedNode {
    fn evaluate(&mut self, universe: &mut Universe, value_stack: &mut GlobalValueStack) -> Return {
        let cond_result = propagate!(self.cond_expr.evaluate(universe, value_stack));
        debug_assert!(cond_result.is_boolean());
        if cond_result.as_boolean_unchecked() == self.expected_bool {
            self.body_instrs.evaluate(universe, value_stack)
        } else {
            Return::Local(Value::NIL)
        }
    }
}
