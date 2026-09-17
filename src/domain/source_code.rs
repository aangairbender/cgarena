use std::ops::Deref;

#[derive(Clone)]
pub struct SourceCode(String);

impl From<String> for SourceCode {
    fn from(src: String) -> Self {
        SourceCode(src)
    }
}

impl From<SourceCode> for String {
    fn from(value: SourceCode) -> Self {
        value.0
    }
}

impl Deref for SourceCode {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SourceCode;

    #[test]
    fn accepts_source_code_over_previous_limit() {
        let source = "a".repeat(100_001);

        let source_code = SourceCode::from(source.clone());

        assert_eq!(&*source_code, source);
    }
}
