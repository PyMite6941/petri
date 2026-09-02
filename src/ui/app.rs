use eframe::egui;
use crate::sandbox::sandbox::PetriSandbox;

pub struct PetriApp {
    sandbox: Option<PetriSandbox>,
    sandbox_name:String,
}

impl PetriApp {
    pub fn new() -> Self {
        Self {
            sandbox: PetriSandbox::new_sandbox(
                1,
                String::from("Test Sandbox"),
            ),
            sandbox_name: String::new(),
        }
    }
}

impl eframe::App for PetriApp {
    fn update(&mut self,ctx:&egui::Context,_frame:&mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Petri Sandbox Manager");
            ui.separator();
            ui.label("Create a Sandbox");
            ui.text_edit_singleline(&mut self.sandbox_name);
            ui.separator();
            ui.label("Sandbox:");
            ui.label(format!("ID: {}", self.sandbox.id()));
            if self.sandbox.name().is_empty() {
                self.sandbox.set_name(String::from("petri-sandbox"));
            }
            ui.label(format!("Name: {}", self.sandbox.name()));
            ui.separator();
            ui.label(format!("State: {:?}",self.sandbox.state()));
            ui.separator();
            if ui.button("Start Sandbox").clicked() {
                self.sandbox.start_sandbox();
            }
            if ui.button("Isolate Sandbox").clicked() {
                self.sandbox.isolate_sandbox();
            }
            if ui.button("Destroy Sandbox").clicked() {
                self.sandbox.destroy_sandbox();
            }
        });
    }
}
