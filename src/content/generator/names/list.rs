//! The name parts: stems, endings and reserved names, read from the plain
//! text file `assets/names/names.txt` (SGEN-80). The file is built into
//! the program, so the generator needs no file system and every build
//! names a sector the same way.

use std::sync::OnceLock;

/// The bundled name file.
const NAMES_FILE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/names/names.txt"
));

/// The parts names are made of (SGEN-80).
#[derive(Debug, Clone, PartialEq)]
pub struct NameList {
    pub stems: Vec<String>,
    pub endings: Vec<String>,
    /// Names never made, in lower case (SGEN-81).
    pub reserved: Vec<String>,
}

/// Which part of the file a line belongs to.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Section {
    Stems,
    Endings,
    Reserved,
}

impl NameList {
    /// Reads a name file: sections `[stems]`, `[endings]` and
    /// `[reserved]`, one entry per line; blank lines and lines starting
    /// with `#` are skipped. Stems and endings are kept in lower case.
    /// An unknown section, or an entry before any section, is an error
    /// naming the line.
    pub fn parse(text: &str) -> Result<NameList, String> {
        let mut list = NameList {
            stems: Vec::new(),
            endings: Vec::new(),
            reserved: Vec::new(),
        };
        let mut section = None;
        for (number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = Some(match name {
                    "stems" => Section::Stems,
                    "endings" => Section::Endings,
                    "reserved" => Section::Reserved,
                    other => return Err(format!("line {}: unknown section [{other}]", number + 1)),
                });
                continue;
            }
            let entry = line.to_lowercase();
            match section {
                Some(Section::Stems) => list.stems.push(entry),
                Some(Section::Endings) => list.endings.push(entry),
                Some(Section::Reserved) => list.reserved.push(entry),
                None => {
                    return Err(format!(
                        "line {}: {line:?} is before any section",
                        number + 1
                    ));
                }
            }
        }
        Ok(list)
    }

    /// The bundled name file, read once.
    pub fn bundled() -> &'static NameList {
        static LIST: OnceLock<NameList> = OnceLock::new();
        LIST.get_or_init(|| NameList::parse(NAMES_FILE).expect("the bundled name file is valid"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SGEN-80, SGEN-81: the bundled file has at least 120 stems and 40
    /// endings, and reserves Corvane and Sol.
    #[test]
    fn the_bundled_file_has_enough_parts() {
        let list = NameList::bundled();
        assert!(list.stems.len() >= 120, "{} stems", list.stems.len());
        assert!(list.endings.len() >= 40, "{} endings", list.endings.len());
        assert_eq!(&list.reserved[..2], ["corvane", "sol"]);
        let mut stems = list.stems.clone();
        stems.sort();
        stems.dedup();
        assert_eq!(stems.len(), list.stems.len(), "no stem is listed twice");
    }

    /// SGEN-80: sections, comments and blank lines are read; a bad
    /// section or a stray entry is an error naming its line.
    #[test]
    fn the_name_file_reads_its_sections() {
        let list =
            NameList::parse("# parts\n[stems]\nBel\n\n[endings]\nane\n[reserved]\nSol\n").unwrap();
        assert_eq!(list.stems, ["bel"]);
        assert_eq!(list.endings, ["ane"]);
        assert_eq!(list.reserved, ["sol"]);
        assert!(
            NameList::parse("[moons]\nx\n")
                .unwrap_err()
                .contains("line 1")
        );
        assert!(NameList::parse("bel\n").unwrap_err().contains("line 1"));
    }
}
