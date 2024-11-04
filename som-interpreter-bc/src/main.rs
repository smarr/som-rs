//!
//! This is the interpreter for the Simple Object Machine.
//!
#![warn(missing_docs)]

use std::path::PathBuf;

use anyhow::{bail, Context};
#[cfg(feature = "jemalloc")]
use jemallocator::Jemalloc;
use som_gc::gcref::GCRef;
use som_interpreter_bc::class::Class;
use structopt::StructOpt;
mod shell;

use som_interpreter_bc::disassembler::disassemble_method_body;
use som_interpreter_bc::method::{Method, MethodKind};
#[cfg(feature = "profiler")]
use som_interpreter_bc::profiler::Profiler;
use som_interpreter_bc::universe::Universe;
use som_interpreter_bc::value::Value;
use som_interpreter_bc::{INTERPRETER_RAW_PTR, UNIVERSE_RAW_PTR};

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[derive(Debug, Clone, PartialEq, StructOpt)]
#[structopt(about, author)]
struct Options {
    /// File to evaluate.
    file: Option<PathBuf>,

    /// Arguments to pass to the `#run:` function.
    args: Vec<String>,

    /// Set search path for application classes.
    #[structopt(long, short)]
    classpath: Vec<PathBuf>,

    /// Disassemble the class, instead of executing.
    #[structopt(long, short)]
    disassemble: bool,

    /// Enable verbose output (with timing information).
    #[structopt(short = "v")]
    verbose: bool,

    /// Enable verbose output (with timing information).
    #[structopt(long, short = "hs")]
    heap_size: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    let result = run();
    #[cfg(feature = "profiler")]
    Profiler::global().drop();
    result
}

fn run() -> anyhow::Result<()> {
    let opts: Options = Options::from_args();

    // dbg!(size_of::<Bytecode>()); std::process::exit(0);

    if opts.disassemble {
        return disassemble_class(opts);
    }

    let Some(file) = opts.file else {
        panic!("I deactivated the shell out of laziness. Can be re-enabled");
        // let mut universe = Universe::with_classpath(opts.classpath)?;
        // return shell::interactive(&mut interpreter, &mut universe, opts.verbose);
    };

    let file_stem = file
        .file_stem()
        .context("the given path has no file stem")?
        .to_str()
        .context("the given path contains invalid UTF-8 in its file stem")?;

    let mut classpath = opts.classpath;
    if let Some(directory) = file.parent() {
        classpath.push(directory.to_path_buf());
    }

    let mut universe = {
        match opts.heap_size {
            Some(heap_size) => Universe::with_classpath_and_heap_size(classpath, heap_size)?,
            None => Universe::with_classpath(classpath)?,
        }
    };

    let args = std::iter::once(String::from(file_stem))
        .chain(opts.args.iter().cloned())
        .map(|arg| Value::String(universe.gc_interface.alloc(arg)))
        .collect();

    let mut interpreter = universe.initialize(args).expect("issue running program");

    unsafe {
        INTERPRETER_RAW_PTR = &mut interpreter;
        UNIVERSE_RAW_PTR = &mut universe;
    }

    interpreter.run(&mut universe);

    // let class = universe.load_class_from_path(file)?;
    // let instance = som_interpreter::instance::Instance::from_class(class);
    // let instance = Value::Instance(Rc::new(std::cell::RefCell::new(instance)));

    // let invokable = instance.lookup_method(&universe, "run").unwrap();
    // let output = som_interpreter::invokable::Invoke::invoke(invokable.as_ref(), &mut universe, vec![instance]);

    // match output {
    //     Return::Exception(message) => println!("ERROR: {}", message),
    //     Return::Restart => println!("ERROR: asked for a restart to the top-level"),
    //     _ => {}
    // }

    Ok(())
}

fn disassemble_class(opts: Options) -> anyhow::Result<()> {
    let Some(ref file) = opts.file else {
        bail!("no class specified for disassembly");
    };

    let file_stem = file
        .file_stem()
        .context("the given path has no file stem")?
        .to_str()
        .context("the given path contains invalid UTF-8 in its file stem")?;

    let mut classpath = opts.classpath.clone();
    if let Some(directory) = file.parent() {
        classpath.push(directory.to_path_buf());
    }

    let mut universe = Universe::with_classpath(classpath.clone())?;

    // "Object" special casing needed since `load_class` assumes the class has a superclass and Object doesn't, and I didn't want to change the class loading logic just for the disassembler (tho it's probably fine)
    let class = match file_stem {
        "Object" => Universe::load_system_class(
            &mut universe.interner,
            classpath.as_slice(),
            "Object",
            &mut universe.gc_interface,
        )?,
        _ => universe.load_class(file_stem)?,
    };

    dump_class_methods(class, &opts, file_stem, &mut universe);
    println!("-----------------------------------------");
    dump_class_methods(class.class, &opts, file_stem, &mut universe);

    Ok(())
}

fn dump_class_methods(
    class: GCRef<Class>,
    opts: &Options,
    file_stem: &str,
    universe: &mut Universe,
) {
    let methods: Vec<GCRef<Method>> = if opts.args.is_empty() {
        class
            .methods
            .values()
            .cloned()
            .collect::<Vec<GCRef<Method>>>()
    } else {
        opts.args
            .iter()
            .filter_map(|signature| {
                let symbol = universe.intern_symbol(signature);
                let maybe_method = class.methods.get(&symbol).cloned();

                // if maybe_method.is_none() {
                //     eprintln!("No method named `{signature}` found in class `{file_stem}`.");
                // }

                maybe_method
            })
            .collect()
    };

    for method in methods {
        let method = method;
        match &method.kind {
            MethodKind::Defined(env) => {
                println!(
                    "{class}>>#{signature} ({num_locals} locals, {num_literals} literals) (max stack size: {max_stack_size})",
                    class = file_stem,
                    signature = method.signature(),
                    num_locals = env.nbr_locals,
                    num_literals = env.literals.len(),
                    max_stack_size = env.max_stack_size,
                );

                disassemble_method_body(&universe, &class, env);
            }
            MethodKind::Primitive(_) => {
                println!(
                    "{class}>>#{signature} (primitive)",
                    class = file_stem,
                    signature = method.signature(),
                );
            }
            MethodKind::NotImplemented(_) => {
                println!(
                    "{class}>>#{signature} (not implemented)",
                    class = file_stem,
                    signature = method.signature(),
                );
            }
        }
    }
}
