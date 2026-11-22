#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i128),
    Float(f64),
    Bool(bool),
    Str(String),
    Array(Vec<Value>),
    Table(Vec<(String, Value)>),
    DateTime(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    KeyVal(Vec<String>, Value),
    Table(Vec<String>),
    ArrayTable(Vec<String>),
    Empty,
}
