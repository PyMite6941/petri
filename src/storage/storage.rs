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
}
