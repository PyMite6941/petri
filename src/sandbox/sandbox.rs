use super::state::Isolated;
use super::state::PetriState;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub struct PetriSandbox {
    pub id:u64,
    pub name:String,
    pub state:PetriState,
    pub isolate:Isolated,
    pub directory:PathBuf,
}

impl PetriSandbox {
    pub fn new_sandbox(id:u64,name:String) -> Self {
        Self {
            id,
            name,
            state: PetriState::Created,
            isolate:Isolated::NotIsolated,
            directory: PathBuf::from(format!("sandboxes/sandbox-{}",id)),
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

    pub fn isolate_sandbox(&mut self) -> Result<(), SandboxError> {
        if !self.isolate.can_isolate(&self.state,Isolated::Isolating) {
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
