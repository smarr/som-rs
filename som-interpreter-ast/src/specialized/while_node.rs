use std::cell::RefCell;
use std::rc::Rc;
use crate::evaluate::Evaluate;
use crate::invokable::{Invoke, Return};
use crate::universe::UniverseAST;
use crate::value::Value;
use crate::value::Value::Nil;

#[derive(Clone)]
pub struct WhileNode {
    pub(crate) expected_bool: bool
}

impl Invoke for WhileNode {
    fn invoke_somref(self_: Rc<RefCell<Self>>, universe: &mut UniverseAST, args: Vec<Value>) -> Return {
        self_.borrow_mut().invoke(universe, args)
    }
    
    fn invoke(&mut self, universe: &mut UniverseAST, args: Vec<Value>) -> Return {
        let cond_block_val = unsafe { args.get_unchecked(0) };
        let body_block_arg = unsafe { args.get_unchecked(1) };

        let (mut cond_block, mut body_block) = match (cond_block_val, body_block_arg) {
            (Value::Block(b), Value::Block(c)) => (b.clone(), c.clone()),
            _ => panic!("while[True|False] was not given two blocks as arguments")
        };

        let nbr_locals = cond_block.borrow().block.borrow().nbr_locals;
        
        loop {
            let cond_block_return = universe.with_frame(
                nbr_locals,
                vec![Value::Block(Rc::clone(&cond_block))],
                |universe| cond_block.evaluate(universe),
            );

            let bool_val = match cond_block_return {
                Return::Local(Value::Boolean(b)) => b,
                v => panic!("Invalid, condition block should return a boolean: instead was {:?}.", v)
            };

            if bool_val != self.expected_bool {
                return Return::Local(Nil)
            } else {
                let nbr_locals = body_block.borrow().block.borrow().nbr_locals;
                
                propagate!(universe.with_frame(
                    nbr_locals,
                    vec![Value::Block(Rc::clone(&body_block))],
                    |universe| body_block.evaluate(universe),
                ));
            }
        }
    }
}