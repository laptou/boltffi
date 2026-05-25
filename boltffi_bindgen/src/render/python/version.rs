use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PythonRuntimeVersion {
    major: u8,
    minor: u8,
}

impl PythonRuntimeVersion {
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    pub const fn minimum_supported() -> Self {
        Self::new(3, 10)
    }

    pub fn package_requirement(self) -> String {
        format!(">={self}")
    }
}

impl Display for PythonRuntimeVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::PythonRuntimeVersion;

    #[test]
    fn formats_package_requirement_from_version_floor() {
        assert_eq!(
            PythonRuntimeVersion::minimum_supported().package_requirement(),
            ">=3.10"
        );
    }
}
