use crate::evaluate::Evaluate;
use crate::invokable::{Invoke, Return};
use crate::universe::Universe;
use crate::value::Value;
#[derive(Debug, Clone, PartialEq)]
pub struct ToByDoNode {}

impl Invoke for ToByDoNode {
    fn invoke(&mut self, universe: &mut Universe, nbr_args: usize) -> Return {
        let args = universe.stack_n_last_elems(nbr_args);

        let start_int_val = args.first().unwrap();
        let step_int_val = args.get(1).unwrap();
        let end_int_val = args.get(2).unwrap();
        let body_block_val = args.get(3).unwrap();

        let (start_int, end_int, step_int, mut body_block) = match (
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

        let mut i = start_int;

        let nbr_locals = body_block.block.nbr_locals;
        while i <= end_int {
            propagate!(
                universe.with_frame(nbr_locals, vec![Value::Block(body_block), Value::Integer(i)], |universe| body_block
                    .evaluate(universe),)
            );
            i += step_int;
        }

        Return::Local(Value::Integer(start_int))
    }
}
