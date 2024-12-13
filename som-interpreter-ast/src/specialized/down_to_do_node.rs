use crate::invokable::{Invoke, Return};
use crate::universe::Universe;
use crate::value::Value;
use crate::vm_objects::block::Block;
use som_gc::debug_assert_valid_semispace_ptr;
use som_gc::gcref::Gc;

#[derive(Debug, Clone, PartialEq)]
pub struct DownToDoNode {}

impl Invoke for DownToDoNode {
    fn invoke(&mut self, universe: &mut Universe, stack_args: &mut Vec<Value>, nbr_args: usize) -> Return {
        // let args = unsafe { &*(Universe::stack_borrow_n_last_elems(nbr_args) as *const [Value]) };
        let args = Universe::stack_n_last_elems(stack_args, nbr_args);
        let start_int_val = args.first().unwrap();
        let end_int_val = args.get(1).unwrap();
        let body_block_val = args.get(2).unwrap();

        if let (Some(a), Some(b), Some(c)) = (start_int_val.as_integer(), end_int_val.as_integer(), body_block_val.as_block()) {
            Self::do_int_loop(a, b, &c, universe, stack_args)
        } else if let (Some(a), Some(b), Some(c)) = (start_int_val.as_double(), end_int_val.as_double(), body_block_val.as_block()) {
            Self::do_double_loop(a, b, &c, universe, stack_args)
        } else {
            panic!(
                "downTo:do: was not given two numbers and a block as arguments, but {:?} and {:?} and {:?}",
                start_int_val, end_int_val, body_block_val
            )
        }
    }
}

impl DownToDoNode {
    fn do_int_loop(start_int: i32, end_int: i32, body_block: &Gc<Block>, universe: &mut Universe, stack_args: &mut Vec<Value>) -> Return {
        let nbr_locals = body_block.block.nbr_locals;
        let mut i = start_int;
        while i >= end_int {
            debug_assert_valid_semispace_ptr!(body_block);
            stack_args.push(Value::Block(*body_block));
            stack_args.push(Value::Integer(i));
            propagate!(universe.eval_block_with_frame(stack_args, nbr_locals, 2));
            i -= 1;
        }
        Return::Local(Value::Integer(start_int))
    }

    fn do_double_loop(start_double: f64, end_double: f64, body_block: &Gc<Block>, universe: &mut Universe, stack_args: &mut Vec<Value>) -> Return {
        let nbr_locals = body_block.block.nbr_locals;
        let mut i = start_double;
        while i >= end_double {
            stack_args.push(Value::Block(*body_block));
            stack_args.push(Value::Double(i));
            propagate!(universe.eval_block_with_frame(stack_args, nbr_locals, 2));
            i -= 1.0;
        }
        Return::Local(Value::Double(start_double))
    }
}
