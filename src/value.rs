#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i128),
    Float(f64),
    Bool(bool),
    Str(String),
    Array(Vec<Value>),
    KeyValue((String, Box<Value>)),
    Table(Vec<(String, Box<Value>)>),
    Empty(())
}