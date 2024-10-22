use std::{convert::TryFrom, fs, path::Path};

use factorio_file_parser::SaveHeader;

#[test]
fn can_deserialise_pre_2_0_vanilla() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("tests").join("vanilla.level-init.dat");
    let bytes = fs::read(path)?;

    // attempt to deserialise
    dbg!(SaveHeader::try_from(bytes.as_ref())?);

    Ok(())
}

#[test]
fn can_deserialise_spaceage_vanilla() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("tests").join("spaceage.level-init.dat");
    let bytes = fs::read(path)?;

    // attempt to deserialise
    dbg!(SaveHeader::try_from(bytes.as_ref())?);

    Ok(())
}

#[test]
fn can_deserialise_spaceage_withmods() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("tests").join("spaceage-withmods.level-init.dat");
    let bytes = fs::read(path)?;

    // attempt to deserialise
    dbg!(SaveHeader::try_from(bytes.as_ref())?);

    Ok(())
}

#[test]
fn can_deserialise_pre_2_0_withmods() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("tests").join("pyae.level-init.dat");
    let bytes = fs::read(path)?;

    // attempt to deserialise
    SaveHeader::try_from(bytes.as_ref())?;

    Ok(())
}
