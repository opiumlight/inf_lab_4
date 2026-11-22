use crate::value::{Statement, Value};

pub type Table = Vec<(String, Value)>;

pub fn table_at<'a>(mut table: &'a mut Table, path: &[String]) -> Result<&'a mut Table, String> {
    for key in path {
        let i = match table.iter().position(|(k, _)| k == key) {
            Some(i) => i,
            None => {
                table.push((key.clone(), Value::Table(Vec::new())));
                table.len() - 1
            }
        };
        table = match &mut table[i].1 {
            Value::Table(inner) => inner,
            Value::Array(items) => match items.last_mut() {
                Some(Value::Table(inner)) => inner,
                _ => return Err(format!("«{key}» не таблица")),
            },
            _ => return Err(format!("«{key}» не таблица")),
        };
    }
    Ok(table)
}

pub fn insert(table: &mut Table, path: &[String], val: Value) -> Result<(), String> {
    let (last, parents) = path.split_last().ok_or("пустой ключ")?;
    let target = table_at(table, parents)?;
    if target.iter().any(|(k, _)| k == last) {
        return Err(format!("ключ «{last}» определён дважды"));
    }
    target.push((last.clone(), val));
    Ok(())
}

pub fn build(statements: Vec<Statement>) -> Result<Value, String> {
    let mut root: Table = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut defined: Vec<Vec<String>> = Vec::new();
    for st in statements {
        match st {
            Statement::Empty => {}
            Statement::Table(path) => {
                if defined.contains(&path) {
                    return Err(format!("Table [{}] defined twice", path.join(".")));
                }
                defined.push(path.clone());
                table_at(&mut root, &path)?;
                current = path;
            }
            Statement::ArrayTable(path) => {
                defined.retain(|p| !p.starts_with(&path));
                let (last, parents) = path.split_last().ok_or("empty key")?;
                let parent = table_at(&mut root, parents)?;
                let i = match parent.iter().position(|(k, _)| k == last) {
                    Some(i) => i,
                    None => {
                        parent.push((last.clone(), Value::Array(Vec::new())));
                        parent.len() - 1
                    }
                };
                match &mut parent[i].1 {
                    Value::Array(items) => items.push(Value::Table(Vec::new())),
                    _ => return Err(format!("«{last}» isn't array table")),
                }
                current = path;
            }
            Statement::KeyVal(key, val) => {
                let target = table_at(&mut root, &current)?;
                insert(target, &key, val)?;
            }
        }
    }
    Ok(Value::Table(root))
}
