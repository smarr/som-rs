use crate::evaluate::Evaluate;
use crate::invokable::{Invoke, Return};
use crate::universe::Universe;
use crate::value::Value;
#[derive(Debug, Clone, PartialEq)]
pub struct IfTrueIfFalseNode {}

impl Invoke for IfTrueIfFalseNode {
    fn invoke(&mut self, universe: &mut Universe, nbr_args: usize) -> Return {
        let args = universe.stack_n_last_elems(nbr_args);
        let (cond_block_val, block_1_arg, block_2_arg) = unsafe { (args.get_unchecked(0), args.get_unchecked(1), args.get_unchecked(2)) };

        let bool_val = match cond_block_val.as_boolean() {
            Some(a) => a,
            _ => panic!("ifTrue:ifFalse: condition did not evaluate to boolean"),
        };

        // let (bool_val, body_block, body_block2) = match (cond_block_val, body_block_arg, body_block_arg2) {
        //     (Value::Boolean(a), Value::Block(b), Value::Block(c)) => (*a, b.clone(), c.clone()),
        //     (a, b, c) => panic!("ifTrue:ifFalse: was not given a bool and two blocks as arguments, but {:?} and {:?} and {:?}", a, b, c)
        // };

        let block_to_evaluate = if bool_val { block_1_arg } else { block_2_arg };

        match block_to_evaluate.as_block() {
            Some(mut b) => {
                let nbr_locals = b.block.nbr_locals;
                universe.with_frame(nbr_locals, vec![Value::Block(b)], |universe| b.evaluate(universe))
            }
            None => Return::Local(*block_to_evaluate),
        }
    }
}
