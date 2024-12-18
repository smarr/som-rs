use crate::compiler::Literal;
use crate::interpreter::Interpreter;
use crate::universe::Universe;
use som_core::interner::Interned;

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialLiteralMethod {
    pub(crate) literal: Literal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialGlobalMethod {
    pub(crate) global_name: Interned,
}

impl TrivialGlobalMethod {
    pub fn evaluate(&self, universe: &mut Universe, interpreter: &mut Interpreter) {
        interpreter.current_frame.stack_pop(); // receiver off the stack.
        universe
            .lookup_global(self.global_name)
            .map(|v| interpreter.current_frame.stack_push(v))
            .or_else(|| {
                let frame = interpreter.current_frame;
                let self_value = frame.get_self();
                universe.unknown_global(interpreter, self_value, self.global_name)
            })
            .unwrap_or_else(|| panic!("global not found and unknown_global call failed somehow?"))
    }
}

// #[derive(Debug, Clone, PartialEq)]
// pub struct TrivialGetterMethod {
//     pub(crate) field_idx: u8,
// }
//
// impl Invoke for TrivialGetterMethod {
//     fn invoke(&mut self, _universe: &mut Universe, value_stack: &mut GlobalValueStack, nbr_args: usize) -> Return {
//         debug_assert_eq!(nbr_args, 1);
//         let arg = value_stack.pop();
//
//         if let Some(cls) = arg.as_class() {
//             Return::Local(cls.class().lookup_field(self.field_idx))
//         } else if let Some(instance) = arg.as_instance() {
//             Return::Local(*instance.lookup_field(self.field_idx))
//         } else {
//             panic!("trivial getter not called on a class/instance?")
//         }
//     }
// }
//
// #[derive(Debug, Clone, PartialEq)]
// pub struct TrivialSetterMethod {
//     pub(crate) field_idx: u8,
// }
//
// impl Invoke for TrivialSetterMethod {
//     fn invoke(&mut self, _universe: &mut Universe, value_stack: &mut GlobalValueStack, nbr_args: usize) -> Return {
//         debug_assert_eq!(nbr_args, 2);
//         let val = value_stack.pop();
//         let rcvr = value_stack.pop();
//
//         if let Some(cls) = rcvr.as_class() {
//             cls.class().assign_field(self.field_idx, val);
//             Return::Local(Value::Class(cls))
//         } else if let Some(mut instance) = rcvr.as_instance() {
//             instance.assign_field(self.field_idx, val);
//             Return::Local(Value::Instance(instance))
//         } else {
//             panic!("trivial getter not called on a class/instance?")
//         }
//     }
// }
