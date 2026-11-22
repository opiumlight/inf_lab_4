use crate::value::Value;
use crate::writer::ron::to_ron;
use crate::writer::xml::to_xml;
use std::time::Instant;

mod scanner;
mod toml_parser;
mod tree;
mod value;
mod writer;

fn parse_toml(toml_doc: &str) -> Result<Value, String> {
    let mut parser = toml_parser::TomlParser::new(toml_doc);
    tree::build(parser.parse()?)
}

fn toml_to_ron_external_crate(value: &toml::Value) -> Result<String, Box<dyn std::error::Error>> {
    let config = ron::ser::PrettyConfig::new().depth_limit(4).indentor("\t");
    Ok(ron::ser::to_string_pretty(value, config)?)
}

fn toml_to_xml_external_crate(value: &toml::Value) -> Result<String, Box<dyn std::error::Error>> {
    Ok(quick_xml::se::to_string_with_root("root", value)?)
}

fn measure_time<F: Fn(&str)>(value: &str, f: F) -> f64 {
    let start = Instant::now();
    f(value);
    start.elapsed().as_secs_f64()
}

fn main() {
    // Осн. и доп. 1
    let input = std::fs::read_to_string("data/in.toml").unwrap();
    let parsed_toml = parse_toml(&input).unwrap();
    std::fs::write("data/IR.txt", format!("{:#?}", parsed_toml)).unwrap();
    std::fs::write("data/my_out1.ron", to_ron(&parsed_toml)).unwrap();
    // Доп. 2
    let ext_input: toml::Value = toml::from_str(&input).unwrap();
    std::fs::write(
        "data/ext_out2.ron",
        toml_to_ron_external_crate(&ext_input).unwrap(),
    )
    .unwrap();
    // Доп. 3
    std::fs::write("data/my_out3.xml", to_xml(&parsed_toml).unwrap()).unwrap();
    std::fs::write(
        "data/ext_out4.xml",
        toml_to_xml_external_crate(&ext_input).unwrap(),
    )
    .unwrap();
    // Доп. 4
    let my_parse_time: f64 = (0..100)
        .map(|_| {
            measure_time(&input, |s| {
                parse_toml(s).unwrap();
            })
        })
        .sum();
    let ext_parse_time: f64 = (0..100)
        .map(|_| {
            measure_time(&input, |s| {
                let _: toml::Value = toml::from_str(s).unwrap();
            })
        })
        .sum();
    let my_to_ron_time: f64 = (0..100)
        .map(|_| {
            measure_time(&input, |_| {
                to_ron(&parsed_toml);
            })
        })
        .sum();
    let my_to_xml_time: f64 = (0..100)
        .map(|_| {
            measure_time(&input, |_| {
                to_xml(&parsed_toml).unwrap();
            })
        })
        .sum();
    let ext_to_ron_time: f64 = (0..100)
        .map(|_| {
            measure_time(&input, |_| {
                toml_to_ron_external_crate(&ext_input).unwrap();
            })
        })
        .sum();
    let ext_to_xml_time: f64 = (0..100)
        .map(|_| {
            measure_time(&input, |_| {
                toml_to_xml_external_crate(&ext_input).unwrap();
            })
        })
        .sum();
    println!("-----Сравнение скорости-----");
    println!("|{:.9} vs {:.9}| Парсинг", my_parse_time, ext_parse_time);
    println!("|{:.9} vs {:.9}| to_ron ", my_to_ron_time, ext_to_ron_time);
    println!("|{:.9} vs {:.9}| to_xml ", my_to_xml_time, ext_to_xml_time);
    println!(
        "|{:.9} vs {:.9}| Сумма  ",
        my_to_xml_time + my_parse_time + my_to_ron_time,
        ext_to_xml_time + ext_parse_time + ext_to_ron_time
    );
    println!("|{:11} vs {:11}|", "Собственный", "Библиотека");
    println!("-----Сравнение скорости-----");
}
