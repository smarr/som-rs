use crate::compiler::compile::compile_class;
use crate::gc::{get_callbacks_for_gc, VecValue};
use crate::interpreter::Interpreter;
use crate::value::Value;
use crate::vm_objects::block::Block;
use crate::vm_objects::class::Class;
use crate::vm_objects::frame::Frame;
use anyhow::{anyhow, Error};
use som_core::core_classes::CoreClasses;
use som_core::interner::{Interned, Interner};
use som_gc::gc_interface::GCInterface;
use som_gc::gcref::Gc;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
    // pub globals: HashMap<Interned, Value>,
    pub globals: Vec<(Interned, Value)>,
    /// The path to search in for new classes.
    pub classpath: Vec<PathBuf>,
    /// The interpreter's core classes.
    pub core: CoreClasses<Gc<Class>>,
    /// GC interface for GC operations
    pub gc_interface: &'static mut GCInterface,
}

impl Universe {
    /// Initialize the universe from the given classpath (and default heap size).
    pub fn with_classpath(classpath: Vec<PathBuf>) -> Result<Self, Error> {
        Self::with_classpath_and_heap_size(classpath, DEFAULT_HEAP_SIZE)
    }

    /// Initialize the universe from the given classpath and heap size.
    pub fn with_classpath_and_heap_size(classpath: Vec<PathBuf>, heap_size: usize) -> Result<Self, Error> {
        let mut interner = Interner::with_capacity(100);
        let mut globals = vec![];

        let gc_interface = GCInterface::init(heap_size, get_callbacks_for_gc());

        let object_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Object", gc_interface)?;
        let mut class_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Class", gc_interface)?;
        let metaclass_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Metaclass", gc_interface)?;

        let mut nil_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Nil", gc_interface)?;
        let mut integer_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Integer", gc_interface)?;
        let mut array_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Array", gc_interface)?;
        let mut method_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Method", gc_interface)?;
        let mut symbol_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Symbol", gc_interface)?;
        let mut primitive_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Primitive", gc_interface)?;
        let mut string_class = Self::load_system_class(&mut interner, classpath.as_slice(), "String", gc_interface)?;
        let mut system_class = Self::load_system_class(&mut interner, classpath.as_slice(), "System", gc_interface)?;
        let mut double_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Double", gc_interface)?;

        let mut block_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Block", gc_interface)?;
        let mut block1_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Block1", gc_interface)?;
        let mut block2_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Block2", gc_interface)?;
        let mut block3_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Block3", gc_interface)?;

        let mut boolean_class = Self::load_system_class(&mut interner, classpath.as_slice(), "Boolean", gc_interface)?;
        let mut true_class = Self::load_system_class(&mut interner, classpath.as_slice(), "True", gc_interface)?;
        let mut false_class = Self::load_system_class(&mut interner, classpath.as_slice(), "False", gc_interface)?;

        // initializeSystemClass(objectClass, null, "Object");
        // set_super_class(&object_class, &nil_class, &metaclass_class);
        object_class.class().set_class(&metaclass_class);
        object_class.class().set_super_class(&class_class);
        // initializeSystemClass(classClass, objectClass, "Class");
        set_super_class(&mut class_class, &object_class, &metaclass_class);
        // initializeSystemClass(metaclassClass, classClass, "Metaclass");
        set_super_class(&mut metaclass_class.clone(), &class_class, &metaclass_class);
        // initializeSystemClass(nilClass, objectClass, "Nil");
        set_super_class(&mut nil_class, &object_class, &metaclass_class);
        // initializeSystemClass(arrayClass, objectClass, "Array");
        set_super_class(&mut array_class, &object_class, &metaclass_class);
        // initializeSystemClass(methodClass, arrayClass, "Method");
        set_super_class(&mut method_class, &array_class, &metaclass_class);
        // initializeSystemClass(stringClass, objectClass, "String");
        set_super_class(&mut string_class, &object_class, &metaclass_class);
        // initializeSystemClass(symbolClass, stringClass, "Symbol");
        set_super_class(&mut symbol_class, &string_class, &metaclass_class);
        // initializeSystemClass(integerClass, objectClass, "Integer");
        set_super_class(&mut integer_class, &object_class, &metaclass_class);
        // initializeSystemClass(primitiveClass, objectClass, "Primitive");
        set_super_class(&mut primitive_class, &object_class, &metaclass_class);
        // initializeSystemClass(doubleClass, objectClass, "Double");
        set_super_class(&mut double_class, &object_class, &metaclass_class);

        set_super_class(&mut system_class, &object_class, &metaclass_class);

        set_super_class(&mut block_class, &object_class, &metaclass_class);
        set_super_class(&mut block1_class, &block_class, &metaclass_class);
        set_super_class(&mut block2_class, &block_class, &metaclass_class);
        set_super_class(&mut block3_class, &block_class, &metaclass_class);

        set_super_class(&mut boolean_class, &object_class, &metaclass_class);
        set_super_class(&mut true_class, &boolean_class, &metaclass_class);
        set_super_class(&mut false_class, &boolean_class, &metaclass_class);

        #[rustfmt::skip] {
            globals.push((interner.intern("Object"), Value::Class(object_class)));
            globals.push((interner.intern("Class"), Value::Class(class_class)));
            globals.push((interner.intern("Metaclass"), Value::Class(metaclass_class)));
            globals.push((interner.intern("Nil"), Value::Class(nil_class)));
            globals.push((interner.intern("Integer"), Value::Class(integer_class)));
            globals.push((interner.intern("Array"), Value::Class(array_class)));
            globals.push((interner.intern("Method"), Value::Class(method_class)));
            globals.push((interner.intern("Symbol"), Value::Class(symbol_class)));
            globals.push((interner.intern("Primitive"), Value::Class(primitive_class)));
            globals.push((interner.intern("String"), Value::Class(string_class)));
            globals.push((interner.intern("System"), Value::Class(system_class)));
            globals.push((interner.intern("Double"), Value::Class(double_class)));
            globals.push((interner.intern("Boolean"), Value::Class(boolean_class)));
            globals.push((interner.intern("True"), Value::Class(true_class)));
            globals.push((interner.intern("False"), Value::Class(false_class)));
            globals.push((interner.intern("Block"), Value::Class(block_class)));
            globals.push((interner.intern("Block1"), Value::Class(block1_class)));
            globals.push((interner.intern("Block2"), Value::Class(block2_class)));
            globals.push((interner.intern("Block3"), Value::Class(block3_class)));

            globals.push((interner.intern("true"), Value::Boolean(true)));
            globals.push((interner.intern("false"), Value::Boolean(false)));
            globals.push((interner.intern("nil"), Value::NIL));
            globals.push((interner.intern("system"), Value::SYSTEM));
        };

