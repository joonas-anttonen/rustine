#![allow(dead_code)]

use std::collections::HashMap;

use rustine::log;

#[derive(Debug)]
pub(crate) struct GenInfo {
    pub name: String,
    pub description: Option<String>,
    pub unit: Option<String>,
}

#[derive(Debug)]
pub(crate) struct GenIntReg {
    pub info: Option<GenInfo>,
    pub address: u32,
    pub length: u32,
    pub signed: bool,
    pub big_endian: bool,
}

#[derive(Debug)]
pub(crate) struct GenBoolean {
    pub info: Option<GenInfo>,
    pub value: Option<Box<GenType>>,
    pub true_value: Option<Box<GenType>>,
    pub false_value: Option<Box<GenType>>,
}

#[derive(Debug)]
pub(crate) struct GenCommand {
    pub info: Option<GenInfo>,
    pub value: Box<GenType>,
    pub cmd_value: Box<GenType>,
}

#[derive(Debug)]
pub(crate) enum GenType {
    Command(GenCommand),
    Boolean(GenBoolean),
    Integer,
    ConstantInteger(u32),
    Float,
    ConstantFloat,
    String,
    ConstantString,
    Enumeration,
    IntReg(GenIntReg),
    MaskedIntReg,
    FloatReg,
    StringReg,
    StructReg,
    Converter,
    IntConverter,
    SwissKnife,
    IntSwissKnife,
}

fn node_text_to_u32(node: &roxmltree::Node, radix: u32) -> std::io::Result<u32> {
    match node.text() {
        Some(text) => {
            let text = text.trim_start_matches("0x");
            u32::from_str_radix(text, radix)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }
        None => Err(std::io::Error::from(std::io::ErrorKind::InvalidData)),
    }
}

/// Attempts to extract 'GenInfo' from a given XML node.
fn extract_info(node: &roxmltree::Node) -> Option<GenInfo> {
    let name = if let Some(name) = node.attribute("Name") {
        name.to_string()
    } else {
        return None;
    };

    let description = node
        .children()
        .find(|n| n.has_tag_name("Description"))
        .and_then(|n| n.text());
    let unit = node
        .children()
        .find(|n| n.has_tag_name("Unit"))
        .and_then(|n| n.text());
    Some(GenInfo {
        name,
        description: description.map(|d| d.to_string()),
        unit: unit.map(|u| u.to_string()),
    })
}

fn node_to_type(node: &roxmltree::Node) -> std::io::Result<Box<GenType>> {
    match node.tag_name().name() {
        "IntReg" => {
            let mut address: u32 = 0;
            let mut length: u32 = 0;
            let mut signed: bool = false;
            let mut big_endian: bool = true;

            for child in node.children() {
                match child.tag_name().name() {
                    "Address" => address = node_text_to_u32(&child, 16)?,
                    "Length" => length = node_text_to_u32(&child, 10)?,
                    "Sign" => signed = child.text().unwrap_or("Unsigned") == "Signed",
                    "Endianess" => big_endian = child.text().unwrap_or("BigEndian") == "BigEndian",
                    _ => {}
                }
            }
            Ok(Box::new(GenType::IntReg(GenIntReg {
                info: extract_info(node),
                address,
                length,
                signed,
                big_endian,
            })))
        }
        _ => Err(std::io::Error::from(std::io::ErrorKind::Unsupported)),
    }
}

pub(crate) fn fun_name(xml_content: &str) -> std::io::Result<Vec<GenType>> {
    let doc = match roxmltree::Document::parse(&xml_content) {
        Ok(doc) => doc,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    };

    let maybe_register_description = doc.root().first_child();
    if maybe_register_description.is_none() {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    let register_description = maybe_register_description.unwrap();

    // Pre-pass: Map names to nodes, we will need efficient lookups later.
    // XML structure is a flat list of nodes that reference each other by name in no particular order.
    let mut name_to_node = HashMap::<String, roxmltree::Node>::new();
    {
        for node in register_description.children() {
            if node.is_element() {
                let name = if let Some(name) = node.attribute("Name") {
                    name.to_string()
                } else {
                    continue;
                };
                name_to_node.insert(name, node);
            }
        }
    }

    let mut gen_features = Vec::<GenType>::new();
    let mut gen_features_map = HashMap::<String, usize>::new();
    for node in register_description.children() {
        if !node.is_element() {
            continue;
        }

        let info = extract_info(&node);
        if info.is_none() {
            continue;
        }
        let info = info.unwrap();
        let info_name = info.name.clone();

        match node.tag_name().name() {
            "Command" => {
                let mut cmd_value = None;
                let mut value = None;

                for cmd_element in node.children() {
                    match cmd_element.tag_name().name() {
                        "CommandValue" => match node_text_to_u32(&cmd_element, 10) {
                            Ok(val) => cmd_value = Some(val),
                            Err(e) => {
                                log::warning!("Failed to parse CommandValue: {}", e);
                                continue;
                            }
                        },
                        "pValue" => {
                            if let Some(n) = cmd_element.text().and_then(|t| name_to_node.get(t)) {
                                match node_to_type(&n) {
                                    Ok(t) => value = Some(t),
                                    Err(e) => {
                                        log::warning!("Failed to parse pValue: {}", e);
                                        continue;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                let cmd = match (value, cmd_value) {
                    (Some(value), Some(cmd_value)) => GenType::Command(GenCommand {
                        info: Some(info),
                        value: value,
                        cmd_value: Box::new(GenType::ConstantInteger(cmd_value)),
                    }),
                    _ => {
                        log::warning!("Command {} is incomplete", info_name);
                        continue;
                    }
                };
                gen_features.push(cmd);
                gen_features_map.insert(info_name, gen_features.len() - 1);
            }
            _ => {}
        };
    }

    Ok(gen_features)
}
