use thiserror::Error;

/// The error type of this crate.
#[derive(Debug, Error)]
pub enum Error {
    /// Registration of module with `module_name` failed.
    ///
    /// Can be returned by [`Context::register_module()`] and [`register_modules!`].
    #[error("Module '{module_name}' already exists, can't register it.")]
    ModuleRegistrationFailed { module_name: String },
    /// Registration of class `class_name` inside `module_name` failed.
    ///
    /// Can be returned by [`Context::register_class()`] and [`register_modules!`].
    #[error("Cannot register foreign class '{class_name}' in module '{module_name}': module either does not exist or it is locked.")]
    ClassRegistrationFailed {
        module_name: String,
        class_name: String,
    },
    /// Registration of method with `method_signature` inside `module_name` failed.
    ///
    /// Can be returned by [`Context::register_fn()`] and [`register_modules!`].
    #[error("Cannot foreign method class '{method_signature}' in module '{module_name}': module either does not exist or it is locked.")]
    MethodRegistrationFailed {
        module_name: String,
        method_signature: String,
    },
}

/// The result of operations in this crate that may fail. Alias of `std::result::Result<(), Error>`.
pub type Result = std::result::Result<(), Error>;
