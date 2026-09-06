#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetriState {
    Created,
    Starting,
    Running,
    Stopping,
    Stopped,
    Destroyed,
}

impl PetriState {
    pub fn can_transition(&self, next: &PetriState) -> bool {
        match (self,next) {
            (PetriState::Created,PetriState::Starting) => true,
            (PetriState::Starting,PetriState::Stopped) => true,
            (PetriState::Starting,PetriState::Running) => true,
            (PetriState::Running,PetriState::Stopping) => true,
            (PetriState::Running,PetriState::Destroyed) => true,
            (PetriState::Stopping,PetriState::Stopped) => true,
            (PetriState::Stopped,PetriState::Destroyed) => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum SandboxError {
    InvalidStateTransition,
    SandboxNotFound,
    DirectoryCreationFailed,
    ProcessLaunchFailed(std::io::Error),
    ProcessStopFailed,
    PermissionNotFound(PetriPermissions),
    IsolationFailed,
    InvalidConfiguration(String),
}

impl From<std::io::Error> for SandboxError {
    fn from(error:std::io::Error) -> Self {
        SandboxError::ProcessLaunchFailed(error)
    }
}

#[derive(Debug)]
pub enum Isolated {
    NotIsolated,
    Isolating,
    Isolated,
}

impl Isolated {
    pub fn can_isolate(&self,state:&PetriState,next:&Isolated) -> bool {
        match (self,state,next) {
            (Isolated::NotIsolated,PetriState::Running,Isolated::Isolating) => true,
            (Isolated::NotIsolated,PetriState::Stopped,Isolated::Isolating) => true,
            _ => false,
        }
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PetriPermissions {
    ReadFiles,
    WriteFiles,
    ExecutePrograms,
    NetworkAccess,
}