        Ok(Self {
            globals,
            interner,
            classpath,
            core: CoreClasses {
                object_class,
                class_class,
                metaclass_class,
                nil_class,
                integer_class,
                array_class,
                method_class,
                symbol_class,
                primitive_class,
                string_class,
                system_class,
                double_class,
                block_class,
                block1_class,
                block2_class,
                block3_class,
                boolean_class,
                true_class,
                false_class,
            },
            gc_interface,
        })
    }

    /// Load a class from its name into this universe.
    pub fn load_class(&mut self, class_name: impl Into<String>) -> Result<Gc<Class>, Error> {
        let class_name = class_name.into();

        for path in self.classpath.iter() {
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
                let symbol = self.intern_symbol(super_class.as_str());
                match self.lookup_global(symbol) {
                    v if v.is_some() && v.unwrap().is_class() => v.unwrap().as_class().unwrap(),
                    _ => self.load_class(super_class)?,
                }
            } else {
                self.core.object_class
            };

            let mut class =
                compile_class(&mut self.interner, &defn, Some(&super_class), self.gc_interface).ok_or_else(|| Error::msg(String::new()))?;
            set_super_class(&mut class, &super_class, &self.core.metaclass_class);

            let symbol = self.intern_symbol(class.name());
            self.globals.push((symbol, Value::Class(class)));

            return Ok(class);
        }

        Err(anyhow!("could not find the '{}' class", class_name))
    }

    /// Load a system class (with an incomplete hierarchy).
    pub fn load_system_class(
        interner: &mut Interner,
        classpath: &[impl AsRef<Path>],
        class_name: impl Into<String>,
        allocator: &mut GCInterface,
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
            let class = compile_class(interner, &defn, None, allocator).ok_or_else(|| Error::msg(String::new()))?;

            return Ok(class);
        }

        Err(anyhow!("could not find the '{}' system class", class_name))
    }

    /// Intern a symbol.
    pub fn intern_symbol(&mut self, symbol: &str) -> Interned {
        self.interner.intern(symbol)
    }

    pub fn has_global(&self, idx: Interned) -> bool {
        self.globals.iter().any(|(interned, _)| *interned == idx)
    }

    /// Lookup a symbol.
    pub fn lookup_symbol(&self, symbol: Interned) -> &str {
        self.interner.lookup(symbol)
    }

    /// Search for a global binding.
    pub fn lookup_global(&self, idx: Interned) -> Option<Value> {
        self.globals.iter().find(|(interned, _)| *interned == idx).map(|(_, value)| *value)
    }

    /// Assign a value to a global binding.
    pub fn assign_global(&mut self, name: Interned, value: Value) {
        self.globals.push((name, value));
    }
}

