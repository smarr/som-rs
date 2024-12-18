use crate::interpreter::Interpreter;
use crate::universe::Universe;
use som_core::interner::Interned;

#[derive(Debug, Clone, PartialEq)]
pub struct TrivialGlobalMethod {
    pub(crate) global_name: Interned,
}

impl TrivialGlobalMethod {
    pub fn evaluate(&self, universe: &mut Universe, interpreter: &mut Interpreter) {
        interpreter.current_frame.stack_pop();
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
