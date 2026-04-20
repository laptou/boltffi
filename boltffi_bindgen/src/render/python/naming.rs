use boltffi_ffi_rules::naming;
use heck::{ToShoutySnakeCase, ToUpperCamelCase};

pub struct NamingConvention;

impl NamingConvention {
    pub fn function_name(name: &str) -> String {
        Self::escape_keyword(&naming::to_snake_case(name))
    }

    pub fn method_name(name: &str) -> String {
        Self::escape_keyword(&naming::to_snake_case(name))
    }

    pub fn param_name(name: &str) -> String {
        Self::escape_keyword(&naming::to_snake_case(name))
    }

    pub fn class_name(name: &str) -> String {
        Self::escape_keyword(&name.to_upper_camel_case())
    }

    pub fn enum_member_name(name: &str) -> String {
        name.to_shouty_snake_case()
    }

    pub fn native_member_name(owner_name: &str, member_name: &str) -> String {
        format!(
            "_boltffi_{}_{}",
            naming::to_snake_case(owner_name),
            naming::to_snake_case(member_name)
        )
    }

    pub fn reserved_int_enum_member_names() -> &'static [&'static str] {
        &["name", "value"]
    }

    pub fn is_reserved_int_enum_callable_name(name: &str) -> bool {
        Self::reserved_int_enum_member_names().contains(&name)
            || Self::is_dunder_name(name)
            || Self::is_sunder_name(name)
    }

    pub fn native_loader_name() -> &'static str {
        "_initialize_loader"
    }

    pub fn is_valid_module_name(name: &str) -> bool {
        Self::is_identifier(name) && !Self::is_python_keyword(name)
    }

    fn escape_keyword(name: &str) -> String {
        if Self::is_python_keyword(name) {
            format!("{name}_")
        } else {
            name.to_string()
        }
    }

    fn is_dunder_name(name: &str) -> bool {
        name.len() > 4
            && name.starts_with("__")
            && name.ends_with("__")
            && !name[2..].starts_with('_')
            && !name[..name.len() - 2].ends_with('_')
    }

    fn is_sunder_name(name: &str) -> bool {
        name.len() > 2
            && name.starts_with('_')
            && name.ends_with('_')
            && !name[1..].starts_with('_')
            && !name[..name.len() - 1].ends_with('_')
    }

    fn is_identifier(name: &str) -> bool {
        let mut characters = name.chars();
        let Some(first_character) = characters.next() else {
            return false;
        };

        (first_character == '_' || first_character.is_alphabetic())
            && characters.all(|character| character == '_' || character.is_alphanumeric())
    }

    fn is_python_keyword(name: &str) -> bool {
        matches!(
            name,
            "False"
                | "None"
                | "True"
                | "and"
                | "as"
                | "assert"
                | "async"
                | "await"
                | "break"
                | "case"
                | "class"
                | "continue"
                | "def"
                | "del"
                | "elif"
                | "else"
                | "except"
                | "finally"
                | "for"
                | "from"
                | "global"
                | "if"
                | "import"
                | "in"
                | "is"
                | "lambda"
                | "match"
                | "nonlocal"
                | "not"
                | "or"
                | "pass"
                | "raise"
                | "return"
                | "try"
                | "type"
                | "while"
                | "with"
                | "yield"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::NamingConvention;

    #[test]
    fn escapes_python_keywords() {
        assert_eq!(NamingConvention::function_name("class"), "class_");
        assert_eq!(NamingConvention::function_name("match"), "match_");
        assert_eq!(NamingConvention::param_name("from"), "from_");
        assert_eq!(NamingConvention::param_name("type"), "type_");
    }

    #[test]
    fn lower_names_follow_python_casing() {
        assert_eq!(NamingConvention::class_name("direction"), "Direction");
        assert_eq!(
            NamingConvention::enum_member_name("pending_delivery"),
            "PENDING_DELIVERY"
        );
        assert_eq!(
            NamingConvention::method_name("from_degrees"),
            "from_degrees"
        );
        assert_eq!(
            NamingConvention::native_member_name("direction", "from_degrees"),
            "_boltffi_direction_from_degrees"
        );
    }

    #[test]
    fn validates_python_module_names() {
        assert!(NamingConvention::is_valid_module_name("demo_runtime"));
        assert!(!NamingConvention::is_valid_module_name("demo-runtime"));
        assert!(!NamingConvention::is_valid_module_name("demo.runtime"));
        assert!(!NamingConvention::is_valid_module_name("3demo"));
        assert!(!NamingConvention::is_valid_module_name("class"));
    }

    #[test]
    fn exposes_reserved_int_enum_member_names() {
        assert_eq!(
            NamingConvention::reserved_int_enum_member_names(),
            &["name", "value"]
        );
    }

    #[test]
    fn rejects_reserved_int_enum_callable_names() {
        assert!(NamingConvention::is_reserved_int_enum_callable_name("name"));
        assert!(NamingConvention::is_reserved_int_enum_callable_name(
            "value"
        ));
        assert!(NamingConvention::is_reserved_int_enum_callable_name(
            "__new__"
        ));
        assert!(NamingConvention::is_reserved_int_enum_callable_name(
            "_missing_"
        ));
        assert!(!NamingConvention::is_reserved_int_enum_callable_name(
            "label"
        ));
        assert!(!NamingConvention::is_reserved_int_enum_callable_name(
            "_private"
        ));
    }

    #[test]
    fn exposes_native_loader_name() {
        assert_eq!(NamingConvention::native_loader_name(), "_initialize_loader");
    }
}
