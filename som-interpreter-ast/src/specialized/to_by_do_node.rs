use crate::invokable::{Invoke, Return};
use crate::universe::{GlobalValueStack, Universe};
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ToByDoNode {}

impl Invoke for ToByDoNode {
    fn invoke(&mut self, universe: &mut Universe, value_stack: &mut GlobalValueStack, nbr_args: usize) -> Return {
        // let args = value_stack.pop_n_last(nbr_args);
        let args = value_stack.borrow_n_last(nbr_args);
        let start_int_val = &args[0];
        let step_int_val = &args[1];
        let end_int_val = &args[2];
        let body_block_val = &args[3];

        let (start_int, end_int, step_int, body_block) = match (
            start_int_val.as_integer(),
            step_int_val.as_integer(),
            end_int_val.as_integer(),
            body_block_val.as_block(),
        ) {
            (Some(a), Some(b), Some(c), Some(d)) => (a, b, c, d),
            (a, b, c, d) => panic!(
                "to:by:do: was not given three ints and a block as arguments, but {:?} and {:?} and {:?} and {:?}",
                a, b, c, d
            ),
        };

        let nbr_locals = body_block.block.nbr_locals;
        let mut i = start_int;
        while i <= end_int {
            value_stack.push(Value::Integer(i));

            match universe.eval_block_with_frame_no_pop(value_stack, nbr_locals, 2) {
                Return::Local(..) => {}
                ret => {
                    value_stack.pop_n_last(nbr_args + 1); // all the arguments, and also including the integer index
                    return ret;
                }
            };

            value_stack.pop();
            i += step_int;
        }

        value_stack.pop_n_last(nbr_args);
        Return::Local(Value::Integer(start_int))
    }
}
