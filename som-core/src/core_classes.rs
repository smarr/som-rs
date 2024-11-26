use som_gc::gcref::Gc;

/// The core classes of the SOM interpreter.
///
/// This struct allows to always keep a reference to important classes,
/// even in case of modifications to global bindings by user-defined code.
#[derive(Debug)]
pub struct CoreClasses<Class> {
    /// The **Object** class.
    pub object_class: Gc<Class>,
    /// The **Class** class.
    pub class_class: Gc<Class>,
    /// The **Class** class.
    pub metaclass_class: Gc<Class>,

    /// The **Nil** class.
    pub nil_class: Gc<Class>,
    /// The **Integer** class.
    pub integer_class: Gc<Class>,
    /// The **Double** class.
    pub double_class: Gc<Class>,
    /// The **Array** class.
    pub array_class: Gc<Class>,
    /// The **Method** class.
    pub method_class: Gc<Class>,
    /// The **Primitive** class.
    pub primitive_class: Gc<Class>,
    /// The **Symbol** class.
    pub symbol_class: Gc<Class>,
    /// The **String** class.
    pub string_class: Gc<Class>,
    /// The **System** class.
    pub system_class: Gc<Class>,

    /// The **Block** class.
    pub block_class: Gc<Class>,
    /// The **Block1** class.
    pub block1_class: Gc<Class>,
    /// The **Block2** class.
    pub block2_class: Gc<Class>,
    /// The **Block3** class.
    pub block3_class: Gc<Class>,

    /// The **Boolean** class.
    pub boolean_class: Gc<Class>,
    /// The **True** class.
    pub true_class: Gc<Class>,
    /// The **False** class.
    pub false_class: Gc<Class>,
}

impl<Class> CoreClasses<Class> {
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

pub struct CoreClassesIter<'a, Class> {
    fields: std::vec::IntoIter<&'a Gc<Class>>,
}

impl<'a, Class> Iterator for CoreClassesIter<'a, Class> {
    type Item = &'a Gc<Class>;

    fn next(&mut self) -> Option<Self::Item> {
        self.fields.next()
    }
}
