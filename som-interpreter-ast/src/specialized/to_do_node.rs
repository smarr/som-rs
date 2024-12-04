use crate::evaluate::Evaluate;
use crate::invokable::{Invoke, Return};
use crate::universe::Universe;
use crate::value::Value;
use crate::vm_objects::block::Block;
use som_gc::gcref::Gc;

#[derive(Debug, Clone, PartialEq)]
pub struct ToDoNode {}

impl Invoke for ToDoNode {
    fn invoke(&mut self, universe: &mut Universe, nbr_args: usize) -> Return {
        let args = universe.stack_n_last_elems(nbr_args);

        let start_int_val = args.first().unwrap();
        let end_int_val = args.get(1).unwrap();
        let body_block_val = args.get(2).unwrap();

        if let (Some(a), Some(b), Some(c)) = (start_int_val.as_integer(), end_int_val.as_integer(), body_block_val.as_block()) {
            Self::do_int_loop(a, b, c, universe)
        } else if let (Some(a), Some(b), Some(c)) = (start_int_val.as_double(), end_int_val.as_double(), body_block_val.as_block()) {
            Self::do_double_loop(a, b, c, universe)
        } else {
            panic!(
                "to:do: was not given two ints and a block as arguments, but {:?} and {:?} and {:?}",
                start_int_val, end_int_val, body_block_val
            )
        }
    }
}

impl ToDoNode {
    fn do_int_loop(start_int: i32, end_int: i32, mut body_block: Gc<Block>, universe: &mut Universe) -> Return {
        let nbr_locals = body_block.block.nbr_locals;

        for i in start_int..=end_int {
            universe.stack_args.push(Value::Block(body_block));
            universe.stack_args.push(Value::Integer(i));
            propagate!(universe.with_frame(nbr_locals, 2, |universe| body_block.evaluate(universe),));
        }
        Return::Local(Value::Integer(start_int))
    }

    fn do_double_loop(start_double: f64, end_double: f64, mut body_block: Gc<Block>, universe: &mut Universe) -> Return {
        let nbr_locals = body_block.block.nbr_locals;
        let mut i = start_double;

        while i <= end_double {
            universe.stack_args.push(Value::Block(body_block));
            universe.stack_args.push(Value::Double(i));
            propagate!(universe.with_frame(nbr_locals, 2, |universe| body_block.evaluate(universe),));
            i += 1.0
        }

        Return::Local(Value::Double(start_double))
    }
}
