mod emit;
mod lower;
mod mappings;
mod names;
mod plan;
mod templates;

pub use emit::{JavaEmitter, JavaFile, JavaOutput};
pub use lower::JavaLowerer;
pub use names::NamingConvention;
pub use plan::*;

use boltffi_ffi_rules::naming::{LibraryName, Name};

#[derive(Debug, Clone)]
pub struct JavaOptions {
    pub library_name: Option<Name<LibraryName>>,
    pub min_java_version: JavaVersion,
    pub desktop_loader: bool,
}

impl Default for JavaOptions {
    fn default() -> Self {
        Self {
            library_name: None,
            min_java_version: JavaVersion::default(),
            desktop_loader: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct JavaVersion(pub u8);

impl Default for JavaVersion {
    fn default() -> Self {
        Self(8)
    }
}

impl JavaVersion {
    pub const JAVA_8: Self = Self(8);
    pub const JAVA_11: Self = Self(11);
    pub const JAVA_17: Self = Self(17);
    pub const JAVA_21: Self = Self(21);
    pub const JAVA_22: Self = Self(22);
    pub const JAVA_23: Self = Self(23);
    pub const JAVA_24: Self = Self(24);

    pub fn supports_flow_api(&self) -> bool {
        self.0 >= 9
    }

    pub fn supports_records(&self) -> bool {
        self.0 >= 16
    }

    pub fn supports_sealed(&self) -> bool {
        self.0 >= 17
    }

    pub fn supports_virtual_threads(&self) -> bool {
        self.0 >= 21
    }

    pub fn supports_completable_future(&self) -> bool {
        self.0 >= 8
    }

    pub fn supports_cleaner(&self) -> bool {
        self.0 >= 9
    }
}
