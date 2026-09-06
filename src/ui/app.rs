use eframe::egui;
use core::error;
use std::{fmt, string};
use crate::sandbox::{sandbox::PetriSandbox, state::SandboxError};

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

impl fmt::Display for SandboxError {
    fn fmt(&self,f:&mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxError::InvalidStateTransition => {
                write!(f,"Invalid sandbox state transition")
            }
            SandboxError::ProcessLaunchFailed(error) => {
                write!(f,"Failed to launch process: {}",error)
            }
            SandboxError::SandboxNotFound => {
                write!(f,"Sandbox not found")
            }
            SandboxError::DirectoryCreationFailed => {
                write!(f,"Failed to create sandbox directory")
            }
            SandboxError::IsolationFailed => {
                write!(f,"Failed to isolate sandbox")
            }
            SandboxError::InvalidConfiguration(error) => {
                write!(f,"Invalid Configuration: {}",error)
            }
            SandboxError::ProcessStopFailed => {
                write!(f,"Failed to stop process")
            }
            SandboxError::PermissionNotFound(permission) => {
                write!(f,"Invalid Permission: {}",permission)
            }
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
