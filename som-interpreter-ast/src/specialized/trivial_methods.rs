use crate::ast::AstLiteral;
use crate::evaluate::Evaluate;
use crate::invokable::{Invoke, Return};
use crate::universe::{GlobalValueStack, Universe};
use crate::value::Value;
use crate::vm_objects::frame::FrameAccess;
use som_core::interner::Interned;
#[derive(Debug, Clone, PartialEq)]
pub struct TrivialLiteralMethod {
    pub(crate) literal: AstLiteral,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialGlobalMethod {
    pub(crate) global_name: Interned,
}

impl Evaluate for TrivialGlobalMethod {
    fn evaluate(&mut self, universe: &mut Universe, value_stack: &mut GlobalValueStack) -> Return {
        // TODO: logic duplicated with globalread - need to avoid that somehow
        universe
            .lookup_global(self.global_name)
            .map(Return::Local)
            .or_else(|| {
                let frame = universe.current_frame;
                let self_value = frame.get_self();
                universe.unknown_global(value_stack, self_value, self.global_name)
            })
            .unwrap_or_else(|| panic!("global not found and unknown_global call failed somehow?"))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialGetterMethod {
    pub(crate) field_idx: u8,
}

impl Invoke for TrivialGetterMethod {
    fn invoke(&mut self, _universe: &mut Universe, value_stack: &mut GlobalValueStack, nbr_args: usize) -> Return {
        let args = value_stack.pop_n_last(nbr_args);

        let arg = args.first().unwrap();

        if let Some(cls) = arg.as_class() {
            Return::Local(cls.class().lookup_field(self.field_idx))
        } else if let Some(instance) = arg.as_instance() {
            Return::Local(*instance.lookup_field(self.field_idx))
        } else {
            panic!("trivial getter not called on a class/instance?")
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialSetterMethod {
    pub(crate) field_idx: u8,
}

impl Invoke for TrivialSetterMethod {
    fn invoke(&mut self, _universe: &mut Universe, value_stack: &mut GlobalValueStack, nbr_args: usize) -> Return {
        let args = value_stack.pop_n_last(nbr_args);

        let val = args.get(1).unwrap();
        let rcvr = args.first().unwrap();

        if let Some(cls) = rcvr.as_class() {
            cls.class().assign_field(self.field_idx, *val);
            Return::Local(Value::Class(cls))
        } else if let Some(mut instance) = rcvr.as_instance() {
            instance.assign_field(self.field_idx, *val);
            Return::Local(Value::Instance(instance))
        } else {
            panic!("trivial getter not called on a class/instance?")
        }
    }
}
