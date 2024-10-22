use crate::error::{Error, Result};
use std::fmt::{Debug, Display};
use std::io::Read;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ModSettings {
    pub version: Version,
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
                version: version.into(),
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
        s.write_version(u64::from(self.version));

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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SaveHeader {
    pub factorio_version: Version,
    /// Name of campaign e.g. `freeplay` or `transport-belt-madness`
    pub campaign: String,
    /// Name of the campaign level
    pub name: String,
    /// Name of the base mod, should always be `base`
    pub base_mod: String,
    pub difficulty: u8,
    // not sure??
    pub finished: bool,
    /// Whether the victory condition has been satisfied
    pub player_won: bool,
    /// Name of the subsequent campaign level
    pub next_level: String,
    /// not sure??
    pub can_continue: bool,
    /// If game is finished, but player has chosen to continue??
    pub finished_but_continuing: bool,
    /// Whether a replay is being recorded
    pub saving_replay: bool,
    /// not sure??
    pub allow_non_admin_debug_options: bool,
    /// version of the game this save was loaded from
    pub loaded_from: Version48,
    /// build of the game this save was loaded from
    pub loaded_from_build: BuildNumber,
    /// whether commands are allowed
    pub allowed_commands: bool,
    /// list of mods attached to the save
    pub mods: Vec<SaveHeaderMod>,
}

impl TryFrom<&[u8]> for SaveHeader {
    type Error = Error;

    fn try_from(input: &[u8]) -> Result<Self> {
        let mut d = Deserialiser { byte_slice: input };

        // First is 8 bytes representing game version
        let factorio_version = d.parse_version()?;

        // Next is a single unused byte
        let _ = d.parse_bool()?;

        let campaign = d.parse_string_saveheader()?;

        let name = d.parse_string_saveheader()?;

        let base_mod = d.parse_string_saveheader()?;

        // Next is a number representing difficulty
        let difficulty = d.next_u8()?;

        let finished = d.parse_bool()?;

        let player_won = d.parse_bool()?;

        let next_level = d.parse_string_saveheader()?;

        let can_continue = d.parse_bool()?;

        let finished_but_continuing = d.parse_bool()?;

        let saving_replay = d.parse_bool()?;

        let allow_non_admin_debug_options = d.parse_bool()?;

        let loaded_from = d.parse_version48()?;

        let loaded_from_build = match factorio_version.main >= 2 {
            true => BuildNumber::Build32(d.next_u32()?),
            false => BuildNumber::Build16(d.next_u16()?),
        };

        let allowed_commands = d.parse_bool()?;

        // 2.0 seems to have introduced 4 new bytes here, not sure what they are
        // All test samples seem to have these exact bytes:
        //   00 00 A0 00
        // Skip them for now
        if factorio_version.main >= 2 {
            for _ in 0..4 {
                d.next_u8()?;
            }
        }

        // Next is the number of mods attached to the save
        let num_mods = d.next_u32_optim()?;
        let mut mods = Vec::with_capacity(num_mods as usize);
        // Iterate and build SaveHeaderMods
        for _ in 0..num_mods {
            mods.push(SaveHeaderMod {
                name: d.parse_string_saveheader()?,
                version: d.parse_version48()?,
                crc: d.next_u32()?,
            });
        }

        Ok(SaveHeader {
            factorio_version,
            campaign,
            name,
            base_mod,
            difficulty,
            finished,
            player_won,
            next_level,
            can_continue,
            finished_but_continuing,
            saving_replay,
            allow_non_admin_debug_options,
            loaded_from,
            loaded_from_build,
            allowed_commands,
            mods,
        })
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SaveHeaderMod {
    pub name: String,
    pub version: Version48,
    pub crc: u32,
}

impl Display for SaveHeaderMod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.version)
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

    fn next_u16_optim(&mut self) -> Result<u16> {
        let byte = self.next_u8()?;
        if byte != 0xFF {
            Ok(byte as u16)
        } else {
            self.next_u16()
        }
    }

    fn next_u32(&mut self) -> Result<u32> {
        let next_slice: &[u8; 4] = &self.byte_slice[0..4]
            .try_into()
            .map_err(|_| Error::ByteSlicingError)?;
        self.byte_slice = &self.byte_slice[4..];
        Ok(u32::from_le_bytes(*next_slice))
    }

    fn next_u32_optim(&mut self) -> Result<u32> {
        // Read 1 preamble byte
        let so_byte = self.next_u8()?;
        if so_byte != 0xFF {
            // If this value < 255, then use it as our value
            Ok(so_byte as u32)
        } else {
            // Otherwise, read the full unsigned int from the next 4 bytes
            self.next_u32()
        }
    }

    fn parse_bool(&mut self) -> Result<bool> {
        let b = self.next_u8()?;
        Ok(b != 0)
    }

    fn parse_double(&mut self) -> Result<f64> {
        let next_slice: &[u8; 8] = &self.byte_slice[0..8]
            .try_into()
            .map_err(|_| Error::ByteSlicingError)?;
        self.byte_slice = &self.byte_slice[8..];
        Ok(f64::from_le_bytes(*next_slice))
    }

    fn parse_string(&mut self) -> Result<String> {
        self._parse_string(true)
    }

    fn parse_string_saveheader(&mut self) -> Result<String> {
        self._parse_string(false)
    }

    fn _parse_string(&mut self, has_empty_indicator: bool) -> Result<String> {
        // in mod-settings dat, there is an extra byte indicating if the string is empty?
        if has_empty_indicator && self.parse_bool()? {
            Ok(String::new())
        } else {
            let len = self.next_u32_optim()?;

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

    fn parse_version(&mut self) -> Result<Version> {
        let main = self.next_u16()?;
        let major = self.next_u16()?;
        let minor = self.next_u16()?;
        let developer = self.next_u16()?;

        Ok(Version {
            main,
            major,
            minor,
            developer,
        })
    }

    fn parse_version48(&mut self) -> Result<Version48> {
        let main = self.next_u16_optim()?;
        let major = self.next_u16_optim()?;
        let minor = self.next_u16_optim()?;

        Ok(Version48 {
            main,
            major,
            minor,
        })
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Version {
    main: u16,
    major: u16,
    minor: u16,
    developer: u16,
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.main, self.major, self.minor, self.developer)
    }
}

impl From<Version> for u64 {
    fn from(value: Version) -> Self {
        let ret = value.developer as u64
            | (value.minor as u64) << 16
            | (value.major as u64) << 32
            | (value.main as u64) << 48;
        ret
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Version48 {
    main: u16,
    major: u16,
    minor: u16,
}

impl Display for Version48 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.main, self.major, self.minor)
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum BuildNumber {
    Build16(u16),
    Build32(u32),
}

impl Display for BuildNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let b = match self {
            BuildNumber::Build16(x) => x.to_string(),
            BuildNumber::Build32(x) => x.to_string(),
        };
        write!(f, "{}", b)
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
