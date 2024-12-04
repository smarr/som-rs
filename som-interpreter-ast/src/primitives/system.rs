use crate::convert::{Primitive, StringLike};
use crate::gc::VecValue;
use crate::primitives::{PrimInfo, PrimitiveFn};
use crate::universe::Universe;
use crate::value::Value;
use anyhow::{bail, Context, Error};
use once_cell::sync::Lazy;
use som_core::interner::Interned;
use som_gc::gcref::Gc;
use std::convert::TryFrom;
use std::fs;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| {
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
        ("gcStats", self::gc_stats.into_func(), true),
        ("exit:", self::exit.into_func(), true),
        ("global:", self::global.into_func(), true),
        ("global:put:", self::global_put.into_func(), true),
        ("hasGlobal:", self::has_global.into_func(), true),
        ("printStackTrace", self::print_stack_trace.into_func(), true),
    ])
});
pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

fn load_file(universe: &mut Universe, _: Value, path: StringLike) -> Result<Value, Error> {
    let path = match path {
        StringLike::String(ref string) => string,
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    match fs::read_to_string(path) {
        Ok(value) => Ok(Value::String(universe.gc_interface.alloc(value))),
        Err(_) => Ok(Value::NIL),
    }
}

fn print_string(universe: &mut Universe, _: Value, string: StringLike) -> Result<Value, Error> {
    let string = match string {
        StringLike::String(ref string) => string,
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    print!("{}", string);
    Ok(Value::SYSTEM)
}

fn print_newline(_: &mut Universe, _: Value) -> Result<Value, Error> {
    println!();
    Ok(Value::NIL)
}

fn error_print(universe: &mut Universe, _: Value, string: StringLike) -> Result<Value, Error> {
    let string = match string {
        StringLike::String(ref string) => string,
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    eprint!("{}", string);
    Ok(Value::SYSTEM)
}

fn error_println(universe: &mut Universe, _: Value, string: StringLike) -> Result<Value, Error> {
    const _: &str = "System>>#errorPrintln:";

    let string = match string {
        StringLike::String(ref string) => string,
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    eprintln!("{}", string);
    Ok(Value::SYSTEM)
}

fn load(universe: &mut Universe, _: Value, class_name: Interned) -> Result<Value, Error> {
    const SIGNATURE: &str = "System>>#load:";

    let name = universe.lookup_symbol(class_name).to_string();

    if let Some(cached_class) = universe.lookup_global(&name) {
        if cached_class.is_class() {
            return Ok(cached_class);
        }
    }

    match universe.load_class(name) {
        Ok(class) => Ok(Value::Class(class)),
        Err(err) => bail!(format!("'{}': {}", SIGNATURE, err)),
    }
}

fn has_global(universe: &mut Universe, _: Value, name: Interned) -> Result<Value, Error> {
    const _: &str = "System>>#hasGlobal:";
    let symbol = universe.lookup_symbol(name);
    Ok(Value::Boolean(universe.has_global(symbol)))
}

fn global(universe: &mut Universe, _: Value, name: Interned) -> Result<Value, Error> {
    let symbol = universe.lookup_symbol(name);
    Ok(universe.lookup_global(symbol).unwrap_or(Value::NIL))
}

fn global_put(universe: &mut Universe, _: Value, name: Interned, value: Value) -> Result<Value, Error> {
    let symbol = universe.lookup_symbol(name).to_string();
    universe.assign_global(symbol, &value);
    Ok(value)
}

fn exit(_: &mut Universe, status: i32) -> Result<Value, Error> {
    const _: &str = "System>>#exit:";
    std::process::exit(status)
}

fn ticks(universe: &mut Universe, _: Value) -> Result<Value, Error> {
    const SIGNATURE: &str = "System>>#ticks";

    let x = universe
        .start_time
        .elapsed()
        .as_micros()
        .try_into()
        .with_context(|| format!("`{SIGNATURE}`: could not convert `i128` to `i32`"))
        .unwrap();

    Ok(Value::Integer(x))
}

fn time(universe: &mut Universe, _: Value) -> Result<Value, Error> {
    const SIGNATURE: &str = "System>>#time";

    match i32::try_from(universe.start_time.elapsed().as_millis()) {
        Ok(micros) => Ok(Value::Integer(micros)),
        Err(err) => bail!(format!("'{}': {}", SIGNATURE, err)),
    }
}

// this function is unusable after my recent changes to the frame. needs to be fixed when a compilation flag for frame debug info is enabled
fn print_stack_trace(_: &mut Universe, _: Value) -> Result<bool, Error> {
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
    Ok(true)
}

fn full_gc(universe: &mut Universe, _: Value) -> Result<Value, Error> {
    Ok(Value::Boolean(universe.gc_interface.full_gc_request()))
}

fn gc_stats(universe: &mut Universe, _: Value) -> Result<Gc<VecValue>, Error> {
    let gc_interface = &universe.gc_interface;
    let total_gc = gc_interface.get_nbr_collections();
    let total_gc_time = gc_interface.get_total_gc_time();
    let total_bytes_alloc = gc_interface.get_used_bytes();

    Ok(universe.gc_interface.alloc(VecValue(vec![
        Value::Integer(total_gc as i32),
        Value::Integer(total_gc_time as i32),
        Value::Integer(total_bytes_alloc as i32),
    ])))
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}
