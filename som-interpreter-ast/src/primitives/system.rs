use crate::convert::{Primitive, StringLike};
use crate::invokable::Return;
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseAST;
use crate::value::Value;
use anyhow::Context;
use once_cell::sync::Lazy;
use som_core::gc::GCRef;
use som_core::interner::Interned;
use std::convert::TryFrom;
use std::fs;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> = Lazy::new(|| {
    Box::new([
        ("loadFile:", self::load_file.into_func(), true),
        ("printString:", self::print_string.into_func(), true),
        ("printNewline", self::print_newline.into_func(), true),
        ("errorPrint:", self::error_print.into_func(), true),
        ("errorPrintln:", self::error_println.into_func(), true),
        ("load:", self::load.into_func(), true),
        ("ticks", self::ticks.into_func(), true),
        ("time", self::time.into_func(), true),
        ("fullGC", self::full_gc.into_func(), true),
        ("exit:", self::exit.into_func(), true),
        ("global:", self::global.into_func(), true),
        ("global:put:", self::global_put.into_func(), true),
        ("hasGlobal:", self::has_global.into_func(), true),
        ("printStackTrace", self::print_stack_trace.into_func(), true),
    ])
});
pub static CLASS_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> =
    Lazy::new(|| Box::new([]));

fn load_file(universe: &mut UniverseAST, _: Value, path: StringLike) -> Return {
    let path = match path {
        StringLike::String(ref string) => string.to_obj(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    match fs::read_to_string(path) {
        Ok(value) => Return::Local(Value::String(GCRef::<String>::alloc(value, &mut universe.gc_interface))),
        Err(_) => Return::Local(Value::NIL),
    }
}

fn print_string(universe: &mut UniverseAST, _: Value, string: StringLike) -> Return {
    let string = match string {
        StringLike::String(ref string) => string.to_obj(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    print!("{}", string);
    Return::Local(Value::SYSTEM)
}

fn print_newline(_: &mut UniverseAST, _: Value) -> Return {
    println!();
    Return::Local(Value::NIL)
}

fn error_print(universe: &mut UniverseAST, _: Value, string: StringLike) -> Return {
    let string = match string {
        StringLike::String(ref string) => string.to_obj(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    eprint!("{}", string);
    Return::Local(Value::SYSTEM)
}

fn error_println(universe: &mut UniverseAST, _: Value, string: StringLike) -> Return {
    const _: &str = "System>>#errorPrintln:";

    let string = match string {
        StringLike::String(ref string) => string.to_obj(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    eprintln!("{}", string);
    Return::Local(Value::SYSTEM)
}

fn load(universe: &mut UniverseAST, _: Value, class_name: Interned) -> Return {
    const SIGNATURE: &str = "System>>#load:";

    let name = universe.lookup_symbol(class_name).to_string();

    if let Some(cached_class) = universe.lookup_global(&name) {
        if cached_class.is_class() {
            return Return::Local(cached_class);
        }
    }

    match universe.load_class(name) {
        Ok(class) => Return::Local(Value::Class(class)),
        Err(err) => Return::Exception(format!("'{}': {}", SIGNATURE, err)),
    }
}

fn has_global(universe: &mut UniverseAST, _: Value, name: Interned) -> Return {
    const _: &str = "System>>#hasGlobal:";
    let symbol = universe.lookup_symbol(name);
    Return::Local(Value::Boolean(universe.has_global(symbol)))
}

fn global(universe: &mut UniverseAST, _: Value, name: Interned) -> Return {
    let symbol = universe.lookup_symbol(name);
    Return::Local(universe.lookup_global(symbol).unwrap_or(Value::NIL))
}

fn global_put(universe: &mut UniverseAST, _: Value, name: Interned, value: Value) -> Return {
    let symbol = universe.lookup_symbol(name).to_string();
    universe.assign_global(symbol, &value);
    Return::Local(value)
}

fn exit(_: &mut UniverseAST, status: i32) -> Return {
    const _: &str = "System>>#exit:";
    std::process::exit(status)
}

fn ticks(universe: &mut UniverseAST, _: Value) -> Return {
    const SIGNATURE: &str = "System>>#ticks";

    let x = universe.start_time
        .elapsed()
        .as_micros()
        .try_into()
        .with_context(|| format!("`{SIGNATURE}`: could not convert `i128` to `i32`")).unwrap();
    
    Return::Local(Value::Integer(x))
}

fn time(universe: &mut UniverseAST, _: Value) -> Return {
    const SIGNATURE: &str = "System>>#time";

    match i32::try_from(universe.start_time.elapsed().as_millis()) {
        Ok(micros) => Return::Local(Value::Integer(micros)),
        Err(err) => Return::Exception(format!("'{}': {}", SIGNATURE, err)),
    }
}

// this function is unusable after my recent changes to the frame. needs to be fixed when a compilation flag for frame debug info is enabled
fn print_stack_trace(_: &mut UniverseAST, _: Vec<Value>) -> Return {
    // const SIGNATURE: &str = "System>>#printStackTrace";

    dbg!("printStackTrace is broken (on purpose). It can be fixed and reenabled with a debug flag, though.");
    /*
            for frame in &universe.frames {
            // let class = frame.borrow().get_method_holder(universe);
            // let signature = frame.borrow().get_method_signature();
            // let signature = universe.lookup_symbol(signature);
            let signature = "we do not support method signatures in stack traces anymore...";
            // let block = match frame.borrow().kind() {
            //     FrameKind::Block { .. } => "$block",
            //     _ => "",
            // };
            // println!("{}>>#{}{}", class.borrow().name(), signature, block);
            println!("{}>>#{}", class.borrow().name(), signature);
        }
    */
    Return::Local(Value::Boolean(true))
}

fn full_gc(_: &mut UniverseAST, _: Value) -> Return {
    // We don't do any garbage collection at all, so we return false.
    Return::Local(Value::Boolean(false))
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