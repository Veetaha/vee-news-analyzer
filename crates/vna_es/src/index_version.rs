#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct IndexVersion(u32);

impl Default for IndexVersion {
    fn default() -> Self {
        Self(1)
    }
}

impl IndexVersion {
    pub fn incremented(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn from_index_name(name: &str) -> Option<Self> {
        let index = name.rfind('_')?;
        Some(Self(name[index + 1..].parse().ok()?))
    }

    pub fn attach_to_alias(self, alias: &str) -> String {
        format!("{}_{}", alias, self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_parses(index_name: &str, expected: Option<u32>) {
        assert_eq!(
            IndexVersion::from_index_name(index_name),
            expected.map(IndexVersion)
        );
    }

    #[test]
    fn parses_valid_index_name_with_version() {
        assert_parses("_1", Some(1));
        assert_parses("blah_0", Some(0));
        assert_parses("blah_blah_bruh_42", Some(42));
    }

    #[test]
    fn returns_none_on_invalid_index_name_with_version() {
        assert_parses("", None);
        assert_parses("_", None);
        assert_parses("bruh", None);
        assert_parses("bruh_", None);
        assert_parses("_bruh", None);
        assert_parses("bruh___4_a", None);
    }
}
