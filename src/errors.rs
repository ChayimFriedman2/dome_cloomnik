use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Module '{module_name}' already exists, can't register it.")]
    ModuleRegistrationFailed { module_name: String },
    #[error("Cannot register foreign class '{class_name}' in module '{module_name}': module either does not exist or it is locked.")]
    ClassRegistrationFailed {
        module_name: String,
        class_name: String,
    },
    #[error("Cannot foreign method class '{method_signature}' in module '{module_name}': module either does not exist or it is locked.")]
    MethodRegistrationFailed {
        module_name: String,
        method_signature: String,
    },
}

pub type Result = std::result::Result<(), Error>;
