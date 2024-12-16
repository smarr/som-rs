use crate::invokable::{Invoke, Return};
use crate::universe::{GlobalValueStack, Universe};
use crate::value::Value;
use crate::vm_objects::block::Block;
use som_gc::gcref::Gc;

#[derive(Debug, Clone, PartialEq)]
pub struct DownToDoNode {}

impl Invoke for DownToDoNode {
    fn invoke(&mut self, universe: &mut Universe, value_stack: &mut GlobalValueStack, nbr_args: usize) -> Return {
        let args = value_stack.borrow_n_last(nbr_args);
        let start_int_val = args.first().unwrap();
        let end_int_val = args.get(1).unwrap();
        let body_block_val = unsafe { &*(args.get(2).unwrap() as *const Value) }; // ugly fix... to keep a ref to it while still passing the stack mutably

        let ret = {
            if let (Some(a), Some(b), true) = (
                start_int_val.as_integer(),
                end_int_val.as_integer(),
                body_block_val.is_ptr::<Block, Gc<Block>>(),
            ) {
                Self::do_int_loop(a, b, body_block_val, universe, value_stack)
            } else if let (Some(a), Some(b), true) = (
                start_int_val.as_double(),
                end_int_val.as_double(),
                body_block_val.is_ptr::<Block, Gc<Block>>(),
            ) {
                Self::do_double_loop(a, b, body_block_val, universe, value_stack)
            } else {
                panic!(
                    "downTo:do: was not given two numbers and a block as arguments, but {:?} and {:?} and {:?}",
                    start_int_val, end_int_val, body_block_val
                )
            }
        };

        value_stack.pop_n_last(nbr_args);

        ret
    }
}

impl DownToDoNode {
    fn do_int_loop(start_int: i32, end_int: i32, body_block_val: &Value, universe: &mut Universe, value_stack: &mut GlobalValueStack) -> Return {
        let nbr_locals = body_block_val.as_block().unwrap().block.nbr_locals;
        let mut i = start_int;
        while i >= end_int {
            value_stack.push(Value::Integer(i));
            match universe.eval_block_with_frame_no_pop(value_stack, nbr_locals, 2) {
                Return::Local(..) => {}
                ret => {
                    value_stack.pop();
                    return ret;
                }
            };
            value_stack.pop();
            i -= 1;
        }
        Return::Local(Value::Integer(start_int))
    }

    fn do_double_loop(
        start_double: f64,
        end_double: f64,
        body_block_val: &Value,
        universe: &mut Universe,
        value_stack: &mut GlobalValueStack,
    ) -> Return {
        let nbr_locals = body_block_val.as_block().unwrap().block.nbr_locals;
        let mut i = start_double;
        while i >= end_double {
            value_stack.push(Value::Double(i));
            match universe.eval_block_with_frame_no_pop(value_stack, nbr_locals, 2) {
                Return::Local(..) => {}
                ret => {
                    value_stack.pop();
                    return ret;
                }
            };
            value_stack.pop();
            i -= 1.0;
        }
        Return::Local(Value::Double(start_double))
    }
}
