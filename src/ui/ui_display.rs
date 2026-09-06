use std::fmt;
use crate::sandbox::state::{PetriPermissions,SandboxError};

impl fmt::Display for SandboxError {
    fn fmt(&self,f:&mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxError::InvalidStateTransition => {
                write!(f,"Invalid sandbox state transition")
            }
            SandboxError::SandboxNotFound => {
                write!(f,"Sandbox not found")
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
            SandboxError::PermissionNotFound(permission) => {
                write!(f,"Permission not found: {}",permission)
            }
            SandboxError::IsolationFailed => {
                write!(f,"Failed to isolate sandbox")
            }
            SandboxError::InvalidConfiguration(message) => {
                write!(f,"Invalid sandbox configuration: {}",message)
            }
        }
    }
}

impl fmt::Display for PetriPermissions {
    fn fmt(&self,f:&mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            PetriPermissions::ReadFiles => "read files",
            PetriPermissions::WriteFiles => "write files",
            PetriPermissions::ExecutePrograms => "execute programs",
            PetriPermissions::NetworkAccess => "network access",
        };
        write!(f,"{}",name)
    }
}
