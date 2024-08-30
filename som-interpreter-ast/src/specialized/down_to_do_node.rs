use std::rc::Rc;
use crate::block::Block;
use crate::evaluate::Evaluate;
use crate::invokable::{Invoke, Return};
use crate::SOMRef;
use crate::universe::UniverseAST;
use crate::value::Value;

#[derive(Clone)]
pub struct DownToDoNode {}

impl Invoke for DownToDoNode {
    fn invoke_somref(self_: SOMRef<Self>, universe: &mut UniverseAST, args: Vec<Value>) -> Return {
        self_.borrow_mut().invoke(universe, args)
    }
    
    fn invoke(&mut self, universe: &mut UniverseAST, args: Vec<Value>) -> Return {
        let start_int_val = args.first().unwrap();
        let end_int_val = args.get(1).unwrap();
        let body_block_val = args.get(2).unwrap();

        match (start_int_val, end_int_val, body_block_val) {
            (Value::Integer(a), Value::Integer(b), Value::Block(c)) => Self::do_int_loop(*a, *b, c.clone(), universe),
            (Value::Double(a), Value::Double(b), Value::Block(c)) => Self::do_double_loop(*a, *b, c.clone(), universe),
            (a, b, c) => panic!("downTo:do: was not given two ints and a block as arguments, but {:?} and {:?} and {:?}", a, b, c)
        }
    }
}

impl DownToDoNode {
    fn do_int_loop(start_int: i64, end_int: i64, mut body_block: SOMRef<Block>, universe: &mut UniverseAST) -> Return {
        let nbr_locals = body_block.borrow().block.borrow().nbr_locals;
        let mut i = start_int;
        while i >= end_int {
            propagate!(universe.with_frame(
                nbr_locals,
                vec![Value::Block(Rc::clone(&body_block)), Value::Integer(i)],
                |universe| body_block.evaluate(universe),
            ));
            i -= 1;
        }
        Return::Local(Value::Integer(start_int))
    }

    fn do_double_loop(start_double: f64, end_double: f64, mut body_block: SOMRef<Block>, universe: &mut UniverseAST) -> Return {
        let nbr_locals = body_block.borrow().block.borrow().nbr_locals;
        let mut i = start_double;
        while i >= end_double {
            propagate!(universe.with_frame(
                nbr_locals,
                vec![Value::Block(Rc::clone(&body_block)), Value::Double(i)],
                |universe| body_block.evaluate(universe),
            ));
            i -= 1.0;
        }
        Return::Local(Value::Double(start_double))
    }
}