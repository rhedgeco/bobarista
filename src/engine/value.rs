use dashu::{float::DBig, integer::IBig};
use derive_more::Display;

#[derive(Debug, Display, Clone)]
pub enum Value {
    #[display(fmt = "none")]
    None,
    #[display(fmt = "{}", _0)]
    Bool(bool),
    #[display(fmt = "{}", _0)]
    Int(IBig),
    #[display(fmt = "{}", _0)]
    Float(DBig),
    #[display(fmt = "{}", _0)]
    String(String),
}

impl Value {
    pub fn type_name(&self) -> String {
        match self {
            Value::None => format!("none"),
            Value::Bool(_) => format!("bool"),
            Value::Int(_) => format!("int"),
            Value::Float(_) => format!("float"),
            Value::String(_) => format!("string"),
        }
    }
}
