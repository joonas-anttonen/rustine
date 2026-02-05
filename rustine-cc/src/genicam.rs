#![allow(dead_code)]

use std::collections::HashMap;

use rustine::log;

mod expression;
pub use expression::evaluate;

#[derive(Debug)]
pub(crate) struct GenIInfo {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug)]
pub(crate) struct GenIIntReg {
    pub info: Option<GenIInfo>,
    pub address: u32,
    pub length: u32,
    pub signed: bool,
    pub big_endian: bool,
}

#[derive(Debug)]
pub(crate) struct GenIBoolean {
    pub info: Option<GenIInfo>,
    pub value: Option<Box<GenIType>>,
    pub true_value: Option<Box<GenIType>>,
    pub false_value: Option<Box<GenIType>>,
}

#[derive(Debug)]
pub(crate) struct GenIInteger {
    pub info: Option<GenIInfo>,
    pub value: Box<GenIType>,
    pub min: Option<Box<GenIType>>,
    pub max: Option<Box<GenIType>>,
    pub increment: Option<Box<GenIType>>,
    pub unit: Option<String>,
}

#[derive(Debug)]
pub(crate) struct GenIFloat {
    pub info: Option<GenIInfo>,
    pub value: Box<GenIType>,
    pub min: Option<Box<GenIType>>,
    pub max: Option<Box<GenIType>>,
    pub increment: Option<Box<GenIType>>,
    pub unit: Option<String>,
}

#[derive(Debug)]
pub(crate) struct GenIConverter {
    pub info: Option<GenIInfo>,
    pub value: Box<GenIType>,
    pub expression_to: String,
    pub expression_from: String,
    pub variables: HashMap<String, Box<GenIType>>,
}

#[derive(Debug)]
pub(crate) struct GenICommand {
    pub info: Option<GenIInfo>,
    pub value: Box<GenIType>,
    pub cmd_value: u32,
}

#[derive(Debug)]
pub(crate) struct GenIEnumeration {
    pub info: Option<GenIInfo>,
    pub value: Box<GenIType>,
    pub names_to_values: HashMap<String, u32>,
    pub values_to_names: HashMap<u32, String>,
}

impl GenIEnumeration {
    pub fn name_to_value(&self, name: &str) -> Option<u32> {
        self.names_to_values.get(name).copied()
    }

    pub fn value_to_name(&self, value: u32) -> Option<&str> {
        self.values_to_names.get(&value).map(|s| s.as_str())
    }
}

#[derive(Debug)]
pub(crate) enum GenIType {
    Command(GenICommand),
    Boolean(GenIBoolean),
    Integer(GenIInteger),
    ConstantInteger(u32),
    Float(GenIFloat),
    ConstantFloat(f64),
    String,
    ConstantString,
    Enumeration(GenIEnumeration),
    IntReg(GenIIntReg),
    MaskedIntReg,
    FloatReg,
    StringReg,
    StructReg,
    Converter(GenIConverter),
    IntConverter,
    SwissKnife,
    IntSwissKnife,
}

impl GenIType {
    /// Returns the unit of measurement associated with this type, if any.
    pub fn get_unit(&self) -> Option<&str> {
        match self {
            Self::Integer(i) => i.unit.as_deref(),
            Self::Float(f) => f.unit.as_deref(),
            _ => None,
        }
    }
}

pub(crate) struct GenICam {
    features: Vec<GenIType>,
    features_map: HashMap<String, usize>,
}

impl GenICam {
    pub(crate) fn new(features: Vec<GenIType>, features_map: HashMap<String, usize>) -> Self {
        Self {
            features,
            features_map,
        }
    }

    pub fn dump(&self) {
        for feature in &self.features {
            log::info!("Feature: {:#?}", feature);
        }
    }

    /// Attempts to get a command by its name.
    ///
    /// [`None`] if the command is not found or if the feature is not a command.
    pub fn get_command_by_name(&self, name: &str) -> Option<&GenICommand> {
        match self
            .features_map
            .get(name)
            .and_then(|&index| self.features.get(index))
        {
            Some(gtype) => match gtype {
                GenIType::Command(cmd) => Some(cmd),
                _ => {
                    log::warning!("Feature is not a command: {}", name);
                    None
                }
            },
            None => {
                log::warning!("Command not found: {}", name);
                None
            }
        }
    }

