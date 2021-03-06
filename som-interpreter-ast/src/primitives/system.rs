use std::convert::TryFrom;
// use std::io::BufRead;
// use std::rc::Rc;

use crate::expect_args;
use crate::invokable::Return;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;

// fn read_line(_: &mut Universe, args: Vec<Value>) -> Return {
//     const SIGNATURE: &str = "System>>#readLine";

//     expect_args!(SIGNATURE, args, [Value::System]);

//     match std::io::stdin().lock().lines().next() {
//         Some(Ok(line)) => Return::Local(Value::String(Rc::new(line))),
//         Some(Err(err)) => Return::Exception(format!("'{}': {}", SIGNATURE, err)),
//         None => Return::Exception(format!("'{}': {}", SIGNATURE, "error")),
//     }
// }

fn print_string(universe: &mut Universe, args: Vec<Value>) -> Return {
    const SIGNATURE: &str = "System>>#printString:";

    expect_args!(SIGNATURE, args, [
        Value::System,
        value => value,
    ]);

    let string = match value {
        Value::String(ref string) => string,
        Value::Symbol(sym) => universe.lookup_symbol(sym),
        _ => return Return::Exception(format!("'{}': wrong type", SIGNATURE)),
    };

    print!("{}", string);
    Return::Local(Value::System)
}

fn print_newline(_: &mut Universe, args: Vec<Value>) -> Return {
    const SIGNATURE: &'static str = "System>>#printNewline";

    expect_args!(SIGNATURE, args, [Value::System]);

    println!();
    Return::Local(Value::Nil)
}

fn load(universe: &mut Universe, args: Vec<Value>) -> Return {
    const SIGNATURE: &str = "System>>#load:";

    expect_args!(SIGNATURE, args, [
        Value::System,
        Value::Symbol(sym) => sym,
    ]);

    let name = universe.lookup_symbol(sym).to_string();
    match universe.load_class(name) {
        Ok(class) => Return::Local(Value::Class(class)),
        Err(err) => Return::Exception(format!("'{}': {}", SIGNATURE, err)),
    }
}

fn global(universe: &mut Universe, args: Vec<Value>) -> Return {
    const SIGNATURE: &str = "System>>#global:";

    expect_args!(SIGNATURE, args, [
        Value::System,
        Value::Symbol(sym) => sym,
    ]);

    let symbol = universe.lookup_symbol(sym);
    Return::Local(universe.lookup_global(symbol).unwrap_or(Value::Nil))
}

fn global_put(universe: &mut Universe, args: Vec<Value>) -> Return {
    const SIGNATURE: &str = "System>>#global:put:";

    expect_args!(SIGNATURE, args, [
        Value::System,
        Value::Symbol(sym) => sym,
        value => value,
    ]);

    let symbol = universe.lookup_symbol(sym).to_string();
    universe.assign_global(symbol, value.clone());
    Return::Local(value)
}

fn exit(_: &mut Universe, args: Vec<Value>) -> Return {
    const SIGNATURE: &str = "System>>#exit:";

    expect_args!(SIGNATURE, args, [
        Value::System,
        Value::Integer(code) => code,
    ]);

    match i32::try_from(code) {
        Ok(code) => std::process::exit(code),
        Err(err) => Return::Exception(format!("'{}': {}", SIGNATURE, err)),
    }
}

fn ticks(universe: &mut Universe, args: Vec<Value>) -> Return {
    const SIGNATURE: &str = "System>>#ticks";

    expect_args!(SIGNATURE, args, [Value::System]);

    match i64::try_from(universe.start_time.elapsed().as_micros()) {
        Ok(micros) => Return::Local(Value::Integer(micros)),
        Err(err) => Return::Exception(format!("'{}': {}", SIGNATURE, err)),
    }
}

fn time(universe: &mut Universe, args: Vec<Value>) -> Return {
    const SIGNATURE: &str = "System>>#time";

    expect_args!(SIGNATURE, args, [Value::System]);

    match i64::try_from(universe.start_time.elapsed().as_millis()) {
        Ok(micros) => Return::Local(Value::Integer(micros)),
        Err(err) => Return::Exception(format!("'{}': {}", SIGNATURE, err)),
    }
}

fn full_gc(_: &mut Universe, args: Vec<Value>) -> Return {
    const SIGNATURE: &str = "System>>#fullGC";

    expect_args!(SIGNATURE, args, [Value::System]);

    // We don't do any garbage collection at all, so we return false.
    Return::Local(Value::Boolean(false))
}

/// Search for a primitive matching the given signature.
pub fn get_primitive(signature: impl AsRef<str>) -> Option<PrimitiveFn> {
    match signature.as_ref() {
        // "readLine" => Some(self::read_line),
        "printString:" => Some(self::print_string),
        "printNewline" => Some(self::print_newline),
        "load:" => Some(self::load),
        "ticks" => Some(self::ticks),
        "time" => Some(self::time),
        "fullGC" => Some(self::full_gc),
        "exit:" => Some(self::exit),
        "global:" => Some(self::global),
        "global:put:" => Some(self::global_put),
        _ => None,
    }
}
