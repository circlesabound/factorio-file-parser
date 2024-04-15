use std::{convert::TryFrom, fs, path::Path};

use factorio_file_parser::SaveHeader;

#[test]
fn can_deserialise_vanilla() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("tests").join("vanilla.level-init.dat");
    let bytes = fs::read(path)?;

    // attempt to deserialise
    SaveHeader::try_from(bytes.as_ref())?;

    Ok(())
}


#[test]
fn can_deserialise_withmods() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("tests").join("pyae.level-init.dat");
    let bytes = fs::read(path)?;

    // attempt to deserialise
    SaveHeader::try_from(bytes.as_ref())?;

    Ok(())
}
