use crate::error::{Error, Result};
use std::io::Read;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};

#[derive(Debug)]
pub struct ModSettings {
    pub version: u64,
    pub startup: PropertyTree,
    pub runtime_global: PropertyTree,
    pub runtime_per_user: PropertyTree,
}

impl ModSettings {
    pub fn try_from_bytes(input: &[u8]) -> Result<Self> {
        let mut d = Deserialiser { byte_slice: input };

        // First is 8 bytes representing game version
        let version = d.parse_version()?;

        // Next is a single byte always set to false (not 1)
        let false_sentinel = d.parse_bool()?;
        if false_sentinel {
            return Err(Error::Syntax(
                "After-version sentinel expected to be false, got true".to_owned(),
            ));
        }

        // Then is a dictionary-type PropertyTree with empty key
        // This contains the three settings sections
        let startup;
        let runtime_global;
        let runtime_per_user;
        match d.parse_property_tree()? {
            PropertyTree::Dictionary(mut dict) => {
                match dict.remove("startup") {
                    None => {
                        return Err(Error::Syntax(
                            "Settings section 'startup' missing".to_owned(),
                        ))
                    }
                    Some(d) => startup = d,
                };
                match dict.remove("runtime-global") {
                    None => {
                        return Err(Error::Syntax(
                            "Settings section 'runtime-global' missing".to_owned(),
                        ))
                    }
                    Some(d) => runtime_global = d,
                };
                match dict.remove("runtime-per-user") {
                    None => {
                        return Err(Error::Syntax(
                            "Settings section 'runtime-per-user' missing".to_owned(),
                        ))
                    }
                    Some(d) => runtime_per_user = d,
                };
            }
            _ => {
                return Err(Error::Syntax(
                    "Top-level PropertyTree not dictionary type".to_owned(),
                ))
            }
        }

        Ok(ModSettings {
            version,
            startup,
            runtime_global,
            runtime_per_user,
        })
    }
}

struct Deserialiser<'a> {
    byte_slice: &'a [u8],
}

impl<'a> Deserialiser<'a> {
    fn peek_u8(&mut self) -> Result<u8> {
        match self.byte_slice.bytes().next() {
            None => Err(Error::Eof),
            Some(r) => r.map_err(|e| Error::Message(format!("{:?}", e))),
        }
    }

    fn next_u8(&mut self) -> Result<u8> {
        let b = self.peek_u8()?;
        self.byte_slice = &self.byte_slice[1..];
        Ok(b)
    }

    fn next_u16(&mut self) -> Result<u16> {
        let next_slice: &[u8; 2] = &self.byte_slice[0..2]
            .try_into()
            .map_err(|_| Error::ByteSlicingError)?;
        self.byte_slice = &self.byte_slice[2..];
        Ok(u16::from_le_bytes(*next_slice))
    }

    fn next_u32(&mut self) -> Result<u32> {
        let next_slice: &[u8; 4] = &self.byte_slice[0..4]
            .try_into()
            .map_err(|_| Error::ByteSlicingError)?;
        self.byte_slice = &self.byte_slice[4..];
        Ok(u32::from_le_bytes(*next_slice))
    }

    fn parse_bool(&mut self) -> Result<bool> {
        let b = self.next_u8()?;
        Ok(b == 1)
    }

    fn parse_double(&mut self) -> Result<f64> {
        let next_slice: &[u8; 8] = &self.byte_slice[0..8]
            .try_into()
            .map_err(|_| Error::ByteSlicingError)?;
        self.byte_slice = &self.byte_slice[8..];
        Ok(f64::from_le_bytes(*next_slice))
    }

