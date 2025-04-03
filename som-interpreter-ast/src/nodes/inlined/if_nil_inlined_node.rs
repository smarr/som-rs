use crate::ast::{AstBody, AstExpression};
use crate::evaluate::Evaluate;
use crate::invokable::Return;
use crate::universe::{GlobalValueStack, Universe};
use indenter::indented;
use std::fmt::Write;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq)]
pub struct IfNilInlinedNode {
    pub expects_nil: bool,
    pub cond_expr: AstExpression,
    pub body_instrs: AstBody,
}

impl Display for IfNilInlinedNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "IfNilInlinedNode (expected bool: {}):", self.expects_nil)?;
        writeln!(indented(f), "condition expr:")?;
        write!(indented(&mut indented(f)), "{}", self.cond_expr)?;
        writeln!(indented(f), "body block:")?;
        write!(indented(&mut indented(f)), "{}", self.body_instrs)
    }
}

impl Evaluate for IfNilInlinedNode {
    fn evaluate(&mut self, universe: &mut Universe, value_stack: &mut GlobalValueStack) -> Return {
        let cond_result = propagate!(self.cond_expr.evaluate(universe, value_stack));
        match (self.expects_nil, cond_result.is_nil()) {
            (true, true) | (false, false) => self.body_instrs.evaluate(universe, value_stack),
            (false, true) | (true, false) => Return::Local(cond_result),
        }
    }
}
