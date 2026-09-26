use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::{Child,Command};
use crate::sandbox::sandbox::SandboxConfig;
use crate::sandbox::state::{PetriPermissions,SandboxError};

// every petri container is named petri-<sandbox id> so a container can always
// be found again from the sandbox, even after a restart.
const CONTAINER_PREFIX:&str = "petri-";

// the container workdir that the sandbox files/ directory is mounted onto
const WORKDIR:&str = "/work";

// Split a program string into a binary and its arguments the way a shell
// would. Plain split_whitespace tears quoted arguments apart, so
//   sh -c "while true; do :; done"
// became seven broken tokens and the container died on a syntax error.
// Quotes group, and are not kept in the token.
fn split_program(program:&str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote:Option<char> = None;
    let mut started = false;

    for character in program.chars() {
        match quote {
            Some(open) if character == open => {
                quote = None;
            }
            Some(_) => current.push(character),
            None if character == '\'' || character == '"' => {
                quote = Some(character);
                // an empty quoted string is still an argument
                started = true;
            }
            None if character.is_whitespace() => {
                if started || !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            None => current.push(character),
        }
    }
    if started || !current.is_empty() {
        parts.push(current);
    }
    parts
}

#[derive(Debug)]
pub enum PetriProcess {
    None,
    Host(Child),
    Container {id:String},
}

// run a docker subcommand to completion and hand back its stdout.
// docker writes the failure reason to stderr, so that becomes the error text.
fn docker(args:&[&str]) -> Result<String,SandboxError> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .map_err(SandboxError::ProcessLaunchFailed)?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(SandboxError::InvalidConfiguration(format!("docker {}: {}",args[0],message)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

impl PetriProcess {
    pub fn container_name(sandbox_id:u64) -> String {
        format!("{}{}",CONTAINER_PREFIX,sandbox_id)
    }

    // is the docker daemon reachable? checked once at startup so a missing
    // daemon gives a clear message instead of a confusing spawn failure.
    pub fn docker_available() -> bool {
        Command::new("docker")
            .args(["info","--format","{{.ServerVersion}}"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    // build the container without starting it. this runs when the sandbox is
    // created, so the container exists for the whole life of the sandbox.
    pub fn create_container(
        sandbox_id:u64,
        config:&SandboxConfig,
        directory:&Path,
    ) -> Result<Self,SandboxError> {
        let name = Self::container_name(sandbox_id);

        // docker rejects relative bind mounts, and directory is relative
        let files = directory.join("files");
        let absolute = std::fs::canonicalize(&files).map_err(SandboxError::StorageFailed)?;
        let mount = format!("{}:{}",absolute.display(),WORKDIR);

        // --cap-drop ALL takes CAP_DAC_OVERRIDE away, so container root can no
        // longer ignore file permissions. run as whoever owns the mount instead
        // - the container then writes to files/ as an ordinary user, and is not
        // root inside the container either.
        let owner = std::fs::metadata(&absolute).map_err(SandboxError::StorageFailed)?;
        let user = format!("{}:{}",owner.uid(),owner.gid());

        // a stale container from a previous run would make create fail on the
        // name conflict, so clear it first. failure here is fine - it usually
        // just means there was nothing to remove.
        let _ = docker(&["rm","-f",&name]);

        let mut args:Vec<String> = vec![
            "create".into(),
            "--name".into(), name.clone(),
            "--user".into(), user,
            "--cap-drop".into(), "ALL".into(),
            "--security-opt".into(), "no-new-privileges".into(),
            "--memory".into(), "256m".into(),
            "--pids-limit".into(), "128".into(),
            "-v".into(), mount,
            "-w".into(), WORKDIR.into(),
        ];

        // the sandbox only reaches the network if it was granted it
        let networked = config.network_enabled
            && config.permissions.contains(&PetriPermissions::NetworkAccess);
        if !networked {
            args.push("--network".into());
            args.push("none".into());
        }

        // without WriteFiles the whole root filesystem is immutable. the bind
        // mount stays writable either way - that is the point of the mount.
        if !config.permissions.contains(&PetriPermissions::WriteFiles) {
            args.push("--read-only".into());
        }

        args.push(config.image.clone());

        // Command::new takes the executable only, so the program string has to
        // be split the same way a shell would split it.
        for part in split_program(&config.program) {
            args.push(part);
        }

        let borrowed:Vec<&str> = args.iter().map(|a| a.as_str()).collect();
        let id = docker(&borrowed)?;
        Ok(PetriProcess::Container { id })
    }

    pub fn start(&mut self,sandbox_id:u64) -> Result<(),SandboxError> {
        match self {
            PetriProcess::Container { .. } => {
                docker(&["start",&Self::container_name(sandbox_id)])?;
                Ok(())
            }
            _ => Err(SandboxError::InvalidConfiguration("no container to start".into())),
        }
    }

    // Ok(None) when there is nothing to report on
    pub fn is_running(&mut self,sandbox_id:u64) -> bool {
        match self {
            PetriProcess::None => false,
            PetriProcess::Host(child) => matches!(child.try_wait(),Ok(None)),
            PetriProcess::Container { .. } => {
                let name = Self::container_name(sandbox_id);
                docker(&["inspect","-f","{{.State.Running}}",&name])
                    .map(|out| out == "true")
                    .unwrap_or(false)
            }
        }
    }

    pub fn status(&self,sandbox_id:u64) -> Option<String> {
        match self {
            PetriProcess::Container { .. } => {
                docker(&["inspect","-f","{{.State.Status}}",&Self::container_name(sandbox_id)]).ok()
            }
            _ => None,
        }
    }

    // graceful stop: SIGTERM, then SIGKILL after the timeout
    pub fn stop(&mut self,sandbox_id:u64) -> Result<(),SandboxError> {
        match self {
            PetriProcess::Container { .. } => {
                docker(&["stop","-t","5",&Self::container_name(sandbox_id)])?;
                Ok(())
            }
            PetriProcess::Host(child) => {
                child.kill().map_err(|_| SandboxError::ProcessStopFailed)?;
                child.wait().map_err(|_| SandboxError::ProcessStopFailed)?;
                Ok(())
            }
            PetriProcess::None => Ok(()),
        }
    }

    // isolation. the freezer cgroup suspends every process in the container
    // mid-syscall, so there is no window to race.
    pub fn pause(&mut self,sandbox_id:u64) -> Result<(),SandboxError> {
        match self {
            PetriProcess::Container { .. } => {
                docker(&["pause",&Self::container_name(sandbox_id)])
                    .map_err(|_| SandboxError::IsolationFailed)?;
                Ok(())
            }
            _ => Err(SandboxError::IsolationFailed),
        }
    }

    pub fn unpause(&mut self,sandbox_id:u64) -> Result<(),SandboxError> {
        match self {
            PetriProcess::Container { .. } => {
                docker(&["unpause",&Self::container_name(sandbox_id)])
                    .map_err(|_| SandboxError::IsolationFailed)?;
                Ok(())
            }
            _ => Err(SandboxError::IsolationFailed),
        }
    }

    // remove the container entirely. -f stops it first if it is still running.
    pub fn remove(&mut self,sandbox_id:u64) -> Result<(),SandboxError> {
        if let PetriProcess::Container { .. } = self {
            docker(&["rm","-f",&Self::container_name(sandbox_id)])?;
        }
        *self = PetriProcess::None;
        Ok(())
    }

    pub fn describe(&self) -> String {
        match self {
            PetriProcess::None => "none".to_string(),
            PetriProcess::Host(child) => format!("pid {}",child.id()),
            PetriProcess::Container { id } => {
                // docker ids are long; the short form is what the CLI shows
                let short:String = id.chars().take(12).collect();
                format!("container {}",short)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::split_program;

    #[test]
    fn plain_words() {
        assert_eq!(split_program("sleep 30"),vec!["sleep","30"]);
    }

    #[test]
    fn double_quotes_group() {
        assert_eq!(
            split_program("sh -c \"while true; do :; done\""),
            vec!["sh","-c","while true; do :; done"],
        );
    }

    #[test]
    fn single_quotes_group() {
        assert_eq!(
            split_program("echo 'hello world'"),
            vec!["echo","hello world"],
        );
    }

    #[test]
    fn extra_whitespace_is_ignored() {
        assert_eq!(split_program("  ls   -la  "),vec!["ls","-la"]);
    }

    #[test]
    fn empty_is_empty() {
        assert!(split_program("").is_empty());
    }
}
