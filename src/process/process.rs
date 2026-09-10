#[derive(Debug)]
pub enum PetriProcess {
    None,
    Host(Child),
    Container {id:String},
}