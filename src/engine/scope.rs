use std::collections::HashMap;

use crate::ast::Ident;

use super::types::Value;

#[derive(Debug, Default)]
pub struct Scope {
    vars: HashMap<Ident, Value>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_var(&self, ident: &Ident) -> bool {
        self.vars.contains_key(ident)
    }

    pub fn get_var(&self, ident: &Ident) -> Option<&Value> {
        self.vars.get(ident)
    }

    pub fn init_var(&mut self, ident: Ident, value: Value) {
        self.vars.insert(ident, value);
    }

    pub fn set_var(&mut self, ident: &Ident, new_value: Value) -> Option<Value> {
        match self.vars.get_mut(ident) {
            None => Some(new_value),
            Some(value) => {
                *value = new_value;
                None
            }
        }
    }
}
