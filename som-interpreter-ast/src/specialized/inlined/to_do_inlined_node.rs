use std::fmt::{Display, Formatter};

use crate::ast::{AstBody, AstExpression};
use crate::evaluate::Evaluate;
use crate::invokable::Return;
use crate::universe::Universe;
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ToDoInlinedNode {
    pub start: AstExpression,
    pub end: AstExpression,
    pub body: AstBody,
    pub accumulator_idx: usize,
}

impl Display for ToDoInlinedNode {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Evaluate for ToDoInlinedNode {
    fn evaluate(&mut self, universe: &mut Universe) -> Return {
        let start = propagate!(self.start.evaluate(universe));
        let end = propagate!(self.end.evaluate(universe));

        if let (Some(start_int), Some(end_int)) = (start.as_integer(), end.as_integer()) {
            self.int_loop(universe, start_int, end_int)
        } else if let (Some(start_double), Some(end_double)) = (start.as_double(), end.as_double()) {
            self.float_loop(universe, start_double, end_double)
        } else {
            unimplemented!("to:do: case that isn't int nor float")
        }
    }
}

impl ToDoInlinedNode {
    fn int_loop(&mut self, universe: &mut Universe, start: i32, end: i32) -> Return {
        for i in start..=end {
            universe.assign_local(self.accumulator_idx as u8, &Value::Integer(i));
            propagate!(self.body.evaluate(universe));
        }

        Return::Local(Value::Integer(start))
    }

    fn float_loop(&mut self, universe: &mut Universe, start: f64, end: f64) -> Return {
        let mut i = start;

        while i <= end {
            universe.assign_local(self.accumulator_idx as u8, &Value::Double(i));
            propagate!(self.body.evaluate(universe));
            i += 1.0
        }

        Return::Local(Value::Double(start))
    }
}
