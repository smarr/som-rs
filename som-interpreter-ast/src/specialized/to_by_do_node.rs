use crate::invokable::{Invoke, Return};
use crate::universe::{GlobalValueStack, Universe};
use crate::value::Value;
use crate::vm_objects::block::Block;
use som_gc::gcref::Gc;

#[derive(Debug, Clone, PartialEq)]
pub struct ToByDoNode {}

impl Invoke for ToByDoNode {
    fn invoke(&mut self, universe: &mut Universe, value_stack: &mut GlobalValueStack, nbr_args: usize) -> Return {
        // let args = value_stack.pop_n_last(nbr_args);
        let args = value_stack.borrow_n_last(nbr_args);

        let start_int_val = args.first().unwrap();
        let step_int_val = args.get(1).unwrap();
        let end_int_val = args.get(2).unwrap();
        let body_block_val = unsafe { &*(args.get(3).unwrap() as *const Value) }; // ugly fix... to keep a ref to it while still passing the stack mutably

        let (start_int, end_int, step_int) = match (
            start_int_val.as_integer(),
            step_int_val.as_integer(),
            end_int_val.as_integer(),
            body_block_val.is_ptr::<Block, Gc<Block>>(),
        ) {
            (Some(a), Some(b), Some(c), true) => (a, b, c),
            (a, b, c, d) => panic!(
                "to:by:do: was not given three ints and a block as arguments, but {:?} and {:?} and {:?} and {:?}",
                a, b, c, d
            ),
        };

        let mut i = start_int;

        let nbr_locals = body_block_val.as_block().unwrap().block.nbr_locals;
        while i <= end_int {
            value_stack.push(*body_block_val);
            value_stack.push(Value::Integer(i));
            propagate!(universe.eval_block_with_frame(value_stack, nbr_locals, 2));
            i += step_int;
        }

        value_stack.pop_n_last(nbr_args);
        Return::Local(Value::Integer(start_int))
    }
}
