use serde::{Serialize,Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetriState {
    ReadyToCreate,
    Created,
    Starting,
    Running,
    Stopping,
    Stopped,
    Ran,
    Destroyed,
}

impl PetriState {
    pub fn can_transition(&self, next: &PetriState) -> bool {
        match (self,next) {
            (PetriState::ReadyToCreate,PetriState::Created) => true,
            (PetriState::ReadyToCreate,PetriState::Destroyed) => true,
            (PetriState::Created,PetriState::Starting) => true,
            (PetriState::Created,PetriState::Destroyed) => true,
            (PetriState::Starting,PetriState::Stopped) => true,
            (PetriState::Starting,PetriState::Running) => true,
            (PetriState::Running,PetriState::Stopping) => true,
            (PetriState::Running,PetriState::Destroyed) => true,
            (PetriState::Stopping,PetriState::Stopped) => true,
            (PetriState::Stopped,PetriState::Destroyed) => true,
            (PetriState::Stopped,PetriState::Ran) => true,
            (PetriState::Ran,PetriState::Destroyed) => true,
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
    PermissionNotFound,
    IsolationFailed,
    InvalidConfiguration(String),
    StorageFailed(std::io::Error),
    ConfigSerializeFailed(toml::ser::Error),
    ConfigParseFailed(toml::de::Error),
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
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

#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
pub enum PetriPermissions {
    ReadFiles,
    WriteFiles,
    ExecutePrograms,
    NetworkAccess,
}