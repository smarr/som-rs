use crate::compiler::{value_from_literal, Literal};
use crate::interpreter::Interpreter;
use crate::universe::Universe;
use crate::vm_objects::instance::Instance;
use som_core::interner::Interned;

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialLiteralMethod {
    pub(crate) literal: Literal,
}

impl TrivialLiteralMethod {
    pub fn evaluate(&self, universe: &mut Universe, interpreter: &mut Interpreter) {
        interpreter.current_frame.stack_pop(); // receiver

        let value_from_literal = value_from_literal(&interpreter.current_frame, &self.literal, universe.gc_interface);
        interpreter.current_frame.stack_push(value_from_literal)
    }
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

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialGetterMethod {
    pub(crate) field_idx: u8,
}

impl TrivialGetterMethod {
    pub fn invoke(&self, _universe: &mut Universe, interpreter: &mut Interpreter) {
        let arg = interpreter.current_frame.stack_pop();

        if let Some(cls) = arg.as_class() {
            interpreter.current_frame.stack_push(cls.class().lookup_field(self.field_idx as usize))
        } else if let Some(instance) = arg.as_instance() {
            interpreter.current_frame.stack_push(*Instance::lookup_field(instance, self.field_idx as usize))
        } else {
            panic!("trivial getter not called on a class/instance?")
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialSetterMethod {
    pub(crate) field_idx: u8,
}

impl TrivialSetterMethod {
    pub fn invoke(&self, _universe: &mut Universe, interpreter: &mut Interpreter) {
        let val = interpreter.current_frame.stack_pop();
        let rcvr = interpreter.current_frame.stack_last();

        if let Some(cls) = rcvr.as_class() {
            cls.class().assign_field(self.field_idx as usize, val);
        } else if let Some(instance) = rcvr.as_instance() {
            Instance::assign_field(instance, self.field_idx as usize, val)
        } else {
            panic!("trivial getter not called on a class/instance?")
        }
    }
}
