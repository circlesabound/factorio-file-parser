use std::{fs, path::Path};

use factorio_mod_settings_parser::ModSettings;

#[test]
fn test_deserialise() -> Result<(), Box<dyn std::error::Error>> {
    // read file
    let path = Path::new("tests").join("mod-settings.dat");
    let bytes = fs::read(path)?;

    // attempt to deserialise
    ModSettings::try_from_bytes(&bytes)?;

    Ok(())
}