    fn parse_string(&mut self) -> Result<String> {
        // 1 bool indicating if the string is empty
        if self.parse_bool()? {
            Ok(String::new())
        } else {
            // Space-optimised unsigned int representing string length
            // Read 1 preamble byte
            let so_byte = self.next_u8()?;

            // If this value < 255, then use it as our value
            // Otherwise, read the full unsigned int from the next 4 bytes
            let len: u32;
            if so_byte < 255 {
                len = so_byte as u32;
            } else {
                len = self.next_u32()?;
            }

            // Read `len` bytes representing UTF-8 string
            let len = len as usize;
            let next_slice = self.byte_slice[0..len]
                .try_into()
                .map_err(|_| Error::ByteSlicingError)?;
            let utf8 = std::str::from_utf8(next_slice)
                .map_err(|e| Error::Utf8(e))?
                .to_string();
            self.byte_slice = &self.byte_slice[len..];

            Ok(utf8)
        }
    }

    fn parse_version(&mut self) -> Result<u64> {
        let main_version = self.next_u16()?;
        let major_version = self.next_u16()?;
        let minor_version = self.next_u16()?;
        let developer_version = self.next_u16()?;

        let version = developer_version as u64
            | (minor_version as u64) << 16
            | (major_version as u64) << 32
            | (main_version as u64) << 48;
        Ok(version)
    }

    fn parse_property_tree(&mut self) -> Result<PropertyTree> {
        // One byte representing the PropertyTreeType
        let type_u8 = self.next_u8()?;

        // One bool "not important outside of Factorio internals"
        self.parse_bool()?;

        match type_u8.try_into()? {
            PropertyTreeType::None => {
                // Nothing
                Ok(PropertyTree::None)
            }
            PropertyTreeType::Bool => {
                // 1 bool
                Ok(PropertyTree::Bool(self.parse_bool()?))
            }
            PropertyTreeType::Number => {
                // 1 double
                Ok(PropertyTree::Number(self.parse_double()?))
            }
            PropertyTreeType::String => {
                // 1 string
                Ok(PropertyTree::String(self.parse_string()?))
            }
            PropertyTreeType::List => {
                // 1 u32 representing the number of elements
                let len = self.next_u32()?;

                // Iterate over list items
                let mut list = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    // 1 string, unused
                    self.parse_string()?;

                    // 1 property tree
                    list.push(self.parse_property_tree()?);
                }

                Ok(PropertyTree::List(list))
            }
            PropertyTreeType::Dictionary => {
                // 1 u32 representing the number of elements
                let len = self.next_u32()?;

                // Iterate over dict items
                let mut dict = HashMap::with_capacity(len as usize);
                for _ in 0..len {
                    // 1 string representing the key
                    let key = self.parse_string()?;

                    // 1 property tree
                    let value = self.parse_property_tree()?;

                    dict.insert(key, value);
                }

                Ok(PropertyTree::Dictionary(dict))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum PropertyTree {
    None,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<PropertyTree>),
    Dictionary(HashMap<String, PropertyTree>),
}

enum PropertyTreeType {
    None,
    Bool,
    Number,
    String,
    List,
    Dictionary,
}

impl TryFrom<u8> for PropertyTreeType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(PropertyTreeType::None),
            1 => Ok(PropertyTreeType::Bool),
            2 => Ok(PropertyTreeType::Number),
            3 => Ok(PropertyTreeType::String),
            4 => Ok(PropertyTreeType::List),
            5 => Ok(PropertyTreeType::Dictionary),
            _ => Err(Error::OutOfRange),
        }
    }
}

impl TryFrom<PropertyTreeType> for u8 {
    type Error = Error;

    fn try_from(value: PropertyTreeType) -> Result<Self> {
        match value {
            PropertyTreeType::None => Ok(0),
            PropertyTreeType::Bool => Ok(1),
            PropertyTreeType::Number => Ok(2),
            PropertyTreeType::String => Ok(3),
            PropertyTreeType::List => Ok(4),
            PropertyTreeType::Dictionary => Ok(5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn can_convert_between_byte_and_propertytreetype() {
        let bytes: Vec<u8> = (0..6).collect();
        for b in bytes {
            let r = b.try_into();
            assert!(r.is_ok());
            let t: PropertyTreeType = r.unwrap();
            let r2 = t.try_into();
            assert!(r2.is_ok());
            assert_eq!(b, r2.unwrap());
        }
    }
}
