#![allow(dead_code)]

use std::{cell::Cell, collections::HashMap, rc::Rc};

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
    pub address: Box<GenIType>,
    pub length: Box<GenIType>,
    pub signed: bool,
    pub big_endian: bool,
}

#[derive(Debug)]
pub(crate) struct GenIFloatReg {
    pub info: Option<GenIInfo>,
    pub address: Box<GenIType>,
    pub length: Box<GenIType>,
    pub big_endian: bool,
}

#[derive(Debug)]
pub(crate) struct GenIStructReg {
    pub info: Option<GenIInfo>,
    pub address: u32,
    pub length: u32,
    pub big_endian: bool,
    /// Either a single bit or a range of bits to read.
    pub value_range: (u32, u32),
}

#[derive(Debug)]
pub(crate) struct GenIBoolean {
    pub info: Option<GenIInfo>,
    pub value: Box<GenIType>,
    pub true_value: Box<GenIType>,
    pub false_value: Box<GenIType>,
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

/// Custom type to represent Integer with pIndex and pValueIndexed.
#[derive(Debug)]
pub(crate) struct GenIIndexedInteger {
    pub index: Box<GenIType>,
    pub values: HashMap<u32, Box<GenIType>>,
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
pub(crate) struct GenISwissKnife {
    pub info: Option<GenIInfo>,
    pub formula: String,
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

pub enum GenIUnit {
    None,
    Second,
    Millisecond,
    Microsecond,
    Nanosecond,
    Percent,
    Decibel,
    Unknown(String),
}

#[derive(Debug)]
pub(crate) enum GenIType {
    Command(GenICommand),
    Boolean(GenIBoolean),
    Integer(GenIInteger),
    Float(GenIFloat),
    String,
    ConstantString,
    Enumeration(GenIEnumeration),
    IntReg(GenIIntReg),
    MaskedIntReg,
    FloatReg(GenIFloatReg),
    StringReg,
    StructReg(GenIStructReg),
    Converter(GenIConverter),
    IntConverter,
    SwissKnife(GenISwissKnife),

    /// Custom type to represent Integer with pIndex and pValueIndexed.
    IndexedInteger(GenIIndexedInteger),
    /// Custom type to represent a variable value.
    /// Enables persistent reading and writing from plain numerical values.
    Variable(Rc<Cell<f64>>),
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
    features_map: HashMap<String, GenIType>,
}

impl GenICam {
    pub(crate) fn new(features_map: HashMap<String, GenIType>) -> Self {
        Self { features_map }
    }

