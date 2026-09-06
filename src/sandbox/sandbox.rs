use super::state::Isolated;
use super::state::PetriState;
use super::state::SandboxError;
use super::state::PetriPermissions;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command,Child};

#[derive(Debug)]
pub struct PetriSandbox {
    pub id:u64,
    pub name:String,
    pub state:PetriState,
    pub isolate:Isolated,
    pub directory:PathBuf,
    pub permissions:vec![],
    pub process:Option<Child>,
}

impl PetriSandbox {
    pub fn new_sandbox(id:u64,name:String) -> Self {
        Self {
            id,
            name,
            state: PetriState::Created,
            isolate:Isolated::NotIsolated,
            directory: PathBuf::from(format!("sandboxes/sandbox-{}",id)),
            permissions: vec![],
            process: None,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn directory(&self) -> &PathBuf {
        &self.directory
    }

    pub fn set_name(&mut self,name:String) {
        self.name = name;
    }

    pub fn start_sandbox(&mut self) -> Result<(), SandboxError> {
        if !self.state.can_transition(&PetriState::Starting) {
            return Err(SandboxError::InvalidStateTransition);
        }
        self.state = PetriState::Starting;
        let child = Command::new("some-program").spawn()?;
        self.process = Some(child);
        self.state = PetriState::Running;
        Ok(())
    }

    pub fn add_permissions(&mut self,permission:PetriPermissions) -> Result<(), SandboxError> {
        self.permissions.push(permission);
        Ok(())
    }

    pub fn remove_permissions(&mut self,permission:PetriPermissions) -> Result<(), SandboxError> {
        if let Some(index) = self.pernissions.iter().position(|p| *p == permission) {
            self.permissions.remove(index);
            Ok(())
        } else {
            Err(SandboxError::PermissionNotFound(permission))
        }
    }

    pub fn isolate_sandbox(&mut self) -> Result<(), SandboxError> {
        if !self.isolate.can_isolate(&self.state,&Isolated::Isolating) {
            return Err(SandboxError::InvalidStateTransition);
        }
        self.isolate = Isolated::Isolating;

        self.isolate = Isolated::Isolated;
        Ok(())
    }

    pub fn destroy_sandbox(&mut self) -> Result<(), SandboxError> {
        if !self.state.can_transition(&PetriState::Destroyed) {
            return Err(SandboxError::InvalidStateTransition);
        }
        self.state = PetriState::Destroyed;
        Ok(())
    }

    pub fn state(&self) -> PetriState {
        self.state
    }
}
