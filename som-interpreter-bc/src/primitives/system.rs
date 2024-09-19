use std::convert::TryInto;
use std::fs;
use std::io::Write;

use crate::class::Class;
use crate::convert::{Nil, Primitive, StringLike, System};
use crate::interner::Interned;
use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseBC;
use crate::value::Value;
use anyhow::{Context, Error};
use once_cell::sync::Lazy;
use som_core::gc::GCRef;

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

fn load_file(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    _: Value,
    path: StringLike,
) -> Result<Option<GCRef<String>>, Error> {
    const _: &str = "System>>#loadFie:";

    let path = match path {
        StringLike::String(ref string) => string.to_obj().as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    let Ok(value) = fs::read_to_string(path) else {
        return Ok(None);
    };

    Ok(Some(universe.gc_interface.allocate(value)))
}

fn print_string(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    _: Value,
    string: StringLike,
) -> Result<System, Error> {
    const _: &str = "System>>#printString:";

    let string = match string {
        StringLike::String(ref string) => string.to_obj().as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    print!("{string}");
    std::io::stdout().flush()?;

    Ok(System)
}

fn print_newline(
    _: &mut Interpreter,
    _: &mut UniverseBC,
    _: Value,
) -> Result<Nil, Error> {
    const _: &'static str = "System>>#printNewline";

    println!();

    Ok(Nil)
}

fn error_print(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    _: Value,
    string: StringLike,
) -> Result<System, Error> {
    const _: &str = "System>>#errorPrint:";

    let string = match string {
        StringLike::String(ref string) => string.to_obj().as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    eprint!("{string}");
    std::io::stderr().flush()?;

    Ok(System)
}

fn error_println(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    _: Value,
    string: StringLike,
) -> Result<System, Error> {
    const _: &str = "System>>#errorPrintln:";

    let string = match string {
        StringLike::String(ref string) => string.to_obj().as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    eprintln!("{string}");

    Ok(System)
}

fn load(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    _: Value,
    class_name: Interned,
) -> Result<GCRef<Class>, Error> {
    const _: &str = "System>>#load:";

    let class_name = universe.lookup_symbol(class_name).to_string();
    let class = universe.load_class(class_name)?;

    Ok(class)
}

fn has_global(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    _: Value,
    name: Interned,
) -> Result<bool, Error> {
    const _: &str = "System>>#hasGlobal:";

    Ok(universe.has_global(name))
}

fn global(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    _: Value,
    name: Interned,
) -> Result<Option<Value>, Error> {
    const _: &str = "System>>#global:";

    Ok(universe.lookup_global(name))
}

fn global_put(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    _: Value,
    name: Interned,
    value: Value,
) -> Result<Option<Value>, Error> {
    const _: &str = "System>>#global:put:";

    Ok(universe.assign_global(name, value.clone()).map(|_| value))
}

fn exit(_: &mut Interpreter, _: &mut UniverseBC, status: i32) -> Result<(), Error> {
    const _: &str = "System>>#exit:";

    std::process::exit(status);
}

fn ticks(
    interpreter: &mut Interpreter,
    _: &mut UniverseBC,
    _: Value,
) -> Result<i32, Error> {
    const SIGNATURE: &str = "System>>#ticks";

    interpreter
        .start_time
        .elapsed()
        .as_micros()
        .try_into()
        .with_context(|| format!("`{SIGNATURE}`: could not convert `i128` to `i32`"))
}

fn time(
    interpreter: &mut Interpreter,
    _: &mut UniverseBC,
    _: Value,
) -> Result<i32, Error> {
    const SIGNATURE: &str = "System>>#time";

    interpreter
        .start_time
        .elapsed()
        .as_millis()
        .try_into()
        .with_context(|| format!("`{SIGNATURE}`: could not convert `i128` to `i32`"))
}

fn print_stack_trace(
    _: &mut Interpreter,
    _: &mut UniverseBC,
    _: Value,
) -> Result<bool, Error> {
    const _: &str = "System>>#printStackTrace";

    todo!("no stack trace implemented. it's doable, though")
/*    for frame in &interpreter.frames {
        let frame_ref = frame.borrow();
        let class = frame_ref.get_method_holder();
        let method = frame_ref.get_method();
        let bytecode_idx = frame_ref.bytecode_idx;
        let block = match frame_ref.kind() {
            FrameKind::Block { .. } => "$block",
            _ => "",
        };
        println!(
            "{}>>#{}{} @bi: {}",
            class.borrow().name(),
            method.signature(),
            block,
            bytecode_idx
        );
    }

    Ok(true)*/
}

fn full_gc(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    _: Value,
) -> Result<bool, Error> {
    const _: &str = "System>>#fullGC";

    universe.gc_interface.full_gc_request();

    Ok(true)
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
