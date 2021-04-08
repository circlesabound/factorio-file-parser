use std::{convert::{TryFrom, TryInto}, fs, path::Path};

use factorio_mod_settings_parser::ModSettings;

#[test]
fn can_deserialise_sample() -> Result<(), Box<dyn std::error::Error>> {
    // read file
    let path = Path::new("tests").join("mod-settings.dat");
    let bytes = fs::read(path)?;

    // attempt to deserialise
    ModSettings::try_from(bytes.as_ref())?;

    Ok(())
}

#[test]
fn can_deserialise_and_serialise_sample() -> Result<(), Box<dyn std::error::Error>> {
    // read file
    let path = Path::new("tests").join("mod-settings.dat");
    let bytes = fs::read(path)?;

    // attempt to deserialise
    let ms = ModSettings::try_from(bytes.as_ref())?;

    // serialise back
    let bytes2: Vec<u8> = ms.try_into()?;

    // this likely won't be equal because the sample is taken from a game, with unknown propertytree ordering
    // so we deserialise again
    let ms2 = ModSettings::try_from(bytes2.as_ref())?;

    // then serialise again
    let bytes3: Vec<u8> = ms2.try_into()?;

    // now assert byte-by-byte equality
    assert_eq!(bytes2, bytes3);

    Ok(())
}
