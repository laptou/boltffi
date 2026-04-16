#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PythonLowerError {
    #[error(
        "Python top-level function name `{generated_name}` collides between function `{existing_function}` and function `{colliding_function}`"
    )]
    TopLevelFunctionNameCollision {
        generated_name: String,
        existing_function: String,
        colliding_function: String,
    },
    #[error(
        "Python parameter name `{generated_name}` collides in function `{function_name}` between parameter `{existing_parameter}` and parameter `{colliding_parameter}`"
    )]
    ParameterNameCollision {
        function_name: String,
        generated_name: String,
        existing_parameter: String,
        colliding_parameter: String,
    },
}
