use std::collections::HashMap;
use thiserror::Error;

/// Represents a single memory patch with an address and its corresponding data.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Patch {
    pub address: u64,
    pub data: Vec<u8>,
}

/// Represents the entire content of a parsed .pchtxt file.
///
/// It holds the NSO Build ID, any flags found, and a list of patches
/// that were located within the first `@enabled` / `@stop` section.
#[derive(Debug, PartialEq, Default)]
pub struct PchtxtFile {
    pub nsobid: Option<String>,
    pub flags: HashMap<String, String>,
    pub patches: Vec<Patch>,
}

/// Defines possible errors that can occur during parsing.
#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("Invalid hex value for address: '{0}'")]
    InvalidAddress(String),
    #[error("Invalid hex value for data: '{0}'")]
    InvalidData(String),
}

impl PchtxtFile {
    /// Parses a string slice containing the content of a .pchtxt file.
    ///
    /// # Arguments
    ///
    /// * `content` - A string slice that holds the .pchtxt data.
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok(PchtxtFile)` on success or a `Err(ParseError)` on failure.
    ///
    /// # Example
    ///
    /// ```
    /// use pchtxt_parser::PchtxtFile;
    ///
    /// let pchtxt_content = r#"
    /// @nsobid-XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
    /// @flag offset_shift 0x100
    /// @enabled
    /// 00b92780 C10A94D2
    /// @stop
    /// "#;
    ///
    /// let parsed_file = PchtxtFile::parse(pchtxt_content).unwrap();
    /// assert_eq!(parsed_file.patches.len(), 1);
    /// assert_eq!(parsed_file.flags.get("offset_shift"), Some(&"0x100".to_string()));
    /// ```
    pub fn parse(content: &str) -> Result<Self, ParseError> {
        let mut file = PchtxtFile::default();
        let mut is_in_enabled_section = false;

        for line in content.lines() {
            let trimmed_line = line.split(['#', '/']).next().unwrap_or("").trim();

            if trimmed_line.is_empty() {
                continue;
            }

            if trimmed_line.starts_with('@') {
                let directive: Vec<&str> = trimmed_line.split_whitespace().collect();
                if let Some(command) = directive.first() {
                    match *command {
                        "@enabled" => is_in_enabled_section = true,
                        "@stop" => break,
                        "@flag" => match directive.len() {
                            2 => {
                                file.flags.insert(directive[1].to_string(), String::new());
                            }
                            3 => {
                                file.flags
                                    .insert(directive[1].to_string(), directive[2].to_string());
                            }
                            _ => { /* Ignore malformed flag directives */ }
                        },
                        _ if command.starts_with("@nsobid-") => {
                            file.nsobid = command.strip_prefix("@nsobid-").map(String::from);
                        }
                        _ => { /* Ignore other unknown directives */ }
                    }
                }
            } else if is_in_enabled_section {
                let parts: Vec<&str> = trimmed_line.split_whitespace().collect();
                if parts.len() == 2 {
                    let addr_str = parts[0];
                    let data_str = parts[1];

                    let address = u64::from_str_radix(addr_str, 16)
                        .map_err(|_| ParseError::InvalidAddress(addr_str.to_string()))?;

                    let data = hex_string_to_bytes(data_str)
                        .map_err(|_| ParseError::InvalidData(data_str.to_string()))?;

                    file.patches.push(Patch { address, data });
                }
            }
        }
        Ok(file)
    }
}

fn hex_string_to_bytes(hex_str: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..hex_str.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16))
        .collect()
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_PCHTXT: &str = r#"
@nsobid-709BFD66115298640155FCC4979DBA151C7CC79A

# Pokémon Violet [0100A3D008C5C000] v4.0.0 - 60FPS

@flag print_values
@flag offset_shift 0x100

// 60FPS

@enabled
02b7a874 21008052
00b92780 C10A94D2
00b92784 813FA0F2
@stop

// A section outside the enabled block should be ignored
01234567 89ABCDEF

@enabled // Re-entering an enabled section should be ignored
02b7c540 00008052
@stop

@Fl4sh_#9174
https://github.com/Fl4sh9174
"#;

    #[test]
    fn it_parses_the_example_file_correctly() {
        let parsed = PchtxtFile::parse(SAMPLE_PCHTXT).unwrap();

        // Test NSOBID
        assert_eq!(
            parsed.nsobid,
            Some("709BFD66115298640155FCC4979DBA151C7CC79A".to_string())
        );

        // Test flags
        assert_eq!(parsed.flags.len(), 2);
        assert_eq!(parsed.flags.get("offset_shift"), Some(&"0x100".to_string()));

        // Test number of patches
        assert_eq!(parsed.patches.len(), 3);

        // Test first patch
        assert_eq!(
            parsed.patches[0],
            Patch {
                address: 0x02b7a874,
                data: vec![0x21, 0x00, 0x80, 0x52]
            }
        );

        // Test last patch
        assert_eq!(
            parsed.patches.last().unwrap(),
            &Patch {
                address: 0x00b92784,
                data: vec![0x81, 0x3F, 0xA0, 0xF2]
            }
        );
    }

    #[test]
    fn it_handles_invalid_address() {
        let content = "@enabled\nNOTANADDRESS 1234\n@stop";
        let result = PchtxtFile::parse(content);
        assert_eq!(
            result,
            Err(ParseError::InvalidAddress("NOTANADDRESS".to_string()))
        );
    }

    #[test]
    fn it_handles_invalid_data() {
        let content = "@enabled\n12345678 NOTHEX\n@stop";
        let result = PchtxtFile::parse(content);
        assert_eq!(result, Err(ParseError::InvalidData("NOTHEX".to_string())));
    }

    #[test]
    fn it_handles_empty_input() {
        let content = "";
        let parsed = PchtxtFile::parse(content).unwrap();
        assert_eq!(parsed, PchtxtFile::default());
    }
}
