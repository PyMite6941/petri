use crate::sandbox::state::SandboxError;
use std::fmt;

impl fmt::Display for SandboxError {
    fn fmt(&self,f:&mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxError::InvalidStateTransition => {
                write!(f,"Invalid sandbox state transition")
            }
            SandboxError::InvalidConfiguration(message) => {
                write!(f,"Invalid sandbox configuration: {}",message)
            }
            SandboxError::DirectoryCreationFailed => {
                write!(f,"Failed to create sandbox directory")
            }
            SandboxError::ProcessLaunchFailed(error) => {
                write!(f,"Failed to launch process: {}",error)
            }
            SandboxError::ProcessStopFailed => {
                write!(f,"Failed to stop process")
            }
            SandboxError::IsolationFailed => {
                write!(f,"Failed to isolate sandbox")
            }
            SandboxError::SandboxNotFound => {
                write!(f,"Sandbox not found")
            }
            SandboxError::PermissionNotFound => {
                write!(f,"Permission not found")
            }
            SandboxError::StorageFailed(error) => {
                write!(f,"Storage error: {}",error)
            }
            SandboxError::ConfigSerializeFailed(error) => {
                write!(f,"Failed to write config: {}",error)
            }
            SandboxError::ConfigParseFailed(error) => {
                write!(f,"Failed to read config: {}",error)
            }
        }
    }
}
