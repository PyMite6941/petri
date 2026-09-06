use eframe::egui;
use crate::sandbox::sandbox::PetriSandbox;

pub struct PetriApp {
    sandboxes: Vec<PetriSandbox>,
    sandbox_name:String,
}

impl PetriApp {
    pub fn new() -> Self {
        Self {
            sandboxes: Vec::new(),
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
            for sandbox in &self.sandboxes {
                ui.label("Sandbox:");
                ui.label(format!("ID: {}", sandbox.id()));
                ui.label(format!("Name: {}", sandbox.name()));
                ui.label(format!("State: {:?}",sandbox.state()));
                ui.separator();
                if ui.button("Start Sandbox").clicked() {
                    sandbox.start_sandbox();
                }
                if ui.button("Isolate Sandbox").clicked() {
                    sandbox.isolate_sandbox();
                }
                if ui.button("Destroy Sandbox").clicked() {
                    sandbox.destroy_sandbox();
                }
                ui.horizontal(|ui|{
                    ui.label("Permissions");
                    ui.text_edit_multiline(&mut sandbox.permissions);
                });
            }
        });
    }
}
