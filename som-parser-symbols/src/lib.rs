//!
//! This crate serves as the syntactical analyser (parser) for the Simple Object Machine.
//!
//! This particular version of the parser works with the tokens outputted by the lexical analyser, instead of directly reading text.
//!

/// SOM-specific parser combinators.
pub mod lang;

use som_core::ast::ClassDef;
use som_lexer::Token;
use som_parser_core::{Parser};

#[derive(Clone, Debug)]
pub struct AstMethodGenCtxt {
    // pub all_locals: Vec<String>,
    pub all_locals: Vec<(String, usize)>, // each var has an assigned scope.
    pub current_scope: usize
}

impl Default for AstMethodGenCtxt {
    fn default() -> Self {
        AstMethodGenCtxt{
            all_locals: vec![],
            current_scope: 0
        }
    }
}

impl AstMethodGenCtxt {
    pub fn add_new_local_vars(&self, new_locals_names: Vec<String>) -> AstMethodGenCtxt {
        AstMethodGenCtxt {
            all_locals: self.all_locals.iter().cloned()
                .chain(new_locals_names.iter().map(|v| (v.clone(), self.current_scope)))
                .collect(),
            current_scope: self.current_scope
        }
    }

    pub fn increase_scope(&self) -> AstMethodGenCtxt {
        AstMethodGenCtxt{ all_locals: self.all_locals.clone(), current_scope: self.current_scope + 1 }
    }

    pub fn decrease_scope(&self) -> AstMethodGenCtxt {
        AstMethodGenCtxt{ all_locals: self.all_locals.clone(), current_scope: self.current_scope - 1 }
    }
}


/// Parses the input of an entire file into an AST.
pub fn parse_file(input: &[Token]) -> Option<ClassDef> {
    self::apply(lang::file(), input)
}

/// Applies a parser and returns the output value if the entirety of the input has been parsed successfully.
pub fn apply<'a, A, P>(mut parser: P, input: &'a [Token]) -> Option<A>
    where
        P: Parser<A, &'a [Token], AstMethodGenCtxt>,
{
    match parser.parse(input, AstMethodGenCtxt::default()) {
        Some((output, tail, _)) if tail.is_empty() => Some(output),
        Some(_) | None => None,
    }
}
