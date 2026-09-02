use std::fs;
use std::path::PathBuf;

pub struct PetriStorage {
    root:PathBuf,
}

impl PetriStorage {
    pub fn new() -> Self {
        Self {
            root: PathBuf::from("sandboxes"),
        }
    }

    pub fn create_sandbox(&self,id:u32) -> std::io::Result<PathBuf> {
        let sandbox_path = self.root.join(format!("sandbox-{}",id));
        fs::create_dir_all(&sandbox_path)?;
        fs::create_dir_all(sandbox_path.join("files"))?;
        fs::create_dir_all(sandbox_path.join("logs"))?;
        Ok(sandbox_path)
    }
}
