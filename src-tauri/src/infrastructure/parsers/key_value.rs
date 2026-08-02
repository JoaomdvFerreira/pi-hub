use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseWarning(pub String);

/// Parses a `PIHUB_KEY=value` line-oriented payload. Lines that aren't a
/// recognized `PIHUB_`-prefixed key=value pair are ignored with a warning
/// rather than aborting the whole parse. Remote output is only ever
/// treated as data here, never as executable content.
pub fn parse_key_value_payload(raw: &str) -> (HashMap<String, String>, Vec<ParseWarning>) {
    let mut fields = HashMap::new();
    let mut warnings = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match line.split_once('=') {
            Some((key, value)) if key.starts_with("PIHUB_") => {
                fields.insert(key.to_string(), value.to_string());
            }
            Some((key, _)) => {
                warnings.push(ParseWarning(format!("ignored unrecognized key '{key}'")));
            }
            None => {
                warnings.push(ParseWarning(format!("ignored malformed line: '{line}'")));
            }
        }
    }

    (fields, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_formed_lines() {
        let (fields, warnings) =
            parse_key_value_payload("PIHUB_HOSTNAME=pi5\nPIHUB_UPTIME_SECONDS=120\n");
        assert_eq!(fields.get("PIHUB_HOSTNAME"), Some(&"pi5".to_string()));
        assert_eq!(fields.get("PIHUB_UPTIME_SECONDS"), Some(&"120".to_string()));
        assert!(warnings.is_empty());
    }

    #[test]
    fn ignores_blank_lines() {
        let (fields, warnings) = parse_key_value_payload("\nPIHUB_HOSTNAME=pi5\n\n\n");
        assert_eq!(fields.len(), 1);
        assert!(warnings.is_empty());
    }

    #[test]
    fn warns_on_unrecognized_key_but_keeps_going() {
        let (fields, warnings) = parse_key_value_payload("SOMETHING_ELSE=1\nPIHUB_HOSTNAME=pi5\n");
        assert_eq!(fields.get("PIHUB_HOSTNAME"), Some(&"pi5".to_string()));
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn warns_on_malformed_line_without_equals() {
        let (fields, warnings) = parse_key_value_payload("not a valid line\nPIHUB_HOSTNAME=pi5\n");
        assert_eq!(fields.get("PIHUB_HOSTNAME"), Some(&"pi5".to_string()));
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn handles_values_containing_equals_signs() {
        let (fields, _) =
            parse_key_value_payload("PIHUB_OS=Debian GNU/Linux 12 (bookworm)=extra\n");
        assert_eq!(
            fields.get("PIHUB_OS"),
            Some(&"Debian GNU/Linux 12 (bookworm)=extra".to_string())
        );
    }

    #[test]
    fn empty_payload_produces_no_fields_or_warnings() {
        let (fields, warnings) = parse_key_value_payload("");
        assert!(fields.is_empty());
        assert!(warnings.is_empty());
    }
}
