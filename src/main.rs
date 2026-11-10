use std::any::Any;
use std::time::Instant;
use crate::value::Value;

mod scanner;
mod toml_parser;
mod value;
mod writer;


fn parse_toml(toml_doc: &str) -> Result<Vec<Value>, String>{
    let mut parser = toml_parser::TomlParser::new(toml_doc);
    parser.parse()
}


fn toml_to_ron_external_crate(toml_doc: &str) -> Result<String, Box<dyn std::error::Error>> {
    let value: toml::Value = toml::from_str(toml_doc)?;
    let output = ron::to_string(&value)?;
    Ok(output)
}


fn measure_time<F: Fn(&str)>(value: &str, f: F) -> f64 {
    let start = Instant::now();
    f(value);
    start.elapsed().as_secs_f64()
}

fn main() {
    // Осн. и доп. 1
    let input = std::fs::read_to_string("data/in.toml").unwrap();
    std::fs::write("data/out.txt", format!("{:#?}", parse_toml(&input).unwrap())).unwrap();
    // Доп. 2
    std::fs::write("data/out2.ron", toml_to_ron_external_crate(&input).unwrap()).unwrap();
    // Доп. 3

    // Доп. 4
    // TODO: добавить к my_time конвертацию
    let my_time: f64 = (0..100).map(|_| measure_time(&input, |s| { let _ = parse_toml(s); })).sum();
    let ext_time: f64 = (0..100).map(|_| measure_time(&input, |s| { let _ = toml_to_ron_external_crate(s); })).sum();
    println!("{} vs. {}", my_time, ext_time);
}
