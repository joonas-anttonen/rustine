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
    pub address: Box<GenIType>,
    pub length: Box<GenIType>,
    pub signed: bool,
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
    StructReg(GenIStructReg),
    Converter(GenIConverter),
    IntConverter,
    SwissKnife(GenISwissKnife),
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

    /// Attempts to get a feature by its name.
    ///
    /// [`None`] if the feature is not found.
    pub fn get_feature_by_name(&self, name: &str) -> Option<&GenIType> {
        self.features_map.get(name)
    }
}

fn node_text_to_u32(node: &roxmltree::Node) -> std::io::Result<u32> {
    match node.text() {
        Some(text) => {
            let radix = if text.starts_with("0x") { 16 } else { 10 };
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
                let bit = node_text_to_u32(&entry_child)?;
                value_range = Some((bit, bit));
            }
            "LSB" => {
                let lsb = node_text_to_u32(&entry_child)?;
                if let Some((_, some_msb)) = value_range {
                    value_range = Some((lsb, some_msb));
                } else {
                    value_range = Some((lsb, lsb));
                }
            }
            "MSB" => {
                let msb = node_text_to_u32(&entry_child)?;
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
            "Address" => address = node_text_to_u32(&child)?,
            "Length" => length = node_text_to_u32(&child)?,
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
) -> std::io::Result<GenIType> {
    if let Some(n) = node.text().and_then(|t| name_to_node.get(t)) {
        // Handle cursed special case of StructReg
        if n.tag_name().name() == "StructReg" {
            get_struct_entry(node.text().unwrap_or(""), n)
        } else {
            node_to_type(n, name_to_node)
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
) -> std::io::Result<GenIType> {
    match node.tag_name().name() {
        "Command" => {
            let mut cmd_value = None;
            let mut value = None;

            for child in node.children() {
                match child.tag_name().name() {
                    "CommandValue" => cmd_value = Some(node_text_to_u32(&child)?),
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
                    "Value" => value = Some(GenIType::ConstantInteger(node_text_to_u32(&child)?)),
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
            let mut address = None;
            let mut length = None;
            let mut signed: bool = false;
            let mut big_endian: bool = true;

            for child in node.children() {
                match child.tag_name().name() {
                    "pAddress" => address = Some(get_required_value(&child, name_to_node)?),
                    "Address" => {
                        address = Some(GenIType::ConstantInteger(node_text_to_u32(&child)?))
                    }
                    "pLength" => length = Some(get_required_value(&child, name_to_node)?),
                    "Length" => length =  Some(GenIType::ConstantInteger(node_text_to_u32(&child)?)),
                    "Sign" => signed = child.text().unwrap_or("Unsigned") == "Signed",
                    "Endianess" => big_endian = child.text().unwrap_or("BigEndian") == "BigEndian",
                    _ => {}
                }
            }

            if address.is_none() || length.is_none() {
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
                address: Box::new(address.unwrap()),
                length: Box::new(length.unwrap()),
                signed,
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
                            Box::new(get_required_value(&child, name_to_node)?),
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
                    "Value" => value = Some(GenIType::ConstantInteger(node_text_to_u32(&child)?)),
                    "pValue" => {
                        value = match get_required_value(&child, name_to_node) {
                            Ok(v) => Some(v),
                            Err(e) => {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!(
                                        "{} missing required fields: {}",
                                        node.attribute("Name").unwrap_or(""),
                                        e
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
                        let value = node_text_to_u32(&value_child.unwrap())?;
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
            format!(
                "Unsupported node type {} {}",
                node.tag_name().name(),
                node.attribute("Name").unwrap_or("")
            ),
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
    let gen_doc = parse_xml(xml_content)?;
    let (gen_node, name_to_node) = preprocess_xml(&gen_doc)?;

    let mut gen_features_map = HashMap::<String, GenIType>::new();
    for node in gen_node.children() {
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
                gen_features_map.insert(info_name, integer);
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
                gen_features_map.insert(info_name, float);
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
                gen_features_map.insert(info_name, enumeration);
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
                gen_features_map.insert(info_name, cmd);
            }
            _ => {}
        };
    }

    Ok(GenICam::new(gen_features_map))
}

fn preprocess_xml<'a>(
    doc: &'a roxmltree::Document<'a>,
) -> std::io::Result<(
    roxmltree::Node<'a, 'a>,
    HashMap<String, roxmltree::Node<'a, 'a>>,
)> {
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
            if node.is_element() {
                let name = if let Some(name) = node.attribute("Name") {
                    name.to_string()
                } else {
                    // Special case:
                    // <StructReg Comment="XXX">
                    //     <StructEntry Name="YYY" />
                    // Map all entry names to the parent StructReg node.
                    if node.tag_name().name() == "StructReg" {
                        for child_node in node.children() {
                            if child_node.tag_name().name() == "StructEntry"
                                && let Some(entry_name) = child_node.attribute("Name")
                            {
                                name_to_node.insert(entry_name.to_string(), node);
                            }
                        }
                    }

                    continue;
                };
                name_to_node.insert(name, node);
            }
        }
    }
    Ok((genicam_node, name_to_node))
}

fn parse_xml(xml_content: &str) -> std::io::Result<roxmltree::Document<'_>> {
    roxmltree::Document::parse(xml_content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let (gen_node, name_to_node) = preprocess_xml(&gen_doc).unwrap();

        let parse_result = node_to_type(&gen_node.first_element_child().unwrap(), &name_to_node);
        if let Err(e) = &parse_result {
            eprintln!("{:?}", e);
        }
        assert!(parse_result.is_ok());
    }
}