    /// Attempts to get an enumeration by its name.
    ///
    /// [`None`] if the enumeration is not found or if the feature is not an enumeration.
    pub fn get_enumeration_by_name(&self, name: &str) -> Option<&GenIEnumeration> {
        match self
            .features_map
            .get(name)
            .and_then(|&index| self.features.get(index))
        {
            Some(gtype) => match gtype {
                GenIType::Enumeration(enumeration) => Some(enumeration),
                _ => {
                    log::warning!("Feature is not an enumeration: {}", name);
                    None
                }
            },
            None => {
                log::warning!("Enumeration not found: {}", name);
                None
            }
        }
    }

    /// Attempts to get a feature by its name.
    ///
    /// [`None`] if the feature is not found.
    pub fn get_feature_by_name(&self, name: &str) -> Option<&GenIType> {
        self.features_map
            .get(name)
            .and_then(|&index| self.features.get(index))
    }
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
fn extract_info(node: &roxmltree::Node) -> Option<GenIInfo> {
    let name = if let Some(name) = node.attribute("Name") {
        name.to_string()
    } else {
        return None;
    };

    let description = node
        .children()
        .find(|n| n.has_tag_name("Description"))
        .and_then(|n| n.text());
    Some(GenIInfo {
        name,
        description: description.map(|d| d.to_string()),
    })
}

fn get_required_value(
    node: &roxmltree::Node,
    name_to_node: &HashMap<String, roxmltree::Node>,
) -> std::io::Result<GenIType> {
    if let Some(n) = node.text().and_then(|t| name_to_node.get(t)) {
        node_to_type(n, name_to_node)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Missing required value for {}", node.tag_name().name()),
        ))
    }
}

fn node_to_type(
    node: &roxmltree::Node,
    name_to_node: &HashMap<String, roxmltree::Node>,
) -> std::io::Result<GenIType> {
    match node.tag_name().name() {
        "Command" => {
            let mut cmd_value = None;
            let mut value = None;

            for child in node.children() {
                match child.tag_name().name() {
                    "CommandValue" => cmd_value = Some(node_text_to_u32(&child, 10)?),
                    "pValue" => value = Some(get_required_value(&child, name_to_node)?),
                    _ => {}
                }
            }

            match (value, cmd_value) {
                (Some(value), Some(cmd_value)) => Ok(GenIType::Command(GenICommand {
                    info: extract_info(node),
                    value: Box::new(value),
                    cmd_value,
                })),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{} missing required fields", node.tag_name().name()),
                )),
            }
        }
        "Integer" => {
            let mut value = None;
            let mut unit = None;
            for child in node.children() {
                match child.tag_name().name() {
                    "Unit" => unit = child.text().map(|s| s.to_string()),
                    "Value" => value = Some(get_required_value(&child, name_to_node)?),
                    "pValue" => value = Some(get_required_value(&child, name_to_node)?),
                    _ => {}
                }
            }

            if value.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} missing required fields",
                        node.attribute("Name").unwrap_or("")
                    ),
                ));
            }

            Ok(GenIType::Integer(GenIInteger {
                info: extract_info(node),
                value: Box::new(value.unwrap()),
                min: None,
                max: None,
                increment: None,
                unit,
            }))
        }
        "Float" => {
            let mut value = None;
            let mut unit = None;
            for child in node.children() {
                match child.tag_name().name() {
                    "Unit" => unit = child.text().map(|s| s.to_string()),
                    "pValue" => value = Some(get_required_value(&child, name_to_node)?),
                    _ => {}
                }
            }

            if value.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} missing required fields",
                        node.attribute("Name").unwrap_or("")
                    ),
                ));
            }

            Ok(GenIType::Float(GenIFloat {
                info: extract_info(node),
                value: Box::new(value.unwrap()),
                min: None,
                max: None,
                increment: None,
                unit,
            }))
        }
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

            if address == 0 || length == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} missing required fields",
                        node.attribute("Name").unwrap_or("")
                    ),
                ));
            }

            Ok(GenIType::IntReg(GenIIntReg {
                info: extract_info(node),
                address,
                length,
                signed,
                big_endian,
            }))
        }
        "Converter" => {
            let mut formula_to = None;
            let mut formula_from = None;
            let mut value = None;
            let mut variables = HashMap::new();

            for child in node.children() {
                match child.tag_name().name() {
                    "FormulaTo" => formula_to = child.text().map(|s| s.to_string()),
                    "FormulaFrom" => formula_from = child.text().map(|s| s.to_string()),
                    "pValue" => value = Some(get_required_value(&child, name_to_node)?),
                    "pVariable" => {
                        variables.insert(
                            child.text().unwrap().to_string(),
                            Box::new(get_required_value(&child, name_to_node)?),
                        );
                    }
                    _ => {}
                }
            }

            if value.is_none() || formula_to.is_none() || formula_from.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{} missing required fields", node.tag_name().name()),
                ));
            }

            Ok(GenIType::Converter(GenIConverter {
                info: extract_info(node),
                expression_to: formula_to.unwrap(),
                expression_from: formula_from.unwrap(),
                value: Box::new(value.unwrap()),
                variables,
            }))
        }
        "Enumeration" => {
            let mut value = None;
            let mut names_to_values = HashMap::new();
            let mut values_to_names = HashMap::new();

            for child in node.children() {
                match child.tag_name().name() {
                    "Value" => value = Some(get_required_value(&child, name_to_node)?),
                    "pValue" => {
                        value = match get_required_value(&child, name_to_node) {
                            Ok(v) => Some(v),
                            Err(_) => {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!(
                                        "{} missing required fields",
                                        node.attribute("Name").unwrap_or("")
                                    ),
                                ));
                            }
                        };
                    }
                    "EnumEntry" => {
                        let name = child.attribute("Name");
                        let value_child = child.children().find(|c| c.tag_name().name() == "Value");
                        if value_child.is_none() || name.is_none() {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "EnumEntry missing required fields",
                            ));
                        }
                        let value = node_text_to_u32(&value_child.unwrap(), 10)?;
                        names_to_values.insert(name.unwrap().to_string(), value);
                        values_to_names.insert(value, name.unwrap().to_string());
                    }
                    _ => {}
                }
            }

            if value.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} missing required fields",
                        node.attribute("Name").unwrap_or("")
                    ),
                ));
            }

            Ok(GenIType::Enumeration(GenIEnumeration {
                info: extract_info(node),
                value: Box::new(value.unwrap()),
                names_to_values,
                values_to_names,
            }))
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!("Unsupported node type {}", node.tag_name().name()),
        )),
    }
}

