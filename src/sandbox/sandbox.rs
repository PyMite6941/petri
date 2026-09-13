use super::state::{Isolated,PetriState,SandboxError,PetriPermissions};
use crate::processes::processes::PetriProcess;
use serde::{Serialize,Deserialize};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};

#[derive(Debug)]
pub struct PetriSandbox {
    pub id:u64,
    pub name:String,
    pub config:SandboxConfig,
    pub state:PetriState,
    pub isolate:Isolated,
    pub directory:PathBuf,
    pub process:PetriProcess,
}

#[derive(Debug,Serialize,Deserialize)]
pub struct SandboxConfig {
    pub name:String,
    pub program:String,
    // the container image the program runs inside
    pub image:String,
    pub network_enabled:bool,
    pub permissions:Vec<PetriPermissions>,
}

impl From<std::io::Error> for SandboxError {
    fn from(error:std::io::Error) -> Self {
        SandboxError::ProcessLaunchFailed(error)
    }
}

impl From<toml::ser::Error> for SandboxError {
    fn from(error:toml::ser::Error) -> Self {
        SandboxError::ConfigSerializeFailed(error)
    }
}

impl From<toml::de::Error> for SandboxError {
    fn from(error:toml::de::Error) -> Self {
        SandboxError::ConfigParseFailed(error)
    }
}

impl PetriSandbox {
    pub fn new_sandbox(id:u64,name:String,config:SandboxConfig) -> Self {
        Self {
            id,
            name,
            config,
            state:PetriState::ReadyToCreate,
            isolate:Isolated::NotIsolated,
            directory:PathBuf::from(format!("sandboxes/sandbox-{}",id)),
            process:PetriProcess::None,
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

    pub fn create_sandbox(&mut self) -> Result<PathBuf, SandboxError> {
        if !self.state.can_transition(&PetriState::Created) {
            return Err(SandboxError::InvalidStateTransition);
        }
        let path = self.directory.clone();
        fs::create_dir_all(path.join("files"))?;
        fs::create_dir_all(path.join("logs"))?;
        // the container is built with the directory, so it exists for the
        // whole life of the sandbox rather than only while it runs
        self.process = PetriProcess::create_container(self.id,&self.config,&self.directory)?;
        self.state = PetriState::Created;
        Ok(path)
    }

    pub fn save_config(&self) -> Result<(),SandboxError> {
        let text = toml::to_string_pretty(&self.config)?;
        fs::write(self.directory.join("config.toml"),text)?;
        Ok(())
    }

    pub fn load_config(&self) -> Result<SandboxConfig,SandboxError> {
        let text = fs::read_to_string(self.directory.join("config.toml"))?;
        let config:SandboxConfig = toml::from_str(&text)?;
        Ok(config)
    }

    pub fn parse_sandboxes() -> Result<Vec<u64>,SandboxError> {
        let entry = fs::read_dir("sandboxes")?;
        let mut ids = Vec::new();
        for item in entry {
            let item = match item {Ok(e)=>e,Err(_)=>continue};
            if let Some(id) = item.file_name().to_str().and_then(|n| n.strip_prefix("sandbox-")).and_then(|n| n.parse::<u64>().ok()) {
                ids.push(id);
            }
        }
        Ok(ids)
    }

    pub fn start_sandbox(&mut self) -> Result<(), SandboxError> {
        if self.state == PetriState::ReadyToCreate {
            self.create_sandbox()?;
            self.save_config()?;
        }
        if !self.state.can_transition(&PetriState::Starting) {
            return Err(SandboxError::InvalidStateTransition);
        }
        self.state = PetriState::Starting;
        self.process.start(self.id)?;
        self.state = PetriState::Running;
        Ok(())
    }

    pub fn stop_sandbox(&mut self) -> Result<(), SandboxError> {
        if !self.state.can_transition(&PetriState::Stopping) {
            return Err(SandboxError::InvalidStateTransition);
        }
        self.state = PetriState::Stopping;
        // the container is kept, only stopped - it is removed on destroy
        self.process.stop(self.id)?;
        self.state = PetriState::Stopped;
        Ok(())
    }

    pub fn add_permissions(&mut self,permission:PetriPermissions) -> Result<(), SandboxError> {
        self.config.permissions.push(permission);
        self.save_config()?;
        Ok(())
    }

    pub fn remove_permissions(&mut self,permission:PetriPermissions) -> Result<(), SandboxError> {
        if let Some(index) = self.config.permissions.iter().position(|p| *p == permission) {
            self.config.permissions.remove(index);
            self.save_config()?;
            Ok(())
        } else {
            Err(SandboxError::PermissionNotFound)
        }
    }

    pub fn isolate_sandbox(&mut self) -> Result<(), SandboxError> {
        if !self.isolate.can_isolate(&self.state,&Isolated::Isolating) {
            return Err(SandboxError::InvalidStateTransition);
        }
        self.isolate = Isolated::Isolating;
        self.process.pause(self.id)?;
        self.isolate = Isolated::Isolated;
        Ok(())
    }

    pub fn destroy_sandbox(&mut self) -> Result<(), SandboxError> {
        if !self.state.can_transition(&PetriState::Destroyed) {
            return Err(SandboxError::InvalidStateTransition);
        }
        // remove the container first, it holds the mount on files/
        self.process.remove(self.id)?;
        fs::remove_dir_all(self.directory.join("files"))?;
        self.state = PetriState::Destroyed;
        Ok(())
    }

    // the program finished on its own - nothing stopped it
    pub fn mark_ran(&mut self) -> Result<(), SandboxError> {
        if !self.state.can_transition(&PetriState::Ran) {
            return Err(SandboxError::InvalidStateTransition);
        }
        self.state = PetriState::Ran;
        Ok(())
    }

    pub fn state(&self) -> PetriState {
        self.state
    }
}
