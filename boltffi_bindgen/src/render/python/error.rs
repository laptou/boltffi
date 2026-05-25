#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PythonLowerError {
    #[error(
        "Python top-level name `{generated_name}` collides between {existing_subject} and {colliding_subject}"
    )]
    TopLevelNameCollision {
        generated_name: String,
        existing_subject: String,
        colliding_subject: String,
    },
    #[error(
        "Python parameter name `{generated_name}` collides in `{callable_name}` between parameter `{existing_parameter}` and parameter `{colliding_parameter}`"
    )]
    ParameterNameCollision {
        callable_name: String,
        generated_name: String,
        existing_parameter: String,
        colliding_parameter: String,
    },
    #[error(
        "Python record field name `{generated_name}` collides in record `{record_name}` between field `{existing_field}` and field `{colliding_field}`"
    )]
    RecordFieldNameCollision {
        record_name: String,
        generated_name: String,
        existing_field: String,
        colliding_field: String,
    },
    #[error(
        "Python record callable name `{generated_name}` collides in record `{record_name}` between {existing_subject} and {colliding_subject}"
    )]
    RecordCallableNameCollision {
        record_name: String,
        generated_name: String,
        existing_subject: String,
        colliding_subject: String,
    },
    #[error(
        "Python enum member name `{generated_name}` collides in enum `{enum_name}` between variant `{existing_variant}` and variant `{colliding_variant}`"
    )]
    EnumMemberNameCollision {
        enum_name: String,
        generated_name: String,
        existing_variant: String,
        colliding_variant: String,
    },
    #[error(
        "Python enum member name `{generated_name}` collides in enum `{enum_name}` between {existing_subject} and {colliding_subject}"
    )]
    EnumCallableNameCollision {
        enum_name: String,
        generated_name: String,
        existing_subject: String,
        colliding_subject: String,
    },
    #[error(
        "Python native module name `{generated_name}` collides between {existing_subject} and {colliding_subject}"
    )]
    NativeModuleNameCollision {
        generated_name: String,
        existing_subject: String,
        colliding_subject: String,
    },
}
