use std::collections::HashMap;

/// Parsed ROM-derived text export. Rendering here deliberately supports only
/// literal text/newlines/end; character substitutions and other controls must
/// be implemented before entries that use them can execute.
#[derive(Clone, Default)]
pub struct DialogueCatalog {
    entries: HashMap<u16, String>,
}

impl DialogueCatalog {
    pub fn parse(source: &str) -> Self {
        let mut entries = HashMap::new();
        for section in source.split("#text:").skip(1) {
            let Some((label, body)) = section.split_once('\n') else { continue };
            let Some(id) = label.trim().strip_prefix("DLG_").and_then(|id| id.parse().ok()) else { continue };
            // Physical line breaks format the export; {n} is the encoded
            // game newline. Keeping both would insert blank dialogue rows.
            entries.insert(id, body.replace('\r', "").replace('\n', "").trim().to_owned());
        }
        Self { entries }
    }

    pub fn literal(&self, id: u16) -> Result<String, String> {
        let mut rest = self.entries.get(&id).map(String::as_str)
            .ok_or_else(|| format!("DLG_{id} is absent from the target text export"))?;
        let mut result = String::new();
        while let Some(start) = rest.find('{') {
            result.push_str(&rest[..start]);
            let Some(end) = rest[start..].find('}').map(|offset| start + offset) else {
                return Err(format!("DLG_{id} contains an unterminated control"));
            };
            match &rest[start + 1..end] {
                "0" => return Ok(result.trim().to_owned()),
                "n" => result.push('\n'),
                control => return Err(format!("DLG_{id} requires unsupported control {{{control}}}")),
            }
            rest = &rest[end + 1..];
        }
        result.push_str(rest);
        Ok(result.trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::DialogueCatalog;

    #[test]
    fn labels_newlines_and_terminators_are_preserved() {
        let catalog = DialogueCatalog::parse("#text:DLG_11\nWedge: Forward!{n}\nMove out!{0}unused\n#text:DLG_12\nNext{0}\n");
        assert_eq!(catalog.literal(11).unwrap(), "Wedge: Forward!\nMove out!");
        assert_eq!(catalog.literal(12).unwrap(), "Next");
        assert!(catalog.literal(13).is_err());
    }

    #[test]
    fn unsupported_controls_are_reported_instead_of_dropped() {
        let catalog = DialogueCatalog::parse("#text:DLG_1\nHello {TERRA}!{0}\n");
        assert!(catalog.literal(1).unwrap_err().contains("unsupported control {TERRA}"));
    }
}