    /// Attempts to get a command by its name.
    ///
    /// [`None`] if the command is not found or if the feature is not a command.
    pub fn get_command_by_name(&self, name: &str) -> Option<&GenICommand> {
        match self.features_map.get(name) {
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
        match self.features_map.get(name) {
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

    /// Attempts to get a boolean by its name.
    ///
    /// [`None`] if the boolean is not found or if the feature is not a boolean.
    pub fn get_boolean_by_name(&self, name: &str) -> Option<&GenIBoolean> {
        match self.features_map.get(name) {
            Some(gtype) => match gtype {
                GenIType::Boolean(boolean) => Some(boolean),
                _ => {
                    log::warning!("Feature is not a boolean: {}", name);
                    None
                }
            },
            None => {
                log::warning!("Boolean not found: {}", name);
                None
            }
        }
    }

    /// Attempts to get a feature by its name.
    ///
    /// [`None`] if the feature is not found.
    pub fn get_feature_by_name(&self, name: &str) -> Option<&GenIType> {
        self.features_map.get(name)
    }
}

fn node_text_to_integer(node: &roxmltree::Node) -> std::io::Result<u32> {
    match node.text() {
        Some(text) => parse_integral_value(text),
        None => Err(std::io::Error::from(std::io::ErrorKind::InvalidData)),
    }
}

fn parse_integral_value(text: &str) -> Result<u32, std::io::Error> {
    let radix = if text.starts_with("0x") { 16 } else { 10 };
    let text = text.trim_start_matches("0x");
    u32::from_str_radix(text, radix)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn node_text_to_f64(node: &roxmltree::Node) -> std::io::Result<f64> {
    match node.text() {
        Some(text) => {
            let text = text.trim();
            text.parse::<f64>()
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

fn get_struct_entry(entry_name: &str, node: &roxmltree::Node) -> std::io::Result<GenIType> {
    let entry = node
        .children()
        .find(|c| c.attribute("Name").is_some_and(|n| n == entry_name));
    if entry.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("StructEntry {} not found", entry_name),
        ));
    }
    let entry = entry.unwrap();

    /*
    <StructReg Comment="N273">
        <pAddress>N277</pAddress>
        <Length>4</Length>
        <Endianess>BigEndian</Endianess>
        <StructEntry Name="N274">
            <Bit>0</Bit>
        </StructEntry>
        <StructEntry Name="N275">
            <Bit>1</Bit>
        </StructEntry>
        <StructEntry Name="N276">
            <LSB>31</LSB>
            <MSB>16</MSB>
        </StructEntry>
    </StructReg>
     */

    let mut value_range = None;

    for entry_child in entry.children() {
        match entry_child.tag_name().name() {
            "Bit" => {
                let bit = node_text_to_integer(&entry_child)?;
                value_range = Some((bit, bit));
            }
            "LSB" => {
                let lsb = node_text_to_integer(&entry_child)?;
                if let Some((_, some_msb)) = value_range {
                    value_range = Some((lsb, some_msb));
                } else {
                    value_range = Some((lsb, lsb));
                }
            }
            "MSB" => {
                let msb = node_text_to_integer(&entry_child)?;
                if let Some((some_lsb, _)) = value_range {
                    value_range = Some((some_lsb, msb));
                } else {
                    value_range = Some((msb, msb));
                }
            }
            _ => {}
        }
    }

    let mut address: u32 = 0;
    let mut length: u32 = 0;
    let mut big_endian: bool = true;

    for child in node.children() {
        match child.tag_name().name() {
            "Address" => address = node_text_to_integer(&child)?,
            "Length" => length = node_text_to_integer(&child)?,
            "Endianess" => big_endian = child.text().unwrap_or("BigEndian") == "BigEndian",
            _ => {}
        }
    }

    if value_range.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{} missing required fields", entry_name),
        ));
    }

    Ok(GenIType::StructReg(GenIStructReg {
        info: extract_info(node),
        address,
        length,
        big_endian,
        value_range: value_range.unwrap(),
    }))
}

fn get_required_value(
    node: &roxmltree::Node,
    name_to_node: &HashMap<String, roxmltree::Node>,
    variable_storage: &mut HashMap<String, Rc<Cell<f64>>>,
) -> std::io::Result<GenIType> {
    if let Some(n) = node.text().and_then(|t| name_to_node.get(t)) {
        // Handle cursed special case of StructReg
        if n.tag_name().name() == "StructReg" {
            get_struct_entry(node.text().unwrap_or(""), n)
        } else {
            node_to_type(n, name_to_node, variable_storage)
        }
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Missing required value for {} {}",
                node.tag_name().name(),
                node.attribute("Name").unwrap_or("")
            ),
        ))
    }
}

fn node_to_type(
    node: &roxmltree::Node,
    name_to_node: &HashMap<String, roxmltree::Node>,
    variable_storage: &mut HashMap<String, Rc<Cell<f64>>>,
) -> std::io::Result<GenIType> {
    match node.tag_name().name() {
        "Command" => {
            let mut cmd_value = None;
            let mut value = None;

            for child in node.children() {
                match child.tag_name().name() {
                    "CommandValue" => cmd_value = Some(node_text_to_integer(&child)?),
                    "pValue" => {
                        value = Some(get_required_value(&child, name_to_node, variable_storage)?)
                    }
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
                    format!("{} no pValue or CommandValue", node.tag_name().name()),
                )),
            }
        }
        "Boolean" => {
            let mut value = None;
            let mut true_value = None;
            let mut false_value = None;

            for child in node.children() {
                match child.tag_name().name() {
                    "OnValue" => {
                        true_value =
                            Some(get_required_value(&child, name_to_node, variable_storage)?)
                    }
                    "OffValue" => {
                        false_value =
                            Some(get_required_value(&child, name_to_node, variable_storage)?)
                    }
                    "Value" => {
                        let variable_value = node_text_to_integer(&child)?;
                        value = Some(get_variable(
                            variable_storage,
                            node,
                            child.tag_name().name(),
                            variable_value as f64,
                        )?);
                    }
                    "pValue" => {
                        value = Some(get_required_value(&child, name_to_node, variable_storage)?)
                    }
                    _ => {}
                }
            }

            if value.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} no Value or pValue or pIndex",
                        node.attribute("Name").unwrap_or("")
                    ),
                ));
            }

            Ok(GenIType::Boolean(GenIBoolean {
                info: extract_info(node),
                value: Box::new(value.unwrap()),
                true_value: Box::new(true_value.unwrap_or(get_variable(
                    variable_storage,
                    node,
                    "True",
                    1.0,
                )?)),
                false_value: Box::new(false_value.unwrap_or(get_variable(
                    variable_storage,
                    node,
                    "False",
                    0.0,
                )?)),
            }))
        }
        "Integer" => {
            let mut value = None;
            let mut value_indices: HashMap<u32, GenIType> = HashMap::new();
            let mut unit = None;
            for child in node.children() {
                match child.tag_name().name() {
                    "Unit" => unit = child.text().map(|s| s.to_string()),
                    "Value" => {
                        let variable_value = node_text_to_integer(&child)?;
                        value = Some(get_variable(
                            variable_storage,
                            node,
                            child.tag_name().name(),
                            variable_value as f64,
                        )?);
                    }
                    "pValue" => {
                        value = Some(get_required_value(&child, name_to_node, variable_storage)?)
                    }
                    // <pValueIndexed Index="SOME_INDEX">SOME_OTHER_VALUE</pValueIndexed>
                    "pValueIndexed" => {
                        if let Some(index) = child.attribute("Index") {
                            let index = parse_integral_value(index)?;
                            let value = get_required_value(&child, name_to_node, variable_storage)?;
                            value_indices.insert(index, value);
                        }
                    }
                    // Appears with pValueIndexed
                    "pIndex" => {
                        value = Some(get_required_value(&child, name_to_node, variable_storage)?);
                    }
                    _ => {}
                }
            }

            if value.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} no Value or pValue or pIndex",
                        node.attribute("Name").unwrap_or("")
                    ),
                ));
            }

            if value_indices.len() > 0 {
                value = Some(GenIType::IndexedInteger(GenIIndexedInteger {
                    index: Box::new(value.unwrap()),
                    values: value_indices
                        .into_iter()
                        .map(|(k, v)| (k, Box::new(v)))
                        .collect(),
                }));
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
                    "Value" => {
                        let variable_value = node_text_to_f64(&child)?;
                        value = Some(get_variable(
                            variable_storage,
                            node,
                            child.tag_name().name(),
                            variable_value,
                        )?);
                    }
                    "pValue" => {
                        value = Some(get_required_value(&child, name_to_node, variable_storage)?)
                    }
                    _ => {}
                }
            }

            if value.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} no Value or pValue",
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
            let mut address = None;
            let mut length = None;
            let mut signed: bool = false;
            let mut big_endian: bool = true;

            for child in node.children() {
                match child.tag_name().name() {
                    "pAddress" => {
                        address = Some(get_required_value(&child, name_to_node, variable_storage)?)
                    }
                    "Address" => {
                        let variable_value = node_text_to_integer(&child)?;
                        address = Some(get_variable(
                            variable_storage,
                            node,
                            child.tag_name().name(),
                            variable_value as f64,
                        )?);
                    }
                    "pLength" => {
                        length = Some(get_required_value(&child, name_to_node, variable_storage)?)
                    }
                    "Length" => {
                        let variable_value = node_text_to_integer(&child)?;
                        length = Some(get_variable(
                            variable_storage,
                            node,
                            child.tag_name().name(),
                            variable_value as f64,
                        )?);
                    }
                    "Sign" => signed = child.text().unwrap_or("Unsigned") == "Signed",
                    "Endianess" => big_endian = child.text().unwrap_or("BigEndian") == "BigEndian",
                    _ => {}
                }
            }

            if address.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} no Address or pAddress",
                        node.attribute("Name").unwrap_or("")
                    ),
                ));
            }

            if length.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} no Length or pLength",
                        node.attribute("Name").unwrap_or("")
                    ),
                ));
            }

            Ok(GenIType::IntReg(GenIIntReg {
                info: extract_info(node),
                address: Box::new(address.unwrap()),
                length: Box::new(length.unwrap()),
                signed,
                big_endian,
            }))
        }
        "FloatReg" => {
            let mut address = None;
            let mut length = None;
            let mut big_endian: bool = true;

            for child in node.children() {
                match child.tag_name().name() {
                    "pAddress" => {
                        address = Some(get_required_value(&child, name_to_node, variable_storage)?)
                    }
                    "Address" => {
                        let variable_value = node_text_to_integer(&child)?;
                        address = Some(get_variable(
                            variable_storage,
                            node,
                            child.tag_name().name(),
                            variable_value as f64,
                        )?);
                    }
                    "pLength" => {
                        length = Some(get_required_value(&child, name_to_node, variable_storage)?)
                    }
                    "Length" => {
                        let variable_value = node_text_to_integer(&child)?;
                        length = Some(get_variable(
                            variable_storage,
                            node,
                            child.tag_name().name(),
                            variable_value as f64,
                        )?);
                    }
                    "Endianess" => big_endian = child.text().unwrap_or("BigEndian") == "BigEndian",
                    _ => {}
                }
            }

            if address.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} no Address or pAddress",
                        node.attribute("Name").unwrap_or("")
                    ),
                ));
            }

            if length.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} no Length or pLength",
                        node.attribute("Name").unwrap_or("")
                    ),
                ));
            }

            Ok(GenIType::FloatReg(GenIFloatReg {
                info: extract_info(node),
                address: Box::new(address.unwrap()),
                length: Box::new(length.unwrap()),
                big_endian,
            }))
        }
        "SwissKnife" | "IntSwissKnife" => {
            let mut variables = HashMap::new();
            let mut formula = None;

            for child in node.children() {
                match child.tag_name().name() {
                    "Formula" => formula = child.text().map(|s| s.to_string()),
                    "pVariable" => {
                        variables.insert(
                            child.text().unwrap().to_string(),
                            Box::new(get_required_value(&child, name_to_node, variable_storage)?),
                        );
                    }
                    _ => {}
                }
            }

            if formula.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{} missing required fields", node.tag_name().name()),
                ));
            }

            Ok(GenIType::SwissKnife(GenISwissKnife {
                info: extract_info(node),
                formula: formula.unwrap(),
                variables,
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
                    "pValue" => {
                        value = Some(get_required_value(&child, name_to_node, variable_storage)?)
                    }
                    "pVariable" => {
                        variables.insert(
                            child.text().unwrap().to_string(),
                            Box::new(get_required_value(&child, name_to_node, variable_storage)?),
                        );
                    }
                    _ => {}
                }
            }

            if value.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{} no pValue", node.tag_name().name()),
                ));
            }

            if formula_to.is_none() || formula_from.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{} no FormulaTo or FormulaFrom", node.tag_name().name()),
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
                    "Value" => {
                        let variable_value = node_text_to_integer(&child)?;
                        value = Some(get_variable(
                            variable_storage,
                            node,
                            child.tag_name().name(),
                            variable_value as f64,
                        )?);
                    }
                    "pValue" => {
                        value = Some(get_required_value(&child, name_to_node, variable_storage)?);
                    }
                    "EnumEntry" => {
                        let name = child.attribute("Name");
                        let value_child = child.children().find(|c| c.tag_name().name() == "Value");
                        if value_child.is_none() || name.is_none() {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "EnumEntry no Name or Value",
                            ));
                        }
                        let value = node_text_to_integer(&value_child.unwrap())?;
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
                        "{} no Value or pValue",
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
            format!(
                "Unsupported: {} {}",
                node.tag_name().name(),
                node.attribute("Name").unwrap_or("")
            ),
        )),
    }
}

