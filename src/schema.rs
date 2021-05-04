use crate::error::{Error, Result};
use std::io::Read;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ModSettings {
    pub version: u64,
    pub startup: PropertyTree,
    pub runtime_global: PropertyTree,
    pub runtime_per_user: PropertyTree,
}

impl TryFrom<&[u8]> for ModSettings {
    type Error = Error;

    fn try_from(input: &[u8]) -> Result<Self> {
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
            PropertyTree::Dictionary(dict) => {
                let mut dict: HashMap<String, PropertyTree> = dict.into_iter().collect();
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

        // Should be at EOF now
        if let Err(Error::Eof) = d.peek_u8() {
            Ok(ModSettings {
                version,
                startup,
                runtime_global,
                runtime_per_user,
            })
        } else {
            Err(Error::TrailingBytes)
        }
    }
}

impl TryInto<Vec<u8>> for ModSettings {
    type Error = Error;

    fn try_into(self) -> Result<Vec<u8>> {
        let mut s = Serialiser::new();

        // Write the version first
        s.write_version(self.version);

        // Next is a bool always set to false
        s.write_bool(false);

        // Construct our top-level property tree, then write it
        let mut dict = Vec::with_capacity(3);
        dict.push(("startup".to_owned(), self.startup));
        dict.push(("runtime-global".to_owned(), self.runtime_global));
        dict.push(("runtime-per-user".to_owned(), self.runtime_per_user));
        let top_level = PropertyTree::Dictionary(dict);
        s.write_property_tree(top_level)?;

        // Done
        Ok(s.bytes)
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
                let mut dict = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    // 1 string representing the key
                    let key = self.parse_string()?;

                    // 1 property tree
                    let value = self.parse_property_tree()?;

                    dict.push((key, value));
                }

                Ok(PropertyTree::Dictionary(dict))
            }
        }
    }
}

struct Serialiser {
    bytes: Vec<u8>,
}

impl Serialiser {
    fn new() -> Self {
        Serialiser { bytes: Vec::new() }
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes.push(value)
    }

    fn write_u16(&mut self, value: u16) {
        self.bytes.extend(value.to_le_bytes().iter())
    }

    fn write_u32(&mut self, value: u32) {
        self.bytes.extend(value.to_le_bytes().iter())
    }

    fn write_bool(&mut self, value: bool) {
        let byte = match value {
            true => 1,
            false => 0,
        };
        self.write_u8(byte)
    }

    fn write_double(&mut self, value: f64) {
        self.bytes.extend(value.to_le_bytes().iter())
    }

    fn write_version(&mut self, version: u64) {
        let main_version = (version >> 48) as u16;
        self.write_u16(main_version);
        let major_version = (version >> 32) as u16;
        self.write_u16(major_version);
        let minor_version = (version >> 16) as u16;
        self.write_u16(minor_version);
        let developer_version = version as u16;
        self.write_u16(developer_version);
    }

    fn write_string(&mut self, value: String) {
        // 1 bool indicating if the string is empty
        if value.is_empty() {
            self.write_bool(true);
        } else {
            self.write_bool(false);

            // Space-optimised unsigned int representing string length
            if value.len() < 255 {
                // If the value < 255 then write the value as a u8
                self.write_u8(value.len() as u8);
            } else {
                // Otherwise write a single byte with value 255, then write our full u32
                self.write_u8(255);
                self.write_u32(value.len() as u32); // assuming usize fits into u32
            }

            // Now write the string encoded as UTF-8
            self.bytes.extend(value.into_bytes());
        }
    }

    fn write_property_tree(&mut self, value: PropertyTree) -> Result<()> {
        match value {
            PropertyTree::None => {
                // 1 byte representing PropertyTreeType
                self.write_u8(PropertyTreeType::None.try_into()?);

                // 1 bool "not important outside of Factorio internals"
                self.write_bool(false);
            }
            PropertyTree::Bool(bool) => {
                // 1 byte representing PropertyTreeType
                self.write_u8(PropertyTreeType::Bool.try_into()?);

                // 1 bool "not important outside of Factorio internals"
                self.write_bool(false);

                // 1 bool, the actual value
                self.write_bool(bool);
            }
            PropertyTree::Number(double) => {
                // 1 byte representing PropertyTreeType
                self.write_u8(PropertyTreeType::Number.try_into()?);

                // 1 bool "not important outside of Factorio internals"
                self.write_bool(false);

                // 1 double
                self.write_double(double);
            }
            PropertyTree::String(string) => {
                // 1 byte representing PropertyTreeType
                self.write_u8(PropertyTreeType::String.try_into()?);

                // 1 bool "not important outside of Factorio internals"
                self.write_bool(false);

                // 1 string
                self.write_string(string);
            }
            PropertyTree::List(list) => {
                // 1 byte representing PropertyTreeType
                self.write_u8(PropertyTreeType::List.try_into()?);

                // 1 bool "not important outside of Factorio internals"
                self.write_bool(false);

                // 1 u32 representing the number of elements
                self.write_u32(list.len() as u32);

                // Iterate over list items
                for item in list {
                    // 1 string, unused
                    self.write_string(String::new());

                    // 1 property tree
                    self.write_property_tree(item)?;
                }
            }
            PropertyTree::Dictionary(dict) => {
                // 1 byte representing PropertyTreeType
                self.write_u8(PropertyTreeType::Dictionary.try_into()?);

                // 1 bool "not important outside of Factorio internals"
                self.write_bool(false);
                // 1 u32 representing the number of elements
                self.write_u32(dict.len() as u32);

                // Iterate over dict items
                for (k, v) in dict {
                    // 1 string representing the key
                    self.write_string(k);

                    // 1 property tree
                    self.write_property_tree(v)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum PropertyTree {
    None,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<PropertyTree>),
    Dictionary(Vec<(String, PropertyTree)>),
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
