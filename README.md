# factorio-file-parser

Rust library to parse various file formats used in the game Factorio:
- Mod settings from the `mod-settings.dat` file
- Save header from `level-init.dat` file from inside a save zip

The logic for parsing mod settings is taken from the information provided in the [Factorio wiki](https://wiki.factorio.com/Mod_settings_file_format), with inspiration from the sample code provided by Factorio dev Rseding91 on the [forums](https://forums.factorio.com/59851).

The save header structure is based on the logic implemented by [OpenFactorioServerManager](https://github.com/OpenFactorioServerManager) which in turn is based on the work of Factorio forum user mickael9 in [this forum thread](https://forums.factorio.com/8568).
