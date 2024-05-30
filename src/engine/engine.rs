use std::ops::Deref;

use dashu::{base::RemEuclid, float::DBig};
use derive_more::Display;

use crate::parser::ast::{Expr, Func, Node, Statement};

use super::{error::RunError, native::native_print, scope::FuncType, Scope, Value};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnaryOpType {
    #[display(fmt = "-")]
    Neg,
    #[display(fmt = "!")]
    Not,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryOpType {
    #[display(fmt = "+")]
    Add,
    #[display(fmt = "-")]
    Sub,
    #[display(fmt = "*")]
    Mul,
    #[display(fmt = "/")]
    Div,
    #[display(fmt = "%")]
    Mod,
    #[display(fmt = "**")]
    Pow,

    #[display(fmt = "==")]
    Eq,
    #[display(fmt = "<")]
    Lt,
    #[display(fmt = ">")]
    Gt,
    #[display(fmt = "!=")]
    NEq,
    #[display(fmt = "<=")]
    LtEq,
    #[display(fmt = ">=")]
    GtEq,
    #[display(fmt = "and")]
    And,
    #[display(fmt = "or")]
    Or,
}

#[derive(Debug)]
pub struct Engine<Data> {
    global_scope: Scope<Data>,
    nested_scopes: Vec<Scope<Data>>,
}

impl<Data> Default for Engine<Data> {
    fn default() -> Self {
        let mut global_scope = Scope::new();
        global_scope.init_native_func(native_print());

        Self {
            global_scope,
            nested_scopes: Vec::new(),
        }
    }
}

impl<Data: Clone> Engine<Data> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scope(&self) -> &Scope<Data> {
        &self.global_scope
    }

    pub fn push_scope(&mut self) {
        self.nested_scopes.push(Scope::new());
    }

    pub fn pop_scope(&mut self) -> bool {
        self.nested_scopes.pop().is_some()
    }

    pub fn set_var(&mut self, ident: impl Into<String>, value: Value) {
        match self.nested_scopes.last_mut() {
            None => self.global_scope.init_var(ident, value),
            Some(scope) => scope.init_var(ident, value),
        }
    }

    pub fn set_func(&mut self, func: Func<Data>) {
        match self.nested_scopes.last_mut() {
            None => self.global_scope.init_func(func),
            Some(scope) => scope.init_func(func),
        }
    }

    pub fn get_var(&self, ident: impl AsRef<str>) -> Option<&Value> {
        for scope in self.nested_scopes.iter().rev() {
            if let Some(value) = scope.get_var(ident.as_ref()) {
                return Some(value);
            }
        }

        self.global_scope.get_var(ident)
    }

    pub fn get_var_mut(&mut self, ident: impl AsRef<str>) -> Option<&mut Value> {
        for scope in self.nested_scopes.iter_mut().rev() {
            if let Some(value) = scope.get_var_mut(ident.as_ref()) {
                return Some(value);
            }
        }

        self.global_scope.get_var_mut(ident)
    }

    pub fn get_func(&self, ident: &Node<Data, String>) -> Result<&FuncType<Data>, RunError<Data>> {
        // try all nested scopes first
        for scope in self.nested_scopes.iter().rev() {
            if let Some(func) = scope.get_func(ident.deref()) {
                return Ok(func);
            }
        }

        // then pull from global scope
        self.global_scope
            .get_func(ident.deref())
            .ok_or_else(|| RunError::UnknownFunction {
                ident: ident.deref().into(),
                data: ident.data().clone(),
            })
    }

    pub fn eval_func(
        &mut self,
        ident: &Node<Data, String>,
        values: Vec<Value>,
    ) -> Result<Value, RunError<Data>> {
        // get and validate function
        let func = self.get_func(ident)?;
        if func.param_count() < values.len() {
            return Err(RunError::ParameterCount {
                expected: func.param_count(),
                found: values.len(),
                data: ident.data().clone(),
            });
        }

        let mut output = Value::None;
        let func = func.clone();
        self.push_scope(); // create scope for function
        match func.clone() {
            FuncType::Native(func) => {
                match (func.native)(values) {
                    Ok(value) => output = value,
                    Err(message) => {
                        self.pop_scope(); // ensure scope is popped before error
                        return Err(RunError::NativeCallError {
                            data: ident.data().clone(),
                            message,
                        });
                    }
                }
            }
            FuncType::Custom(func) => {
                for (param, value) in func.params.iter().zip(values) {
                    // init all variables with their values
                    self.set_var(param.deref(), value);
                }

                for statement in func.body.clone() {
                    match self.eval_statement(&statement) {
                        Ok(value) => output = value,
                        Err(e) => {
                            self.pop_scope(); // ensure scope is popped before error
                            return Err(e);
                        }
                    }
                }
            }
        }
        self.pop_scope(); //pop scope when finished

        Ok(output)
    }

    pub fn eval_statement(
        &mut self,
        statement: &Node<Data, Statement<Data>>,
    ) -> Result<Value, RunError<Data>> {
        match statement.deref() {
            Statement::Expr(expr) => self.eval(expr),
            Statement::Func(func) => {
                self.set_func(func.deref().clone());
                Ok(Value::None)
            }
            Statement::LetAssign(ident, expr) => {
                let value = self.eval(expr)?;
                self.set_var(ident.deref(), value);
                Ok(Value::None)
            }
            Statement::Assign(ident, expr) => {
                let value = self.eval(expr)?;
                self.set_var(ident.deref(), value);
                Ok(Value::None)
            }
            Statement::While(w) => loop {
                match self.eval(&w.cond)? {
                    Value::Bool(true) => (),
                    Value::Bool(false) => return Ok(Value::None),
                    value => {
                        return Err(RunError::TypeMismatch {
                            expected: format!("bool"),
                            found: format!("{}", value.type_name()),
                            data: w.cond.data().clone(),
                        })
                    }
                }

                for statement in w.body.iter() {
                    self.eval_statement(&statement)?;
                }
            },
        }
    }

    pub fn eval(&mut self, expr: &Node<Data, Expr<Data>>) -> Result<Value, RunError<Data>> {
        match expr.deref() {
            Expr::None => Ok(Value::None),
            Expr::Bool(v) => Ok(Value::Bool(*v)),
            Expr::Int(v) => Ok(Value::Int(v.clone())),
            Expr::Float(v) => Ok(Value::Float(v.clone())),
            Expr::String(v) => Ok(Value::String(v.clone())),
            Expr::Call(ident, params) => {
                let mut values = Vec::new();
                for expr in params {
                    values.push(self.eval(expr)?);
                }
                self.eval_func(ident, values)
            }
            Expr::Neg(inner) => {
                let inner = self.eval(inner)?;
                self.eval_unary(UnaryOpType::Neg, inner, expr.data())
            }
            Expr::Not(inner) => {
                let inner = self.eval(inner)?;
                self.eval_unary(UnaryOpType::Not, inner, expr.data())
            }
            Expr::Add(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::Add, rhs, expr.data())
            }
            Expr::Sub(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::Sub, rhs, expr.data())
            }
            Expr::Mul(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::Mul, rhs, expr.data())
            }
            Expr::Div(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::Div, rhs, expr.data())
            }
            Expr::Pow(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::Pow, rhs, expr.data())
            }
            Expr::Mod(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::Mod, rhs, expr.data())
            }
            Expr::Eq(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::Eq, rhs, expr.data())
            }
            Expr::Lt(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::Lt, rhs, expr.data())
            }
            Expr::Gt(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::Gt, rhs, expr.data())
            }
            Expr::NEq(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::NEq, rhs, expr.data())
            }
            Expr::LtEq(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::LtEq, rhs, expr.data())
            }
            Expr::GtEq(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::GtEq, rhs, expr.data())
            }
            Expr::And(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::And, rhs, expr.data())
            }
            Expr::Or(lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                self.eval_binary(lhs, BinaryOpType::Or, rhs, expr.data())
            }
            Expr::Var(ident) => match self.get_var(ident.deref()) {
                Some(value) => Ok(value.clone()),
                None => Err(RunError::UnknownVariable {
                    ident: ident.clone(),
                    data: expr.data().clone(),
                }),
            },
            Expr::Walrus(ident, assign_expr) => {
                let value = self.eval(assign_expr)?;
                self.set_var(ident.deref(), value.clone());
                Ok(value)
            }
            Expr::Ternary(lhs, cond, rhs) => {
                // evaluate condition
                let cond = match self.eval(&cond)? {
                    Value::Bool(cond) => cond,
                    value => {
                        return Err(RunError::TypeMismatch {
                            expected: "'bool'".into(),
                            found: format!("'{}'", value.type_name()),
                            data: cond.data().clone(),
                        })
                    }
                };

                // then evaluate the correct expression
                match cond {
                    true => self.eval(&lhs),
                    false => self.eval(&rhs),
                }
            }
        }
    }

    fn eval_unary(
        &self,
        op: UnaryOpType,
        val: Value,
        data: &Data,
    ) -> Result<Value, RunError<Data>> {
        let vtype = val.type_name();
        match (val, op) {
            (Value::Bool(v), UnaryOpType::Not) => Ok(Value::Bool(!v)),
            (Value::Int(v), UnaryOpType::Neg) => Ok(Value::Int(-v)),
            (Value::Float(v), UnaryOpType::Neg) => Ok(Value::Float(-v)),
            _ => Err(RunError::InvalidUnary {
                op,
                vtype,
                data: data.clone(),
            }),
        }
    }

    fn eval_binary(
        &self,
        val1: Value,
        op: BinaryOpType,
        val2: Value,
        data: &Data,
    ) -> Result<Value, RunError<Data>> {
        let vtype1 = val1.type_name();
        let vtype2 = val2.type_name();

        match (val1, op, val2) {
            // ---------------
            // --- INT OPS ---
            // int add
            (Value::Int(v1), BinaryOpType::Add, Value::Bool(v2)) => Ok(Value::Int(v1 + v2 as i64)),
            (Value::Int(v1), BinaryOpType::Add, Value::Int(v2)) => Ok(Value::Int(v1 + v2)),
            (Value::Int(v1), BinaryOpType::Add, Value::Float(v2)) => Ok(Value::Float(v1 + v2)),
            // int sub
            (Value::Int(v1), BinaryOpType::Sub, Value::Bool(v2)) => Ok(Value::Int(v1 - v2 as i64)),
            (Value::Int(v1), BinaryOpType::Sub, Value::Int(v2)) => Ok(Value::Int(v1 - v2)),
            (Value::Int(v1), BinaryOpType::Sub, Value::Float(v2)) => Ok(Value::Float(v1 - v2)),
            // int mul
            (Value::Int(v1), BinaryOpType::Mul, Value::Bool(v2)) => Ok(Value::Int(v1 * v2 as i64)),
            (Value::Int(v1), BinaryOpType::Mul, Value::Int(v2)) => Ok(Value::Int(v1 * v2)),
            (Value::Int(v1), BinaryOpType::Mul, Value::Float(v2)) => Ok(Value::Float(v1 * v2)),
            // int div
            (Value::Int(v1), BinaryOpType::Div, Value::Int(v2)) => {
                Ok(Value::Float(DBig::from(v1) / v2))
            }
            (Value::Int(v1), BinaryOpType::Div, Value::Float(v2)) => Ok(Value::Float(v1 / v2)),
            // int mod
            (Value::Int(v1), BinaryOpType::Mod, Value::Int(v2)) => {
                Ok(Value::Int(v1.rem_euclid(v2).into()))
            }
            (Value::Int(v1), BinaryOpType::Mod, Value::Float(v2)) => {
                Ok(Value::Float((DBig::from(v1)).rem_euclid(v2)))
            }
            // int pow
            (Value::Int(v1), BinaryOpType::Pow, Value::Int(v2)) => {
                Ok(Value::Float((DBig::from(v1)).powf(&DBig::from(v2))))
            }
            (Value::Int(v1), BinaryOpType::Pow, Value::Float(v2)) => {
                Ok(Value::Float((DBig::from(v1)).powf(&v2)))
            }

            // int equality
            (Value::Int(v1), BinaryOpType::Eq, Value::Int(v2)) => Ok(Value::Bool(v1 == v2)),
            (Value::Int(v1), BinaryOpType::Eq, Value::Float(v2)) => {
                Ok(Value::Bool(DBig::from(v1) == v2))
            }
            // int less than
            (Value::Int(v1), BinaryOpType::Lt, Value::Int(v2)) => Ok(Value::Bool(v1 < v2)),
            (Value::Int(v1), BinaryOpType::Lt, Value::Float(v2)) => {
                Ok(Value::Bool(DBig::from(v1) < v2))
            }
            // int greater than
            (Value::Int(v1), BinaryOpType::Gt, Value::Int(v2)) => Ok(Value::Bool(v1 > v2)),
            (Value::Int(v1), BinaryOpType::Gt, Value::Float(v2)) => {
                Ok(Value::Bool(DBig::from(v1) > v2))
            }
            // int not equal
            (Value::Int(v1), BinaryOpType::NEq, Value::Int(v2)) => Ok(Value::Bool(v1 != v2)),
            (Value::Int(v1), BinaryOpType::NEq, Value::Float(v2)) => {
                Ok(Value::Bool(DBig::from(v1) != v2))
            }
            // int less than equal
            (Value::Int(v1), BinaryOpType::LtEq, Value::Int(v2)) => Ok(Value::Bool(v1 <= v2)),
            (Value::Int(v1), BinaryOpType::LtEq, Value::Float(v2)) => {
                Ok(Value::Bool(DBig::from(v1) <= v2))
            }
            // int greater than equal
            (Value::Int(v1), BinaryOpType::GtEq, Value::Int(v2)) => Ok(Value::Bool(v1 >= v2)),
            (Value::Int(v1), BinaryOpType::GtEq, Value::Float(v2)) => {
                Ok(Value::Bool(DBig::from(v1) >= v2))
            }

            // -----------------
            // --- FLOAT OPS ---
            // float add
            (Value::Float(v1), BinaryOpType::Add, Value::Bool(v2)) => {
                Ok(Value::Float(v1 + v2 as u8))
            }
            (Value::Float(v1), BinaryOpType::Add, Value::Int(v2)) => Ok(Value::Float(v1 + v2)),
            (Value::Float(v1), BinaryOpType::Add, Value::Float(v2)) => Ok(Value::Float(v1 + v2)),
            // float sub
            (Value::Float(v1), BinaryOpType::Sub, Value::Bool(v2)) => {
                Ok(Value::Float(v1 - v2 as u8))
            }
            (Value::Float(v1), BinaryOpType::Sub, Value::Int(v2)) => Ok(Value::Float(v1 - v2)),
            (Value::Float(v1), BinaryOpType::Sub, Value::Float(v2)) => Ok(Value::Float(v1 - v2)),
            // float mul
            (Value::Float(v1), BinaryOpType::Mul, Value::Bool(v2)) => {
                Ok(Value::Float(v1 * v2 as u8))
            }
            (Value::Float(v1), BinaryOpType::Mul, Value::Int(v2)) => Ok(Value::Float(v1 * v2)),
            (Value::Float(v1), BinaryOpType::Mul, Value::Float(v2)) => Ok(Value::Float(v1 * v2)),
            // float div
            (Value::Float(v1), BinaryOpType::Div, Value::Int(v2)) => Ok(Value::Float(v1 / v2)),
            (Value::Float(v1), BinaryOpType::Div, Value::Float(v2)) => Ok(Value::Float(v1 / v2)),
            // float mod
            (Value::Float(v1), BinaryOpType::Mod, Value::Int(v2)) => {
                Ok(Value::Float(v1.rem_euclid(DBig::from(v2))))
            }
            (Value::Float(v1), BinaryOpType::Mod, Value::Float(v2)) => {
                Ok(Value::Float(v1.rem_euclid(v2)))
            }
            // float pow
            (Value::Float(v1), BinaryOpType::Pow, Value::Int(v2)) => {
                Ok(Value::Float(v1.powf(&DBig::from(v2))))
            }
            (Value::Float(v1), BinaryOpType::Pow, Value::Float(v2)) => {
                Ok(Value::Float(v1.powf(&v2)))
            }
            // float equality
            (Value::Float(v1), BinaryOpType::Eq, Value::Int(v2)) => {
                Ok(Value::Bool(v1 == DBig::from(v2)))
            }
            (Value::Float(v1), BinaryOpType::Eq, Value::Float(v2)) => Ok(Value::Bool(v1 == v2)),
            // float less than
            (Value::Float(v1), BinaryOpType::Lt, Value::Int(v2)) => {
                Ok(Value::Bool(v1 < DBig::from(v2)))
            }
            (Value::Float(v1), BinaryOpType::Lt, Value::Float(v2)) => Ok(Value::Bool(v1 < v2)),
            // float greater than
            (Value::Float(v1), BinaryOpType::Gt, Value::Int(v2)) => {
                Ok(Value::Bool(v1 > DBig::from(v2)))
            }
            (Value::Float(v1), BinaryOpType::Gt, Value::Float(v2)) => Ok(Value::Bool(v1 > v2)),
            // float not equal
            (Value::Float(v1), BinaryOpType::NEq, Value::Int(v2)) => {
                Ok(Value::Bool(v1 != DBig::from(v2)))
            }
            (Value::Float(v1), BinaryOpType::NEq, Value::Float(v2)) => Ok(Value::Bool(v1 != v2)),
            // float less than equal
            (Value::Float(v1), BinaryOpType::LtEq, Value::Int(v2)) => {
                Ok(Value::Bool(v1 <= DBig::from(v2)))
            }
            (Value::Float(v1), BinaryOpType::LtEq, Value::Float(v2)) => Ok(Value::Bool(v1 <= v2)),
            // float greater than equal
            (Value::Float(v1), BinaryOpType::GtEq, Value::Int(v2)) => {
                Ok(Value::Bool(v1 >= DBig::from(v2)))
            }
            (Value::Float(v1), BinaryOpType::GtEq, Value::Float(v2)) => Ok(Value::Bool(v1 >= v2)),

            // ------------------
            // --- STRING OPS ---
            // string add
            (Value::String(v1), BinaryOpType::Add, Value::Bool(v2)) => {
                Ok(Value::String(format!("{v1}{v2}")))
            }
            (Value::String(v1), BinaryOpType::Add, Value::Int(v2)) => {
                Ok(Value::String(format!("{v1}{v2}")))
            }
            (Value::String(v1), BinaryOpType::Add, Value::Float(v2)) => {
                Ok(Value::String(format!("{v1}{v2}")))
            }
            (Value::String(v1), BinaryOpType::Add, Value::String(v2)) => {
                Ok(Value::String(format!("{v1}{v2}")))
            }
            // string mul
            (Value::String(v1), BinaryOpType::Mul, Value::Bool(v2)) => {
                Ok(Value::String(v1.repeat(v2 as usize)))
            }
            (Value::String(v1), BinaryOpType::Mul, Value::Int(v2)) => {
                let (sign, ubig) = v2.into_parts();
                match sign {
                    dashu::base::Sign::Negative => return Ok(Value::String("".into())),
                    _ => (),
                }

                match TryInto::<usize>::try_into(ubig) {
                    Ok(count) => Ok(Value::String(v1.repeat(count))),
                    Err(_) => Ok(Value::String(v1.repeat(usize::MAX))),
                }
            }
            // string equality
            (Value::String(v1), BinaryOpType::Eq, Value::String(v2)) => Ok(Value::Bool(v1 == v2)),
            // string less than
            (Value::String(v1), BinaryOpType::Lt, Value::String(v2)) => Ok(Value::Bool(v1 < v2)),
            // string greater than
            (Value::String(v1), BinaryOpType::Gt, Value::String(v2)) => Ok(Value::Bool(v1 > v2)),
            // string not equal
            (Value::String(v1), BinaryOpType::NEq, Value::String(v2)) => Ok(Value::Bool(v1 != v2)),
            // string less than equal
            (Value::String(v1), BinaryOpType::LtEq, Value::String(v2)) => Ok(Value::Bool(v1 <= v2)),
            // string greater than equal
            (Value::String(v1), BinaryOpType::GtEq, Value::String(v2)) => Ok(Value::Bool(v1 >= v2)),

            // -------------------
            // --- BOOLEAN OPS ---
            // boolean equality
            (Value::Bool(v1), BinaryOpType::Eq, Value::Bool(v2)) => Ok(Value::Bool(v1 == v2)),
            // boolean less than
            (Value::Bool(v1), BinaryOpType::Lt, Value::Bool(v2)) => Ok(Value::Bool(v1 < v2)),
            // boolean greater than
            (Value::Bool(v1), BinaryOpType::Gt, Value::Bool(v2)) => Ok(Value::Bool(v1 > v2)),
            // boolean not equal
            (Value::Bool(v1), BinaryOpType::NEq, Value::Bool(v2)) => Ok(Value::Bool(v1 != v2)),
            // boolean less than equal
            (Value::Bool(v1), BinaryOpType::LtEq, Value::Bool(v2)) => Ok(Value::Bool(v1 <= v2)),
            // boolean greater than equal
            (Value::Bool(v1), BinaryOpType::GtEq, Value::Bool(v2)) => Ok(Value::Bool(v1 >= v2)),
            // boolean and
            (Value::Bool(v1), BinaryOpType::And, Value::Bool(v2)) => Ok(Value::Bool(v1 && v2)),
            // boolean or
            (Value::Bool(v1), BinaryOpType::Or, Value::Bool(v2)) => Ok(Value::Bool(v1 || v2)),

            // --------------------
            // --- FAILURE CASE ---
            _ => Err(RunError::InvalidBinary {
                op,
                vtype1,
                vtype2,
                data: data.clone(),
            }),
        }
    }
}
