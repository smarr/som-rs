/// The core classes of the SOM interpreter.
///
/// This struct allows to always keep a reference to important classes,
/// even in case of modifications to global bindings by user-defined code.
#[derive(Debug)]
pub struct CoreClasses<ClassPtr: Copy> {
    /// The **Object** class.
    pub object_class: ClassPtr,
    /// The **Class** class.
    pub class_class: ClassPtr,
    /// The **Class** class.
    pub metaclass_class: ClassPtr,

    /// The **Nil** class.
    pub nil_class: ClassPtr,
    /// The **Integer** class.
    pub integer_class: ClassPtr,
    /// The **Double** class.
    pub double_class: ClassPtr,
    /// The **Array** class.
    pub array_class: ClassPtr,
    /// The **Method** class.
    pub method_class: ClassPtr,
    /// The **Primitive** class.
    pub primitive_class: ClassPtr,
    /// The **Symbol** class.
    pub symbol_class: ClassPtr,
    /// The **String** class.
    pub string_class: ClassPtr,
    /// The **System** class.
    pub system_class: ClassPtr,

    /// The **Block** class.
    pub block_class: ClassPtr,
    /// The **Block1** class.
    pub block1_class: ClassPtr,
    /// The **Block2** class.
    pub block2_class: ClassPtr,
    /// The **Block3** class.
    pub block3_class: ClassPtr,

    /// The **Boolean** class.
    pub boolean_class: ClassPtr,
    /// The **True** class.
    pub true_class: ClassPtr,
    /// The **False** class.
    pub false_class: ClassPtr,
}

impl<ClassPtr: Copy> CoreClasses<ClassPtr> {
    /// Get the **Object** class.
    pub fn object_class(&self) -> ClassPtr {
        self.object_class
    }

    /// Get the **Nil** class.
    pub fn nil_class(&self) -> ClassPtr {
        self.nil_class
    }
    /// Get the **System** class.
    pub fn system_class(&self) -> ClassPtr {
        self.system_class
    }

    /// Get the **Symbol** class.
    pub fn symbol_class(&self) -> ClassPtr {
        self.symbol_class
    }
    /// Get the **String** class.
    pub fn string_class(&self) -> ClassPtr {
        self.string_class
    }
    /// Get the **Array** class.
    pub fn array_class(&self) -> ClassPtr {
        self.array_class
    }

    /// Get the **Integer** class.
    pub fn integer_class(&self) -> ClassPtr {
        self.integer_class
    }
    /// Get the **Double** class.
    pub fn double_class(&self) -> ClassPtr {
        self.double_class
    }

    /// Get the **Block** class.
    pub fn block_class(&self) -> ClassPtr {
        self.block_class
    }
    /// Get the **Block1** class.
    pub fn block1_class(&self) -> ClassPtr {
        self.block1_class
    }
    /// Get the **Block2** class.
    pub fn block2_class(&self) -> ClassPtr {
        self.block2_class
    }
    /// Get the **Block3** class.
    pub fn block3_class(&self) -> ClassPtr {
        self.block3_class
    }

    /// Get the **True** class.
    pub fn true_class(&self) -> ClassPtr {
        self.true_class
    }
    /// Get the **False** class.
    pub fn false_class(&self) -> ClassPtr {
        self.false_class
    }

    /// Get the **Metaclass** class.
    pub fn metaclass_class(&self) -> ClassPtr {
        self.metaclass_class
    }

    /// Get the **Method** class.
    pub fn method_class(&self) -> ClassPtr {
        self.method_class
    }
    /// Get the **Primitive** class.
    pub fn primitive_class(&self) -> ClassPtr {
        self.primitive_class
    }
}

impl<Class: Copy> CoreClasses<Class> {
    pub fn iter(&self) -> CoreClassesIter<Class> {
        CoreClassesIter {
            fields: vec![
                &self.object_class,
                &self.class_class,
                &self.metaclass_class,
                &self.nil_class,
                &self.integer_class,
                &self.double_class,
                &self.array_class,
                &self.method_class,
                &self.primitive_class,
                &self.symbol_class,
                &self.string_class,
                &self.system_class,
                &self.block_class,
                &self.block1_class,
                &self.block2_class,
                &self.block3_class,
                &self.boolean_class,
                &self.true_class,
                &self.false_class,
            ]
            .into_iter(),
        }
    }
}

pub struct CoreClassesIter<'a, ClassPtr> {
    fields: std::vec::IntoIter<&'a ClassPtr>,
}

impl<'a, ClassPtr> Iterator for CoreClassesIter<'a, ClassPtr> {
    type Item = &'a ClassPtr;

    fn next(&mut self) -> Option<Self::Item> {
        self.fields.next()
    }
}
