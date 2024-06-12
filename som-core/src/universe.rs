use std::fmt::{Debug, Formatter};

pub trait Universe {
    // fn load_class(&mut self, class_name: impl Into<String>);
    fn load_class(&mut self, class_name: &str);
}

impl Debug for dyn Universe {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Universe for parser")
    }
}