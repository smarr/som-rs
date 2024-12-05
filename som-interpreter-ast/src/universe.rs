use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::evaluate::Evaluate;
use crate::gc::{get_callbacks_for_gc, VecValue};
use crate::invokable::{Invoke, Return};
use crate::value::Value;
use crate::vm_objects::block::Block;
use crate::vm_objects::class::Class;
use crate::vm_objects::frame::{Frame, FrameAccess};
use anyhow::{anyhow, Error};
use som_core::core_classes::CoreClasses;
use som_core::interner::{Interned, Interner};
use som_gc::gc_interface::GCInterface;
use som_gc::gcref::Gc;

/// GC default heap size
pub const DEFAULT_HEAP_SIZE: usize = 1024 * 1024 * 256;

/// The central data structure for the interpreter.
///
/// It represents the complete state of the interpreter, like the known class definitions,
/// the string interner and the stack frames.
pub struct Universe {
    /// The string interner for symbols.
    pub interner: Interner,
    /// The known global bindings.
    pub globals: HashMap<String, Value>,
    /// The path to search in for new classes.
    pub classpath: Vec<PathBuf>,
    /// The current frame for the operation
    pub current_frame: Gc<Frame>,
    /// The interpreter's core classes.
    pub core: CoreClasses<Gc<Class>>,
    /// The time record of the universe's creation.
    pub start_time: Instant,
    /// GC interface
    pub gc_interface: &'static mut GCInterface,

    // we could pass arguments using the Rust stack, and we used to: but with moving GC, that makes them often unreachable, so we need to manage our own stack
    pub stack_args: Vec<Value>,
}

impl Drop for Universe {
    fn drop(&mut self) {
        let _box: Box<GCInterface> = unsafe { Box::from_raw(self.gc_interface) };
        drop(_box)
    }
}

impl Universe {
    /// Initialize the universe from the given classpath.
    pub fn with_classpath(classpath: Vec<PathBuf>) -> Result<Self, Error> {
        Self::with_classpath_and_heap_size(classpath, DEFAULT_HEAP_SIZE)
    }

    /// Initialize the universe from the given classpath, and given a heap size
    pub fn with_classpath_and_heap_size(classpath: Vec<PathBuf>, heap_size: usize) -> Result<Self, Error> {
        let interner = Interner::with_capacity(100);
        let mut globals: HashMap<String, Value> = HashMap::new();

        let gc_interface = GCInterface::init(heap_size, get_callbacks_for_gc());

        let mut core: CoreClasses<Gc<Class>> = CoreClasses::from_load_cls_fn(|name: &str, super_cls: Option<Gc<Class>>| {
            Self::load_system_class(classpath.as_slice(), name, super_cls, gc_interface).unwrap()
        });

        // TODO: these can be removed for the most part - in the AST at least, we set a lot of super class relationships when loading system classes directly.
        core.object_class.class().set_class(&core.metaclass_class);
        core.object_class.class().set_super_class(&core.class_class);

        set_super_class(&mut core.class_class, &core.object_class, &core.metaclass_class);
        set_super_class(&mut core.metaclass_class.clone(), &core.class_class, &core.metaclass_class);
        // initializeSystemClass(nilClass, objectClass, "Nil");
        set_super_class(&mut core.nil_class, &core.object_class, &core.metaclass_class);
        // initializeSystemClass(arrayClass, objectClass, "Array");
        set_super_class(&mut core.array_class, &core.object_class, &core.metaclass_class);
        // initializeSystemClass(methodClass, arrayClass, "Method");
        set_super_class(&mut core.method_class, &core.array_class, &core.metaclass_class);
        // initializeSystemClass(stringClass, objectClass, "String");
        set_super_class(&mut core.string_class, &core.object_class, &core.metaclass_class);
        // initializeSystemClass(symbolClass, stringClass, "Symbol");
        set_super_class(&mut core.symbol_class, &core.string_class, &core.metaclass_class);
        // initializeSystemClass(integerClass, objectClass, "Integer");
        set_super_class(&mut core.integer_class, &core.object_class, &core.metaclass_class);
        // initializeSystemClass(primitiveClass, objectClass, "Primitive");
        set_super_class(&mut core.primitive_class, &core.object_class, &core.metaclass_class);
        // initializeSystemClass(doubleClass, objectClass, "Double");
        set_super_class(&mut core.double_class, &core.object_class, &core.metaclass_class);

        set_super_class(&mut core.system_class, &core.object_class, &core.metaclass_class);

        set_super_class(&mut core.block_class, &core.object_class, &core.metaclass_class);
        set_super_class(&mut core.block1_class, &core.block_class, &core.metaclass_class);
        set_super_class(&mut core.block2_class, &core.block_class, &core.metaclass_class);
        set_super_class(&mut core.block3_class, &core.block_class, &core.metaclass_class);

        set_super_class(&mut core.boolean_class, &core.object_class, &core.metaclass_class);
        set_super_class(&mut core.true_class, &core.boolean_class, &core.metaclass_class);
        set_super_class(&mut core.false_class, &core.boolean_class, &core.metaclass_class);

        for (cls_name, core_cls) in core.iter() {
            globals.insert(cls_name.into(), Value::Class(*core_cls));
        }

        globals.insert("true".into(), Value::Boolean(true));
        globals.insert("false".into(), Value::Boolean(false));
        globals.insert("nil".into(), Value::NIL);
        globals.insert("system".into(), Value::SYSTEM);

        Ok(Self {
            globals,
            interner,
            classpath,
            current_frame: Gc::default(),
            start_time: Instant::now(),
            core,
            gc_interface,
            stack_args: vec![],
        })
    }

