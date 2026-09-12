use std::process::Child;

#[derive(Debug)]
pub enum PetriProcess {
    None,
    Host(Child),
    Container {id:String},
}