use crate::evaluate::Evaluate;
use crate::invokable::{Invoke, Return};
use crate::universe::Universe;
use crate::value::Value;
#[derive(Debug, Clone, PartialEq)]
pub struct IfNode {
    pub(crate) expected_bool: bool,
}

impl Invoke for IfNode {
    fn invoke(&mut self, universe: &mut Universe, nbr_args: usize) -> Return {
        let args = universe.stack_n_last_elems(nbr_args);

        let cond_block_val = unsafe { args.get_unchecked(0) };
        let body_block_arg = unsafe { args.get_unchecked(1) };

        let (bool_val, mut body_block) = match (cond_block_val.as_boolean(), body_block_arg.as_block()) {
            (Some(b), Some(blk)) => (b, blk),
            (a, b) => panic!("if[True|False] was not given a bool and a block as arguments, but {:?} and {:?}", a, b),
        };

        let nbr_locals = body_block.block.nbr_locals;

        if bool_val != self.expected_bool {
            Return::Local(Value::NIL)
        } else {
            universe.stack_args.push(Value::Block(body_block));
            universe.with_frame(nbr_locals, 1, |universe| body_block.evaluate(universe))
        }
    }
}
