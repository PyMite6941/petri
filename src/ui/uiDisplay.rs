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
            SandboxError::DirectoryCreationFailed(error) => {
                write!(f,"Failed to create sandbox directory: {}",error)
            }
            SandboxError::ProcessLaunchFailed(error) => {
                write!(f,"Failed to launch process: {}",error)
            }
            SandboxError::ProcessStopFailed(error) => {
                write!(f,"Failed to stop process: {}",error)
            }
            SandboxError::IsolationFailed(message) => {
                write!(f,"Failed to isolate sandbox: {}",message)
            }
        }
    }
}