    /// Load a class from its name into this universe.
    pub fn load_class(&mut self, class_name: impl Into<String>) -> Result<Gc<Class>, Error> {
        let class_name = class_name.into();

        for path in &self.classpath {
            let mut path = path.join(class_name.as_str());
            path.set_extension("som");

            // Read file contents.
            let contents = match fs::read_to_string(path.as_path()) {
                Ok(contents) => contents,
                Err(_) => continue,
            };

            // Collect all tokens from the file.
            let tokens: Vec<_> = som_lexer::Lexer::new(contents.as_str()).skip_comments(true).skip_whitespace(true).collect();

            // Parse class definition from the tokens.
            let defn = match som_parser::parse_file(tokens.as_slice()) {
                Some(defn) => defn,
                None => continue,
            };

            if defn.name != class_name {
                return Err(anyhow!("{}: class name is different from file name.", path.display(),));
            }

            let super_class = if let Some(ref super_class) = defn.super_class {
                self.lookup_global(super_class).and_then(Value::as_class).unwrap_or_else(|| self.load_class(super_class).unwrap())
            } else {
                self.core.object_class
            };

            let mut class = Class::from_class_def(defn, Some(super_class), self.gc_interface).map_err(Error::msg)?;
            set_super_class(&mut class, &super_class, &self.core.metaclass_class);

            /*fn has_duplicated_field(class: &SOMRef<Class>) -> Option<(String, (String, String))> {
                let super_class_iterator = std::iter::successors(Some(class), |class| {
                    class.borrow().super_class()
                });
                let mut map = HashMap::<String, String>::new();
                for class in super_class_iterator {
                    let class_name = class.borrow().name().to_string();
                    for (field, _) in class.borrow().locals.iter() {
                        let field_name = field;
                        match map.entry(field_name) {
                            Entry::Occupied(entry) => {
                                return Some((field_name, (class_name, entry.get())))
                            }
                            Entry::Vacant(v) => {
                                v.insert(class_name);
                            }
                        }
                    }
                }
                return None;
            }*/

            /*if let Some((field, (c1, c2))) = has_duplicated_field(&class) {
                return Err(anyhow!(
                    "the field named '{}' is defined more than once (by '{}' and '{}', where the latter inherits from the former)",
                    field, c1, c2,
                ));
            }

            if let Some((field, (c1, c2))) = has_duplicated_field(&class.borrow().class()) {
                return Err(anyhow!(
                    "the field named '{}' is defined more than once (by '{}' and '{}', where the latter inherits from the former)",
                    field, c1, c2,
                ));
            }*/

            self.globals.insert(class.name().to_string(), Value::Class(class));

            return Ok(class);
        }

        Err(anyhow!("could not find the '{}' class", class_name))
    }

    /// Load a system class (with an incomplete hierarchy).
    pub fn load_system_class(
        classpath: &[impl AsRef<Path>],
        class_name: impl Into<String>,
        super_class: Option<Gc<Class>>,
        gc_interface: &mut GCInterface,
    ) -> Result<Gc<Class>, Error> {
        let class_name = class_name.into();
        for path in classpath {
            let mut path = path.as_ref().join(class_name.as_str());
            path.set_extension("som");

            // Read file contents.
            let contents = match fs::read_to_string(path.as_path()) {
                Ok(contents) => contents,
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => return Err(Error::from(err)),
            };

            // Collect all tokens from the file.
            let tokens: Vec<_> = som_lexer::Lexer::new(contents.as_str()).skip_comments(true).skip_whitespace(true).collect();

            // Parse class definition from the tokens.
            let defn = match som_parser::parse_file_no_universe(tokens.as_slice()) {
                Some(defn) => defn,
                None => return Err(anyhow!("could not parse the '{}' system class", class_name)),
            };

            if defn.name != class_name {
                return Err(anyhow!("{}: class name is different from file name.", path.display(),));
            }

            return Class::from_class_def(defn, super_class, gc_interface).map_err(Error::msg);
        }

        Err(anyhow!("could not find the '{}' system class", class_name))
    }
}

