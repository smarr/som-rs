use som_core::ast::Literal;
use crate::evaluate::Evaluate;
use crate::invokable::Return;
use crate::universe::UniverseAST;

#[derive(Clone)]
pub struct TrivialLiteralMethod {
    pub(crate) literal: Literal
}

#[derive(Clone)]
pub struct TrivialGlobalMethod {
    pub(crate) global_name: String
}

impl Evaluate for TrivialGlobalMethod {
    fn evaluate(&self, universe: &mut UniverseAST) -> Return {
        let name = self.global_name.as_str();
        // todo logic duplicated with globalread - need to avoid that
        universe.lookup_global(name)
            .map(Return::Local)
            .or_else(|| {
                let frame = universe.current_frame();
                let self_value = frame.borrow().get_self();
                universe.unknown_global(self_value, name)
            })
            .unwrap_or_else(|| Return::Exception(format!("global variable '{}' not found", name)))
    }
}