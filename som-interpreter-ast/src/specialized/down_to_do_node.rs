use crate::invokable::{Invoke, Return};
use crate::universe::{GlobalValueStack, Universe};
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct DownToDoNode {}

impl Invoke for DownToDoNode {
    fn invoke(&mut self, universe: &mut Universe, value_stack: &mut GlobalValueStack, nbr_args: usize) -> Return {
        let args = value_stack.borrow_n_last(nbr_args);
        let start_int_val = &args[0];
        let end_int_val = &args[1];
        let body_block_val = &args[2];

        let ret = {
            if let (Some(a), Some(b), Some(c)) = (start_int_val.as_integer(), end_int_val.as_integer(), body_block_val.as_block()) {
                Self::do_int_loop(a, b, c.block.nbr_locals, universe, value_stack)
            } else if let (Some(a), Some(b), Some(c)) = (start_int_val.as_double(), end_int_val.as_double(), body_block_val.as_block()) {
                Self::do_double_loop(a, b, c.block.nbr_locals, universe, value_stack)
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
    fn do_int_loop(start_int: i32, end_int: i32, nbr_locals: u8, universe: &mut Universe, value_stack: &mut GlobalValueStack) -> Return {
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

    fn do_double_loop(start_double: f64, end_double: f64, nbr_locals: u8, universe: &mut Universe, value_stack: &mut GlobalValueStack) -> Return {
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
