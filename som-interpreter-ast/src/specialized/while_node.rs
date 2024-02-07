use crate::evaluate::Evaluate;
use crate::invokable::{Invoke, Return};
use crate::universe::Universe;
use crate::value::Value;
use crate::value::Value::Nil;

#[derive(Clone)]
pub struct WhileNode {
    pub expected_bool: bool // only a true atm
}

impl Invoke for WhileNode {
    fn invoke(&self, universe: &mut Universe, args: Vec<Value>) -> Return {
        let cond_block_val = universe.current_frame().borrow().get_self();
        let body_block_arg = args.get(0).unwrap();

        let cond_block = match cond_block_val {
            Value::Block(b) => b.clone(),
            _ => panic!("whileTrue method declared outside the block class")
        };

        let body_block = match body_block_arg {
            Value::Block(b) => b.clone(),
            _ => panic!("while invoked without a block?")
        };

        loop {
            let cond_val = match cond_block.block.body.evaluate(universe) { // TODO aint no way i need to declare empty vecs like this
                Return::Local(val) => val,
                not_return => panic!("Should be unreachable? Evaluated blocks return a Return::Local, instead was {:?}", not_return)
            };

            let bool_val = match cond_val {
                Value::Boolean(v) => v,
                v => panic!("Invalid, condition block should return a boolean: instead was {:?}.", v)
            };

            if bool_val == self.expected_bool {
                break Return::Local(Nil)
            } else {
                body_block.block.body.evaluate(universe);
            }
        }
    }
}