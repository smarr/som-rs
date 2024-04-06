//!
//! This crate serves as the syntactical analyser (parser) for the Simple Object Machine.
//!
//! This particular version of the parser works with the tokens outputted by the lexical analyser, instead of directly reading text.
//!

/// SOM-specific parser combinators.
pub mod lang;

use std::cell::RefCell;
use std::rc::Rc;
use som_core::ast::{ClassDef, Expression};
use som_lexer::Token;
use som_parser_core::{Parser};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum AstGenCtxtType {
    Class,
    Block,
    Method,
}

#[derive(Clone, Debug)]
pub struct AstGenCtxtData {
    kind: AstGenCtxtType, // used for debugging
    name: String, // debugging too
    local_names: Vec<String>,
    param_names: Vec<String>,
    class_instance_fields: Vec<String>,
    class_static_fields: Vec<String>, // it's possible the distinction between static/instance fields is useless, but i don't think so.
    current_scope: usize,
    outer_ctxt: Option<AstGenCtxt>,
}

pub type AstGenCtxt = Rc<RefCell<AstGenCtxtData>>;

enum FoundVar {
    Local(usize, usize),
    Argument(usize, usize),
    Field(usize),
}

impl Default for AstGenCtxtData {
    fn default() -> Self {
        AstGenCtxtData {
            kind: AstGenCtxtType::Class,
            name: "NO NAME".to_string(),
            local_names: vec![],
            param_names: vec![],
            class_static_fields: vec![],
            class_instance_fields: vec![],
            current_scope: 0,
            outer_ctxt: None,
        }
    }
}

impl AstGenCtxtData {
    pub fn new_ctxt_from(outer: AstGenCtxt, kind: AstGenCtxtType) -> AstGenCtxt {
        Rc::new(RefCell::new(
        AstGenCtxtData {
            kind,
            name: "NO NAME".to_string(),
            local_names: vec![],
            param_names: vec![],
            class_instance_fields: vec![],
            class_static_fields: vec![],
            current_scope: outer.borrow().current_scope + 1,
            outer_ctxt: Some(Rc::clone(&outer)),
        }))
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn get_outer(&self) -> AstGenCtxt {
        let outer = self.outer_ctxt.as_ref().unwrap();
        Rc::clone(outer)
    }

    // pub fn add_fields(&mut self, fields_names: &Vec<String>) {
    //     self.class_field_names.extend(fields_names.iter().cloned());
    // }

    pub fn add_instance_fields(&mut self, fields_names: &Vec<String>) {
        self.class_instance_fields.extend(fields_names.iter().cloned());
    }

    pub fn add_static_fields(&mut self, fields_names: &Vec<String>) {
        self.class_static_fields.extend(fields_names.iter().cloned());
    }

    pub fn add_locals(&mut self, new_locals_names: &Vec<String>) {
        self.local_names.extend(new_locals_names.iter().cloned());
    }

    pub fn add_params(&mut self, parameters: &Vec<String>) {
        assert_ne!(self.kind, AstGenCtxtType::Class); // can't add parameters to a class.
        self.param_names.extend(parameters.iter().cloned());
    }

    pub fn get_local(&self, name: &String) -> Option<usize> {
        self.local_names.iter().position(|local| local == name)
    }

    pub fn get_param(&self, name: &String) -> Option<usize> {
        self.param_names.iter().position(|local| *local == *name)
    }

    // pub fn get_field(&self, name: &String) -> Option<usize> {
    //     self.class_field_names.iter().position(|c| c == name)
    // }

    pub fn get_instance_field(&self, name: &String) -> Option<usize> {
        self.class_instance_fields.iter().position(|c| c == name)
    }

    pub fn get_static_field(&self, name: &String) -> Option<usize> {
        self.class_static_fields.iter().position(|c| c == name)
    }

    fn find_var(&self, name: &String) -> Option<FoundVar> {
        self.get_local(name)
            .map(|idx| FoundVar::Local(0, idx))
            .or_else(|| self.get_param(name).map(|idx| FoundVar::Argument(0, idx)))
            .or_else(|| self.get_instance_field(name).map(|idx| FoundVar::Field(idx)))
            .or_else(|| self.get_static_field(name).map(|idx| FoundVar::Field(idx)))
            .or_else(|| {
                match &self.outer_ctxt.as_ref() {
                    None => None,
                    Some(outer) => outer.borrow().find_var(name).map(|found| match found {
                        FoundVar::Local(up_idx, idx) => FoundVar::Local(up_idx + 1, idx),
                        FoundVar::Argument(up_idx, idx) => FoundVar::Argument(up_idx + 1, idx),
                        FoundVar::Field(idx) => FoundVar::Field(idx),
                    })
                }
            })
    }
    fn get_var_read(&self, name: &String) -> Expression {
        if name == "self" {
            return Expression::ArgRead(0, 0);
        }

        match self.find_var(name) {
            None => Expression::GlobalRead(name.clone()),
            Some(v) => {
                match v {
                    FoundVar::Local(up_idx, idx) => {
                        match up_idx {
                            0 => Expression::LocalVarRead(idx),
                            _ => Expression::NonLocalVarRead(up_idx, idx)
                        }
                    },
                    FoundVar::Argument(up_idx, idx) => Expression::ArgRead(up_idx, idx + 1),
                    FoundVar::Field(idx) => Expression::FieldRead(idx)
                }
            }
        }
    }

    fn get_var_write(&self, name: &String, expr: Box<Expression>) -> Expression {
        // if name == "self" {
        //     return Expression::ArgWrite(0, 0, expr); // but really, this isn't a thing... right?
        // }

        match self.find_var(name) {
            None => Expression::GlobalWrite(name.clone(), expr),
            Some(v) => {
                match v {
                    FoundVar::Local(up_idx, idx) => {
                        match up_idx {
                            0 => Expression::LocalVarWrite(idx, expr),
                            _ => Expression::NonLocalVarWrite(up_idx, idx, expr)
                        }
                    },
                    FoundVar::Argument(up_idx, idx) => Expression::ArgWrite(up_idx, idx + 1, expr), // + 1 to adjust for self
                    FoundVar::Field(idx) => Expression::FieldWrite(idx, expr)
                }
            }
        }
    }
}


/// Parses the input of an entire file into an AST.
pub fn parse_file(input: &[Token]) -> Option<ClassDef> {
    self::apply(lang::file(), input)
}

/// Applies a parser and returns the output value if the entirety of the input has been parsed successfully.
pub fn apply<'a, A, P>(mut parser: P, input: &'a [Token]) -> Option<A>
    where
        P: Parser<A, &'a [Token], AstGenCtxt>,
{
    match parser.parse(input, AstGenCtxt::default()) {
        Some((output, tail, _)) if tail.is_empty() => Some(output),
        Some(_) | None => None,
    }
}