impl Universe {
    /// Call `escapedBlock:` on the given value, if it is defined.
    pub fn escaped_block(&mut self, interpreter: &mut Interpreter, value: Value, block: Gc<Block>) -> Option<()> {
        let method_name = self.intern_symbol("escapedBlock:");
        let method = value.lookup_method(self, method_name)?;
        interpreter.push_method_frame_with_args(method, &[value, Value::Block(block)], self.gc_interface);
        Some(())
    }

    /// Call `doesNotUnderstand:` on the given value, if it is defined.
    #[allow(unreachable_code, unused_variables)]
    pub fn does_not_understand(&mut self, interpreter: &mut Interpreter, value: Value, symbol: Interned, args: Vec<Value>) -> Option<()> {
        // dbg!(&interpreter.stack);
        // panic!("does not understand: {:?}, called on {:?}", self.interner.lookup(symbol), &value);

        let method_name = self.intern_symbol("doesNotUnderstand:arguments:");
        let method = value.lookup_method(self, method_name)?;

        // #[cfg(debug_assertions)]
        // {
        //     let stack_trace_fn = crate::primitives::system::get_instance_primitive("printStackTrace")?;
        //     stack_trace_fn(interpreter, self).expect("couldn't print stack trace");
        //     std::process::exit(1);
        // }

        interpreter.push_method_frame_with_args(
            method,
            &[value, Value::Symbol(symbol), Value::Array(self.gc_interface.alloc(VecValue(args)))],
            self.gc_interface,
        );

        Some(())
    }

    /// Call `unknownGlobal:` on the given value, if it is defined.
    pub fn unknown_global(&mut self, interpreter: &mut Interpreter, value: Value, name: Interned) -> Option<()> {
        let method_name = self.intern_symbol("unknownGlobal:");
        let method = value.lookup_method(self, method_name)?;

        interpreter.current_frame.bytecode_idx = interpreter.bytecode_idx;
        interpreter.push_method_frame_with_args(method, &[value, Value::Symbol(name)], self.gc_interface);

        Some(())
    }

    /// Call `System>>#initialize:` with the given name, if it is defined.
    pub fn initialize(&mut self, args: Vec<Value>) -> Option<Interpreter> {
        let method_name = self.interner.intern("initialize:");
        let method = Value::SYSTEM.lookup_method(self, method_name)?;

        let args_vec = self.gc_interface.alloc(VecValue(args));
        let frame_ptr = Frame::alloc_initial_method(method, &[Value::SYSTEM, Value::Array(args_vec)], self.gc_interface);
        let interpreter = Interpreter::new(frame_ptr);

        Some(interpreter)
    }
}

fn set_super_class(class: &mut Gc<Class>, super_class: &Gc<Class>, metaclass_class: &Gc<Class>) {
    class.set_super_class(super_class);
    class.class().set_super_class(&super_class.class());
    class.class().set_class(metaclass_class);
}
