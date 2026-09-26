use super::state::{Isolated,PetriState,SandboxError,PetriPermissions};
use crate::processes::processes::PetriProcess;
use serde::{Serialize,Deserialize};
use crate::processes::monitor::SecurityEvent;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime,UNIX_EPOCH};

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
        fs::create_dir_all(path.join("files"))
            .map_err(|_| SandboxError::DirectoryCreationFailed)?;
        fs::create_dir_all(path.join("logs"))
            .map_err(|_| SandboxError::DirectoryCreationFailed)?;
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
        let config_path = self.directory.join("config.toml");
        if !config_path.exists() {
            return Err(SandboxError::SandboxNotFound);
        }
        let text = fs::read_to_string(config_path)?;
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
        // the lifecycle enters Isolated too, so the state alone says the
        // sandbox is frozen and nothing inside it is progressing
        if self.state.can_transition(&PetriState::Isolated) {
            self.state = PetriState::Isolated;
        }
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

    // give a sandbox its network and scheduler back after it was isolated
    pub fn release_sandbox(&mut self) -> Result<(), SandboxError> {
        if !self.isolate.can_isolate(&self.state,&Isolated::NotIsolated) {
            return Err(SandboxError::InvalidStateTransition);
        }
        self.process.unpause(self.id)?;
        self.isolate = Isolated::NotIsolated;
        // back to running - the container was never stopped, only frozen
        if self.state == PetriState::Isolated {
            self.state = PetriState::Running;
        }
        Ok(())
    }

    // append one line to logs/<file>. the logs directory survives destroy, so
    // this is the record that outlives the sandbox itself.
    fn append_log(&self,file:&str,line:&str) -> Result<(), SandboxError> {
        let path = self.directory.join("logs").join(file);
        let mut handle = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(SandboxError::StorageFailed)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        writeln!(handle,"{} {}",stamp,line).map_err(SandboxError::StorageFailed)?;
        Ok(())
    }

    pub fn append_event(&self,line:&str) -> Result<(), SandboxError> {
        self.append_log("events.log",line)
    }

    pub fn append_security(&self,event:&SecurityEvent) -> Result<(), SandboxError> {
        self.append_log("security.log",&format!("[{}] {}",event.severity.label(),event.message))
    }

    // a risk was detected. contain first, record second - this must not depend
    // on the UI being alive to run.
    pub fn handle_risk(&mut self,event:SecurityEvent) -> Result<(), SandboxError> {
        let _ = self.append_security(&event);
        // severity decides the response. a warning is recorded and surfaced but
        // left running - isolating on every twitch trains people to ignore it.
        if !event.severity.requires_isolation() {
            return Ok(());
        }
        if (self.state == PetriState::Running || self.state == PetriState::Isolated)
            && self.isolate == Isolated::NotIsolated
        {
            self.isolate_sandbox()?;
        }
        Ok(())
    }

    pub fn state(&self) -> PetriState {
        self.state
    }
}

impl Drop for PetriSandbox {
    // a container outlives its parent process, unlike a host child, so this is
    // what stops one surviving Petri. drop cannot fail, so errors go nowhere.
    fn drop(&mut self) {
        let _ = self.process.remove(self.id);
    }
}