fn get_variable(
    variable_storage: &mut HashMap<String, Rc<Cell<f64>>>,
    node: &roxmltree::Node,
    element_name: &str,
    initial_value: f64,
) -> std::io::Result<GenIType> {
    let node_name = if let Some(node_name) = node.attribute("Name") {
        node_name.to_string()
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Node has no Name attribute",
        ));
    };

    let variable_name = format!("{}_{}", node_name, element_name);

    let variable = if let Some(existing) = variable_storage.get(&variable_name) {
        existing.clone()
    } else {
        let new_var = Rc::new(Cell::new(0.0));
        variable_storage.insert(variable_name.clone(), new_var.clone());
        new_var
    };

    variable.set(initial_value);
    Ok(GenIType::Variable(variable))
}

/// GenICam XML -> `GenICam`.
///
/// # Errors
///
/// This function will only return an error if there's a major issue parsing the XML itself.
/// Unsupported node types or other invalid data will not cause an error.
///
pub(crate) fn parse(xml_content: &str) -> std::io::Result<GenICam> {
    let gen_doc = parse_xml(xml_content)?;
    let feature_name_to_node = preprocess_xml(&gen_doc)?;

    let mut variable_storage = HashMap::<String, Rc<Cell<f64>>>::new();

    let mut gen_features_map = HashMap::<String, GenIType>::new();
    for (feature_name, node) in &feature_name_to_node {
        if !node.is_element() {
            continue;
        }

        // TODO: Skip Visibility == "Invisible"
        match node.tag_name().name() {
            "Boolean" => {
                match node_to_type(&node, &feature_name_to_node, &mut variable_storage) {
                    Ok(feature) => {
                        gen_features_map.insert(feature_name.clone(), feature);
                    }
                    Err(e) => {
                        log::warning!(
                            "parse::Boolean {} {}",
                            node.attribute("Name").unwrap_or(""),
                            e.to_string()
                        );
                    }
                };
            }
            "Integer" => {
                match node_to_type(&node, &feature_name_to_node, &mut variable_storage) {
                    Ok(feature) => {
                        gen_features_map.insert(feature_name.clone(), feature);
                    }
                    Err(e) => {
                        log::warning!(
                            "parse::Integer {} {}",
                            node.attribute("Name").unwrap_or(""),
                            e.to_string()
                        );
                    }
                };
            }
            "Float" => {
                match node_to_type(&node, &feature_name_to_node, &mut variable_storage) {
                    Ok(feature) => {
                        gen_features_map.insert(feature_name.clone(), feature);
                    }
                    Err(e) => {
                        log::warning!(
                            "parse::Float {} {}",
                            node.attribute("Name").unwrap_or(""),
                            e
                        );
                    }
                };
            }
            "Enumeration" => {
                match node_to_type(&node, &feature_name_to_node, &mut variable_storage) {
                    Ok(enumeration) => {
                        gen_features_map.insert(feature_name.clone(), enumeration);
                    }
                    Err(e) => {
                        log::warning!(
                            "parse::Enumeration {} {}",
                            node.attribute("Name").unwrap_or(""),
                            e
                        );
                    }
                };
            }
            "Command" => {
                match node_to_type(&node, &feature_name_to_node, &mut variable_storage) {
                    Ok(cmd) => {
                        gen_features_map.insert(feature_name.clone(), cmd);
                    }
                    Err(e) => {
                        log::warning!(
                            "parse::Command {} {}",
                            node.attribute("Name").unwrap_or(""),
                            e
                        );
                    }
                };
            }
            _ => {}
        };
    }

    Ok(GenICam::new(gen_features_map))
}

