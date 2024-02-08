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
        let cond_block_val = args.get(0).unwrap();
        let body_block_arg = args.get(1).unwrap();

        let (cond_block, body_block) = match (cond_block_val, body_block_arg) {
            (Value::Block(b), Value::Block(c)) => (b.clone(), c.clone()),
            _ => panic!("while[True|False] was not given two blocks as arguments")
        };

        loop {
            let bool_val = match cond_block.invoke(universe, args.clone()) {
                Return::Local(Value::Boolean(b)) => b,
                v => panic!("Invalid, condition block should return a boolean: instead was {:?}.", v)
            };

            if bool_val != self.expected_bool {
                break Return::Local(Nil)
            } else {
                let ret_val = body_block.invoke(universe, vec![]);
                match ret_val {
                    Return::Exception(e) => panic!("Exception thrown: {}", e),
                    Return::Restart => {},
                    ret => break ret // TODO shouldn't return non locals be handled as return locals? but then again we are already in the right scope... hmm
                }
            }
        }
    }
}