use once_cell::sync::Lazy;
use crate::instance::Instance;
use crate::invokable::Return;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use som_core::gc::GCRef;
use crate::class::Class;
use crate::convert::Primitive;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> = Lazy::new(|| {
    Box::new({
        [
            ("new", self::new.into_func(), true),
            ("name", self::name.into_func(), true),
            ("fields", self::fields.into_func(), true),
            ("methods", self::methods.into_func(), true),
            ("superclass", self::superclass.into_func(), true),
        ]
    })
});
pub static CLASS_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> =
    Lazy::new(|| Box::new([]));

fn superclass(_: &mut Universe, receiver: GCRef<Class>) -> Return {
    let super_class = receiver.borrow().super_class();
    Return::Local(super_class.map(Value::Class).unwrap_or(Value::NIL))
}

fn new(universe: &mut Universe, receiver: GCRef<Class>) -> Return {
    let instance = Instance::from_class(receiver);
    let instance_ptr = GCRef::<Instance>::alloc(instance, &mut universe.gc_interface);
    Return::Local(Value::Instance(instance_ptr))
}

fn name(universe: &mut Universe, receiver: GCRef<Class>) -> Return {
    let sym = universe.intern_symbol(receiver.borrow().name());
    Return::Local(Value::Symbol(sym))
}

fn methods(universe: &mut Universe, receiver: GCRef<Class>) -> Return {
    let methods = receiver
        .borrow()
        .methods
        .values()
        .map(|invokable| Value::Invokable(invokable.clone()))
        .collect();

    Return::Local(Value::Array(GCRef::<Vec<Value>>::alloc(methods, &mut universe.gc_interface)))
}

fn fields(universe: &mut Universe, receiver: GCRef<Class>) -> Return {
    let fields = receiver.borrow().get_all_field_names().iter()
        .map(|field_name| Value::String(GCRef::<String>::alloc(field_name.clone(), &mut universe.gc_interface)))
        .collect();

    Return::Local(Value::Array(GCRef::<Vec<Value>>::alloc(fields, &mut universe.gc_interface)))
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES
        .iter()
        .find(|it| it.0 == signature)
        .map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES
        .iter()
        .find(|it| it.0 == signature)
        .map(|it| it.1)
}