fn preprocess_xml<'a>(
    doc: &'a roxmltree::Document<'a>,
) -> std::io::Result<HashMap<String, roxmltree::Node<'a, 'a>>> {
    let maybe_register_description = doc.root().first_child();
    if maybe_register_description.is_none() {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    let genicam_node = maybe_register_description.unwrap();
    if genicam_node.tag_name().name() != "RegisterDescription" {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    let mut name_to_node = HashMap::<String, roxmltree::Node>::new();
    {
        for node in genicam_node.children() {
            if !node.is_element() {
                continue;
            }

            // Normal case:
            // Nodes with "Name" attribute are at the top level
            if let Some(name) = node.attribute("Name") {
                name_to_node.insert(name.to_string(), node);
            } else {
                // Special case:
                // Nodes are inside <Group>s
                if node.tag_name().name() == "Group" {
                    for child_node in node.children() {
                        if let Some(name) = child_node.attribute("Name") {
                            name_to_node.insert(name.to_string(), child_node);
                        }
                    }
                }
                // Special case:
                // <StructReg Comment="XXX">
                //     <StructEntry Name="YYY" />
                // Map all entry names to the parent StructReg node.
                else if node.tag_name().name() == "StructReg" {
                    for child_node in node.children() {
                        if child_node.tag_name().name() == "StructEntry"
                            && let Some(entry_name) = child_node.attribute("Name")
                        {
                            name_to_node.insert(entry_name.to_string(), node);
                        }
                    }
                }
            };
        }
    }

    Ok(name_to_node)
}

fn parse_xml(xml_content: &str) -> std::io::Result<roxmltree::Document<'_>> {
    roxmltree::Document::parse(xml_content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_integer() {
        let xml = r#"
        <RegisterDescription>
            <Group>
                <Boolean Name="N1">
                    <pValue>N2</pValue>
                </Boolean>
            </Group>
            <Group>
                <Integer Name="N4" NameSpace="Custom">
                    <Value>0</Value>
                </Integer>
                <Integer Name="N3" NameSpace="Custom">
                    <Value>0</Value>
                </Integer>
                <Integer Name="N2" NameSpace="Custom">
                    <pIndex>N3</pIndex>
                    <pValueIndexed Index="1">N4</pValueIndexed>
                    <pValueIndexed Index="4">N4</pValueIndexed>
                    <pValueDefault>N4</pValueDefault>
                </Integer>
            </Group>
        </RegisterDescription>
        "#;

        let gen_doc = parse_xml(xml).unwrap();
        let name_to_node = preprocess_xml(&gen_doc).unwrap();

        // Assert we have "N1" and "N2" and "N3"
        assert!(name_to_node.contains_key("N1"), "Missing key N1");
        assert!(name_to_node.contains_key("N2"), "Missing key N2");
        assert!(name_to_node.contains_key("N3"), "Missing key N3");

        let parse_result = node_to_type(&name_to_node["N1"], &name_to_node, &mut HashMap::new());

        // What we should have now is GenIBoolean with the value
        // pointing to GenIInteger N2 and N2 value having type GenIIndexedInteger
        assert!(parse_result.is_ok());

        let gen_bool = match parse_result.unwrap() {
            GenIType::Boolean(gen_bool) => gen_bool,
            other => panic!("Expected GenIType::Boolean, got {:?}", other),
        };

        let gen_int = match *gen_bool.value {
            GenIType::Integer(gen_int) => gen_int,
            other => panic!("Expected GenIType::Integer, got {:?}", other),
        };

        let indexed = match *gen_int.value {
            GenIType::IndexedInteger(indexed) => indexed,
            other => panic!("Expected GenIType::IndexedInteger, got {:?}", other),
        };

        let index = match *indexed.index {
            GenIType::Integer(index) => index,
            other => panic!("Expected GenIType::Integer index, got {:?}", other),
        };

        let index_value = match *index.value {
            GenIType::Variable(value) => value.get(),
            other => panic!("Expected GenIType::Variable, got {:?}", other),
        };

        assert_eq!(index_value, 0.0);
        assert_eq!(indexed.values.len(), 2);
        assert!(indexed.values.contains_key(&1));
        assert!(indexed.values.contains_key(&4));

        for value in indexed.values.values() {
            match value.as_ref() {
                GenIType::Integer(gen_int) => match gen_int.value.as_ref() {
                    GenIType::Variable(value) => assert_eq!(value.get(), 0.0),
                    other => panic!("Expected Variable, got {:?}", other),
                },
                other => panic!("Expected GenIType::Integer, got {:?}", other),
            }
        }
    }

    #[test]
    fn unsupported_node_type() {
        let xml = r#"
        <RegisterDescription>
            <Group>
                <Integer Name="N1">
                    <pValue>N2</pValue>
                </Integer>
            </Group>
            <Group>
                <WeirdReg Name="N2">
                    <Address>0x0D04</Address>
                    <Length>4</Length>
                    <LSB>31</LSB>
                    <MSB>16</MSB>
                    <Sign>Unsigned</Sign>
                    <Endianess>BigEndian</Endianess>
                </WeirdReg>
            </Group>
        </RegisterDescription>
        "#;

        let gen_doc = parse_xml(xml).unwrap();
        let name_to_node = preprocess_xml(&gen_doc).unwrap();

        // Assert we have "N1" and "N2"
        assert!(name_to_node.contains_key("N1"), "Missing key N1");
        assert!(name_to_node.contains_key("N2"), "Missing key N2");

        let parse_result = node_to_type(&name_to_node["N1"], &name_to_node, &mut HashMap::new());

        assert!(parse_result.is_err());

        // Assert that the error mentions "WeirdReg"
        if let Err(e) = &parse_result {
            assert!(e.to_string().contains("WeirdReg"));
        }
    }

    #[test]
    fn intreg_paddress_into_intswissknife() {
        let xml = r#"
        <RegisterDescription>
            <IntReg Name="N1">
                <pAddress>N2</pAddress>
                <Length>4</Length>
                <Sign>Unsigned</Sign>
                <Endianess>BigEndian</Endianess>
            </IntReg>
            <IntSwissKnife Name="N2">
                <pVariable Name="V1">N3</pVariable>
                <Formula>1000000000 / V1</Formula>
            </IntSwissKnife>
            <IntReg Name="N3">
                <Address>0x0</Address>
                <Length>4</Length>
                <Sign>Unsigned</Sign>
                <Endianess>BigEndian</Endianess>
            </IntReg>
        </RegisterDescription>
        "#;

        let gen_doc = parse_xml(xml).unwrap();
        let name_to_node = preprocess_xml(&gen_doc).unwrap();

        let parse_result = node_to_type(
            &gen_doc
                .root()
                .first_element_child()
                .unwrap()
                .first_element_child()
                .unwrap(),
            &name_to_node,
            &mut HashMap::new(),
        );
        if let Err(e) = &parse_result {
            eprintln!("{:?}", e);
        }
        assert!(parse_result.is_ok());
    }
}
