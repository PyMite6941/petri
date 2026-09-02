mod sandbox;
mod storage;
mod ui;

use crate::ui::app::PetriApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Petri Sandbox Manager",
        options,
        Box::new(|_creation_context| {
            Ok(Box::new(PetriApp::new()))
        }),
    )
}
