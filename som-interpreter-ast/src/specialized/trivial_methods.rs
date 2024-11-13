use crate::ast::AstLiteral;
use crate::evaluate::Evaluate;
use crate::frame::FrameAccess;
use crate::invokable::{Invoke, Return};
use crate::universe::Universe;
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialLiteralMethod {
    pub(crate) literal: AstLiteral,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialGlobalMethod {
    pub(crate) global_name: String,
}

impl Evaluate for TrivialGlobalMethod {
    fn evaluate(&mut self, universe: &mut Universe) -> Return {
        let name = self.global_name.as_str();
        // TODO: logic duplicated with globalread - need to avoid that somehow
        universe
            .lookup_global(name)
            .map(Return::Local)
            .or_else(|| {
                let frame = universe.current_frame;
                let self_value = frame.get_self();
                universe.unknown_global(self_value, name)
            })
            .unwrap_or_else(|| panic!("global not found and unknown_global call failed somehow?"))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialGetterMethod {
    pub(crate) field_idx: u8,
}

impl Invoke for TrivialGetterMethod {
    fn invoke(&mut self, _: &mut Universe, args: Vec<Value>) -> Return {
        let arg = args.first().unwrap();

        if let Some(cls) = arg.as_class() {
            Return::Local(cls.class().lookup_field(self.field_idx))
        } else if let Some(instance) = arg.as_instance() {
            Return::Local(instance.lookup_local(self.field_idx))
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
    fn invoke(&mut self, _: &mut Universe, args: Vec<Value>) -> Return {
        let val = args.get(1).unwrap();
        let rcvr = args.first().unwrap();

        if let Some(cls) = rcvr.as_class() {
            cls.class().assign_field(self.field_idx, *val);
            Return::Local(Value::Class(cls))
        } else if let Some(mut instance) = rcvr.as_instance() {
            instance.assign_local(self.field_idx, *val);
            Return::Local(Value::Instance(instance))
        } else {
            panic!("trivial getter not called on a class/instance?")
        }
    }
}
