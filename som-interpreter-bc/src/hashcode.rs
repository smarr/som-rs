use std::hash::{Hash, Hasher};

use crate::block::Block;
use crate::class::Class;
use crate::instance::Instance;
use crate::method::Method;
use crate::value::Value;

impl Hash for Value {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        match self {
            Value::Nil => {
                hasher.write(b"#nil#");
            }
            Value::System => {
                hasher.write(b"#system#");
            }
            Value::Boolean(value) => {
                hasher.write(b"#bool#");
                value.hash(hasher);
            }
            Value::Integer(value) => {
                hasher.write(b"#int#");
                value.hash(hasher);
            }
            Value::BigInteger(value) => {
                hasher.write(b"#bigint#");
                value.hash(hasher);
            }
            Value::Double(value) => {
                hasher.write(b"#double#");
                let raw_bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(
                        (value as *const f64) as *const u8,
                        std::mem::size_of::<f64>(),
                    )
                };
                hasher.write(raw_bytes);
            }
            Value::Symbol(value) => {
                hasher.write(b"#sym#");
                value.hash(hasher);
            }
            Value::String(value) => {
                hasher.write(b"#string#");
                value.to_obj().hash(hasher);
            }
            Value::Array(value) => {
                hasher.write(b"#arr#");
                for value in value.borrow().iter() {
                    value.hash(hasher);
                }
            }
            Value::Block(value) => {
                hasher.write(b"#blk#");
                value.to_obj().hash(hasher);
            }
            Value::Class(value) => {
                hasher.write(b"#cls#");
                value.to_obj().hash(hasher);
            }
            Value::Instance(value) => {
                hasher.write(b"#inst#");
                value.to_obj().hash(hasher);
            }
            Value::Invokable(value) => {
                hasher.write(b"#mthd#");
                value.to_obj().hash(hasher);
            }
        }
    }
}

impl Hash for Class {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.name.hash(hasher);
        self.locals.iter().for_each(|value| {
            value.hash(hasher);
        });
    }
}

impl Hash for Instance {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.class.to_obj().hash(hasher);
        self.nbr_fields.hash(hasher);
        // todo better hash that actually reads the values
        // self.locals.iter().for_each(|value| {
        //     value.hash(hasher);
        // });
    }
}

impl Hash for Block {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        let blk_info = self.blk_info.to_obj();
        blk_info.literals.iter().for_each(|it| it.hash(hasher));
        blk_info.nb_locals.hash(hasher);
        // self.blk_info.locals.iter().for_each(|it| it.hash(hasher));
        blk_info.nb_params.hash(hasher);
        blk_info.body.iter().for_each(|it| it.hash(hasher));
    }
}

impl Hash for Method {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.holder.to_obj().hash(hasher);
        hasher.write(b">>");
        self.signature.hash(hasher);
    }
}