/// GenICam XML -> `GenICam`.
///
/// # Errors
///
/// This function will only return an error if there's a major issue parsing the XML itself.
/// Unsupported node types or other invalid data will not cause an error.
///
pub(crate) fn parse(xml_content: &str) -> std::io::Result<GenICam> {
    let doc = match roxmltree::Document::parse(xml_content) {
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

                // Special case: StructReg
                // Comment attribute is the name of the struct,
                // individual fields have Name attributes
            }
        }
    }

    let mut gen_features = Vec::<GenIType>::new();
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
            "Integer" => {
                let integer = match node_to_type(&node, &name_to_node) {
                    Ok(feature) => feature,
                    Err(e) => {
                        log::warning!(
                            "Failed to parse Integer {} {}",
                            node.attribute("Name").unwrap_or(""),
                            e
                        );
                        continue;
                    }
                };
                gen_features.push(integer);
                gen_features_map.insert(info_name, gen_features.len() - 1);
            }
            "Float" => {
                let float = match node_to_type(&node, &name_to_node) {
                    Ok(feature) => feature,
                    Err(e) => {
                        log::warning!(
                            "Failed to parse Float {} {}",
                            node.attribute("Name").unwrap_or(""),
                            e
                        );
                        continue;
                    }
                };
                gen_features.push(float);
                gen_features_map.insert(info_name, gen_features.len() - 1);
            }
            "Enumeration" => {
                let enumeration = match node_to_type(&node, &name_to_node) {
                    Ok(enumeration) => enumeration,
                    Err(e) => {
                        log::warning!(
                            "Failed to parse Enumeration {} {}",
                            node.attribute("Name").unwrap_or(""),
                            e
                        );
                        continue;
                    }
                };
                gen_features.push(enumeration);
                gen_features_map.insert(info_name, gen_features.len() - 1);
            }
            "Command" => {
                let cmd = match node_to_type(&node, &name_to_node) {
                    Ok(cmd) => cmd,
                    Err(e) => {
                        log::warning!(
                            "Failed to parse Command {} {}",
                            node.attribute("Name").unwrap_or(""),
                            e
                        );
                        continue;
                    }
                };
                gen_features.push(cmd);
                gen_features_map.insert(info_name, gen_features.len() - 1);
            }
            _ => {}
        };
    }

    Ok(GenICam::new(gen_features, gen_features_map))
}