impl Universe {
    /// Evaluates a method or other after pushing a new frame onto the stack.
    /// The frame assumes the arguments it needs are on the global argument stack.
    pub fn eval_with_frame<T: Evaluate>(&mut self, nbr_locals: u8, nbr_args: usize, invokable: &mut T) -> Return {
        let frame = Frame::alloc_new_frame(nbr_locals, nbr_args, self);
        frame.debug_check_frame_addresses();
        self.current_frame = frame;
        let ret = invokable.evaluate(self);
        self.current_frame = self.current_frame.prev_frame;
        ret
    }

    /// Evaluates a block after pushing a new block frame.
    pub fn eval_block_with_frame(&mut self, nbr_locals: u8, nbr_args: usize) -> Return {
        let frame = Frame::alloc_new_frame(nbr_locals, nbr_args, self);
        frame.debug_check_frame_addresses();
        self.current_frame = frame;
        let mut invokable = frame.lookup_argument(0).as_block().unwrap();
        let ret = invokable.evaluate(self);
        self.current_frame = self.current_frame.prev_frame;
        ret
    }

    /// Intern a symbol.
    pub fn intern_symbol(&mut self, symbol: &str) -> Interned {
        self.interner.intern(symbol)
    }

    /// Lookup a symbol.
    pub fn lookup_symbol(&self, symbol: Interned) -> &str {
        self.interner.lookup(symbol)
    }

    /// Returns whether a global binding of the specified name exists.
    pub fn has_global(&self, name: impl AsRef<str>) -> bool {
        let name = name.as_ref();
        self.globals.contains_key(name)
    }

    /// Search for a global binding.
    pub fn lookup_global(&self, name: impl AsRef<str>) -> Option<Value> {
        let name = name.as_ref();
        self.globals.get(name).cloned()
    }

    /// Assign a value to a global binding.
    pub fn assign_global(&mut self, name: impl AsRef<str>, value: &Value) -> Option<()> {
        self.globals.insert(name.as_ref().to_string(), *value).map(|_| ())
    }

    #[inline(always)]
    /// Remove N elements off the argument stack and return them as their own vector
    pub fn stack_n_last_elems(&mut self, n: usize) -> Vec<Value> {
        let idx_split_off = self.stack_args.len() - n;
        self.stack_args.split_off(idx_split_off)
    }
}

impl Universe {
    /// Call `escapedBlock:` on the given value, if it is defined.
    pub fn escaped_block(&mut self, value: Value, block: Gc<Block>) -> Option<Return> {
        let mut initialize = value.lookup_method(self, "escapedBlock:")?;

        self.stack_args.push(value);
        self.stack_args.push(Value::Block(block));
        let escaped_block_result = initialize.invoke(self, 2);
        Some(escaped_block_result)
    }

    /// Call `doesNotUnderstand:` on the given value, if it is defined.
    pub fn does_not_understand(&mut self, value: Value, symbol: impl AsRef<str>, args: Vec<Value>) -> Option<Return> {
        let mut initialize = value.lookup_method(self, "doesNotUnderstand:arguments:")?;
        let sym = self.intern_symbol(symbol.as_ref());
        let sym = Value::Symbol(sym);
        let args = Value::Array(self.gc_interface.alloc(VecValue(args)));

        // eprintln!("Couldn't invoke {}; exiting.", symbol.as_ref()); std::process::exit(1);

        self.stack_args.push(value);
        self.stack_args.push(sym);
        self.stack_args.push(args);

        let dnu_result = initialize.invoke(self, 3);
        Some(dnu_result)
    }

    /// Call `unknownGlobal:` on the given value, if it is defined.
    pub fn unknown_global(&mut self, value: Value, name: impl AsRef<str>) -> Option<Return> {
        let sym = self.intern_symbol(name.as_ref());
        let mut method = value.lookup_method(self, "unknownGlobal:")?;

        self.stack_args.push(value);
        self.stack_args.push(Value::Symbol(sym));

        let unknown_global_result = method.invoke(self, 2);
        match unknown_global_result {
            Return::Local(value) | Return::NonLocal(value, _) => Some(Return::Local(value)),
            #[cfg(feature = "inlining-disabled")]
            Return::Restart => panic!("(from 'System>>#unknownGlobal:') incorrectly asked for a restart"),
        }
    }

    /// Call `System>>#initialize:` with the given name, if it is defined.
    pub fn initialize(&mut self, args: Vec<Value>) -> Option<Return> {
        let mut initialize = Value::SYSTEM.lookup_method(self, "initialize:")?;
        let args = Value::Array(self.gc_interface.alloc(VecValue(args)));
        self.stack_args.push(Value::SYSTEM);
        self.stack_args.push(args);
        let program_result = initialize.invoke(self, 2);
        Some(program_result)
    }
}

fn set_super_class(class: &mut Gc<Class>, super_class: &Gc<Class>, metaclass_class: &Gc<Class>) {
    class.set_super_class(super_class);

    class.class().set_super_class(&super_class.class());
    class.class().set_class(metaclass_class);
}
