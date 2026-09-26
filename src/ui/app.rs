use eframe::egui;
use std::collections::HashMap;
use std::time::{Duration,Instant};
use crate::processes::monitor::{Monitor,Severity};
use crate::processes::processes::PetriProcess;
use crate::sandbox::sandbox::{PetriSandbox,SandboxConfig};
use crate::sandbox::state::{Isolated,PetriPermissions,PetriState,SandboxError};

// every permission the picker offers
const ALL_PERMISSIONS:[PetriPermissions;4] = [
    PetriPermissions::ReadFiles,
    PetriPermissions::WriteFiles,
    PetriPermissions::ExecutePrograms,
    PetriPermissions::NetworkAccess,
];

// keep a card's log from growing forever
const MAX_LOG_LINES:usize = 200;

// container image used when the create form leaves Image blank
const DEFAULT_IMAGE:&str = "alpine";

// how often to ask docker whether each running container is still alive.
// every poll spawns a docker subprocess, so this must not run per frame.
const POLL_INTERVAL:Duration = Duration::from_secs(1);

fn permission_label(permission:PetriPermissions) -> &'static str {
    match permission {
        PetriPermissions::ReadFiles => "Read files",
        PetriPermissions::WriteFiles => "Write files",
        PetriPermissions::ExecutePrograms => "Execute programs",
        PetriPermissions::NetworkAccess => "Network access",
    }
}

// dropdown (multi-select) + removable chips.
// returns what the user asked to add / remove so the caller owns the mutation.
fn permission_picker(
    ui:&mut egui::Ui,
    granted:&[PetriPermissions],
) -> (Option<PetriPermissions>,Option<PetriPermissions>) {
    let mut to_add:Option<PetriPermissions> = None;
    let mut to_remove:Option<PetriPermissions> = None;

    let summary = match granted.len() {
        0 => "Select permissions".to_string(),
        1 => "1 permission".to_string(),
        count => format!("{} permissions",count),
    };

    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("permission_picker")
            .selected_text(summary)
            .width(180.0)
            .show_ui(ui, |ui| {
                for permission in ALL_PERMISSIONS {
                    let mut selected = granted.contains(&permission);
                    if ui.checkbox(&mut selected,permission_label(permission)).changed() {
                        if selected {
                            to_add = Some(permission);
                        } else {
                            to_remove = Some(permission);
                        }
                    }
                }
            });

        if granted.is_empty() {
            ui.weak("none granted");
        }
        for permission in granted {
            let permission = *permission;
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(permission_label(permission));
                    if ui.small_button("x").on_hover_text("Remove").clicked() {
                        to_remove = Some(permission);
                    }
                });
            });
        }
    });

    (to_add,to_remove)
}

// an empty config, used only to build the throwaway sandbox that load_config
// reads through. immediately replaced by the config read off disk.
fn blank_config() -> SandboxConfig {
    SandboxConfig {
        name: String::new(),
        program: String::new(),
        image: String::new(),
        network_enabled: false,
        permissions: Vec::new(),
    }
}

pub struct PetriApp {
    sandboxes:Vec<PetriSandbox>,
    next_id:u64,
    // per-sandbox activity log, keyed by sandbox id
    logs:HashMap<u64,Vec<String>>,
    // the "+" form is hidden until asked for
    show_create_form:bool,
    form_name:String,
    form_program:String,
    form_image:String,
    form_network:bool,
    form_permissions:Vec<PetriPermissions>,
    // id -> in-progress rename text. present only while that card is being renamed.
    renaming:HashMap<u64,String>,
    last_poll:Instant,
    docker_ok:bool,
    monitor:Monitor,
    // most recent finding per sandbox, so the card can show it
    last_severity:HashMap<u64,Severity>,
}

impl PetriApp {
    pub fn new() -> Self {
        let mut app = Self {
            sandboxes: Vec::new(),
            next_id: 1,
            logs: HashMap::new(),
            show_create_form: false,
            form_name: String::new(),
            form_program: String::new(),
            form_image: DEFAULT_IMAGE.to_string(),
            form_network: false,
            form_permissions: Vec::new(),
            renaming: HashMap::new(),
            last_poll: Instant::now(),
            docker_ok: PetriProcess::docker_available(),
            monitor: Monitor::new(),
            last_severity: HashMap::new(),
        };
        app.restore_sandboxes();
        app
    }

    // ask docker which containers are actually still running. a sandbox whose
    // program ended by itself is no longer Running, and the card has to say so.
    fn poll_running(&mut self) {
        for index in 0..self.sandboxes.len() {
            if self.sandboxes[index].state() != PetriState::Running {
                continue;
            }
            let id = self.sandboxes[index].id();
            if !self.sandboxes[index].process.is_running(id) {
                let result = self.sandboxes[index].mark_ran();
                self.report(id,"program finished",result);
                continue;
            }

            // still running - ask the monitor whether it looks dangerous
            let name = PetriProcess::container_name(id);
            let networked = self.sandboxes[index].config.network_enabled;
            if let Some(event) = self.monitor.check(&name,networked) {
                let message = event.message.clone();
                let severity = event.severity;
                let isolating = severity.requires_isolation();
                let result = self.sandboxes[index].handle_risk(event);
                self.log(id,format!("{} {}",severity.label(),message));
                // only a critical finding stops the sandbox, so only report
                // the isolation attempt when one was actually made
                if isolating {
                    self.report(id,"isolate on risk",result);
                } else if let Err(error) = result {
                    self.log(id,format!("risk handling failed: {}",error));
                }
                self.last_severity.insert(id,severity);
            }
        }
    }

    // read sandboxes/ back off disk on launch. a sandbox that was running
    // before is gone, so everything comes back as a built, idle sandbox.
    fn restore_sandboxes(&mut self) {
        let ids = match PetriSandbox::parse_sandboxes() {
            Ok(ids) => ids,
            // no sandboxes/ directory yet is the normal first-run case
            Err(_) => return,
        };

        let mut sorted = ids;
        sorted.sort();

        for id in sorted {
            // load_config needs a sandbox to read through, and the directory
            // is derived from the id, so build a throwaway to read with first.
            let probe = PetriSandbox::new_sandbox(id,String::new(),blank_config());
            let config = match probe.load_config() {
                Ok(config) => config,
                Err(_) => continue,
            };

            let mut sandbox = PetriSandbox::new_sandbox(id,config.name.clone(),config);
            // the directory already exists; create_dir_all is idempotent and
            // this moves it out of ReadyToCreate into Created.
            let _ = sandbox.create_sandbox();
            self.sandboxes.push(sandbox);
            self.log(id,"restored from disk".to_string());
        }

        // never hand out an id that already owns a directory
        self.next_id = self.sandboxes.iter().map(|s| s.id()).max().unwrap_or(0) + 1;
    }

    fn log(&mut self,id:u64,line:String) {
        // mirror every line into logs/events.log so history outlives the app.
        // best effort - a sandbox with no directory yet has nowhere to write.
        if let Some(sandbox) = self.sandboxes.iter().find(|s| s.id() == id) {
            let _ = sandbox.append_event(&line);
        }
        let entries = self.logs.entry(id).or_insert_with(Vec::new);
        entries.push(line);
        if entries.len() > MAX_LOG_LINES {
            entries.remove(0);
        }
    }

    // one place to turn a backend Result into a log line. the same line goes
    // to logs/events.log so the history outlives this process.
    fn report(&mut self,id:u64,action:&str,result:Result<(),SandboxError>) {
        let line = match result {
            Ok(()) => format!("{} ok",action),
            Err(error) => format!("{} failed: {}",action,error),
        };
        self.log(id,line);
    }

    fn create_sandbox(&mut self) {
        let name = if self.form_name.trim().is_empty() {
            format!("Sandbox-{}",self.next_id)
        } else {
            self.form_name.trim().to_string()
        };
        let image = if self.form_image.trim().is_empty() {
            DEFAULT_IMAGE.to_string()
        } else {
            self.form_image.trim().to_string()
        };
        let config = SandboxConfig {
            name,
            program: self.form_program.trim().to_string(),
            image,
            network_enabled: self.form_network,
            permissions: self.form_permissions.clone(),
        };
        let id = self.next_id;
        // name is passed separately as well as inside the config
        self.sandboxes.push(PetriSandbox::new_sandbox(id,config.name.clone(),config));
        self.next_id += 1;
        self.log(id,"created".to_string());

        self.form_name.clear();
        self.form_program.clear();
        self.form_image = DEFAULT_IMAGE.to_string();
        self.form_network = false;
        self.form_permissions.clear();
        self.show_create_form = false;
    }

    // the "+" panel - builds a SandboxConfig, nothing else
    fn create_form(&mut self,ui:&mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label("New sandbox");
            egui::Grid::new("create_form").num_columns(2).spacing([12.0,6.0]).show(ui, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut self.form_name);
                ui.end_row();

                ui.label("Program");
                ui.text_edit_singleline(&mut self.form_program);
                ui.end_row();

                ui.label("Image");
                ui.text_edit_singleline(&mut self.form_image)
                    .on_hover_text("container image the program runs inside");
                ui.end_row();

                ui.label("Network");
                let selected_text = if self.form_network { "Enabled" } else { "Disabled" };
                egui::ComboBox::from_id_salt("form_network").selected_text(selected_text).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.form_network,false,"Disabled");
                    ui.selectable_value(&mut self.form_network,true,"Enabled");
                });
                ui.end_row();
            });

            ui.label("Permissions");
            let granted = self.form_permissions.clone();
            let (add,remove) = permission_picker(ui,&granted);
            if let Some(permission) = add {
                if !self.form_permissions.contains(&permission) {
                    self.form_permissions.push(permission);
                }
            }
            if let Some(permission) = remove {
                self.form_permissions.retain(|existing| *existing != permission);
            }

            ui.horizontal(|ui| {
                if ui.button("Create").clicked() {
                    self.create_sandbox();
                }
                if ui.button("Cancel").clicked() {
                    self.show_create_form = false;
                }
            });
        });
    }

    // one sandbox card. returns true if the user asked to drop it from the list.
    fn sandbox_card(&mut self,ui:&mut egui::Ui,index:usize) -> bool {
        let id = self.sandboxes[index].id();
        let mut drop_from_list = false;

        // read-only look at the disk: is the directory actually built?
        // this only decides whether a button is offered - it enforces nothing.
        let directory = self.sandboxes[index].directory().clone();
        let built = directory.join("files").is_dir() && directory.join("logs").is_dir();

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                match self.renaming.get(&id).cloned() {
                    Some(mut draft) => {
                        let response = ui.text_edit_singleline(&mut draft);
                        self.renaming.insert(id,draft.clone());
                        let commit = ui.button("Save").clicked()
                            || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                        if commit {
                            let trimmed = draft.trim().to_string();
                            if trimmed.is_empty() {
                                self.log(id,"rename ignored: name cannot be empty".to_string());
                            } else {
                                self.sandboxes[index].set_name(trimmed.clone());
                                self.sandboxes[index].config.name = trimmed.clone();
                                self.log(id,format!("renamed to {}",trimmed));
                            }
                            self.renaming.remove(&id);
                        }
                        if ui.button("Cancel").clicked() {
                            self.renaming.remove(&id);
                        }
                    }
                    None => {
                        ui.strong(self.sandboxes[index].name().to_string());
                        if ui.small_button("Rename").clicked() {
                            let current = self.sandboxes[index].name().to_string();
                            self.renaming.insert(id,current);
                        }
                    }
                }
                ui.separator();
                ui.label(format!("{:?}",self.sandboxes[index].state()));
                ui.separator();
                ui.label(format!("{:?}",self.sandboxes[index].isolate));
                if !built {
                    ui.separator();
                    ui.weak("not built");
                }
                if let Some(severity) = self.last_severity.get(&id).copied() {
                    ui.separator();
                    let colour = match severity {
                        Severity::Info => egui::Color32::from_rgb(120,160,220),
                        Severity::Warning => egui::Color32::from_rgb(220,170,60),
                        Severity::Critical => egui::Color32::from_rgb(220,80,80),
                    };
                    ui.colored_label(colour,severity.label());
                }
            });

            egui::Grid::new("card_details").num_columns(2).spacing([12.0,4.0]).show(ui, |ui| {
                ui.label("Program");
                let program = self.sandboxes[index].config.program.clone();
                ui.label(if program.is_empty() { "(none set)".to_string() } else { program });
                ui.end_row();

                ui.label("Image");
                ui.label(self.sandboxes[index].config.image.clone());
                ui.end_row();

                ui.label("Network");
                ui.label(if self.sandboxes[index].config.network_enabled { "Enabled" } else { "Disabled" });
                ui.end_row();

                ui.label("Process");
                let described = self.sandboxes[index].process.describe();
                match self.sandboxes[index].process.status(id) {
                    Some(status) => ui.label(format!("{} ({})",described,status)),
                    None => ui.label(described),
                };
                ui.end_row();

                ui.label("Directory");
                ui.label(format!("{}",directory.display()));
                ui.end_row();
            });

            ui.label("Edit permissions");
            let granted = self.sandboxes[index].config.permissions.clone();
            let (add,remove) = permission_picker(ui,&granted);
            if let Some(permission) = add {
                let result = self.sandboxes[index].add_permissions(permission);
                self.report(id,&format!("grant {}",permission_label(permission)),result);
            }
            if let Some(permission) = remove {
                let result = self.sandboxes[index].remove_permissions(permission);
                self.report(id,&format!("revoke {}",permission_label(permission)),result);
            }

            // ask the state machine what is legal rather than hardcoding it here.
            // a button only appears when its transition is actually possible.
            let state = self.sandboxes[index].state();
            let can_create = state.can_transition(&PetriState::Created);
            let can_run = state.can_transition(&PetriState::Starting) && built;
            let can_stop = state.can_transition(&PetriState::Stopping);
            // borrow rather than move - Isolated is not Copy
            let can_isolate = self.sandboxes[index].isolate.can_isolate(&state,&Isolated::Isolating);

            ui.horizontal(|ui| {
                if can_create {
                    if ui.button("Create").on_hover_text("Build the sandbox directory on disk").clicked() {
                        let result = self.sandboxes[index].create_sandbox().map(|_| ());
                        let created = result.is_ok();
                        self.report(id,"create",result);
                        // start_sandbox only saves the config when it creates the
                        // directory itself, so creating from here has to save too
                        // or the sandbox never persists.
                        if created {
                            let saved = self.sandboxes[index].save_config();
                            self.report(id,"save config",saved);
                        }
                    }
                }

                // Run needs a legal transition AND the directory actually built
                if can_run {
                    if ui.button("Run").clicked() {
                        let result = self.sandboxes[index].start_sandbox();
                        self.report(id,"run",result);
                    }
                } else if state.can_transition(&PetriState::Starting) && !built {
                    ui.add_enabled(false,egui::Button::new("Run"))
                        .on_disabled_hover_text("sandbox directory is not built yet - press Create");
                }

                if can_stop {
                    if ui.button("Stop").clicked() {
                        let result = self.sandboxes[index].stop_sandbox();
                        self.report(id,"stop",result);
                    }
                }

                if can_isolate {
                    if ui.button("Isolate").clicked() {
                        let result = self.sandboxes[index].isolate_sandbox();
                        self.report(id,"isolate",result);
                    }
                }

                let can_release = self.sandboxes[index]
                    .isolate
                    .can_isolate(&state,&Isolated::NotIsolated);
                if can_release {
                    if ui.button("Continue")
                        .on_hover_text("release the sandbox and let it run again")
                        .clicked()
                    {
                        let result = self.sandboxes[index].release_sandbox();
                        self.report(id,"continue",result);
                    }
                }

                if state == PetriState::Running {
                    if ui.button("Test risk")
                        .on_hover_text("fire a fake risk to prove isolate-on-risk works")
                        .clicked()
                    {
                        self.monitor.force_trigger = true;
                    }
                }

                // destroy is always offered
                if ui.button("Destroy").clicked() {
                    let result = self.sandboxes[index].destroy_sandbox();
                    self.report(id,"destroy",result);
                }

                if ui.button("Open Directory").clicked() {
                    let path = format!("{}",self.sandboxes[index].directory().display());
                    self.log(id,format!("directory: {}",path));
                }

                if state == PetriState::Destroyed {
                    if ui.button("Remove from list").clicked() {
                        drop_from_list = true;
                    }
                }
            });

            ui.label("Logs:");
            egui::ScrollArea::vertical().max_height(110.0).show(ui, |ui| {
                match self.logs.get(&id) {
                    Some(entries) if !entries.is_empty() => {
                        for entry in entries {
                            ui.label(entry);
                        }
                    }
                    _ => {
                        ui.weak("no activity yet");
                    }
                }
            });
        });

        drop_from_list
    }
}

impl eframe::App for PetriApp {
    fn update(&mut self,ctx:&egui::Context,_frame:&mut eframe::Frame) {
        // immediate mode only redraws on input, so ask for a wake-up or a
        // container that exits with the window idle would never be noticed
        if self.last_poll.elapsed() >= POLL_INTERVAL {
            self.last_poll = Instant::now();
            self.poll_running();
        }
        ctx.request_repaint_after(POLL_INTERVAL);

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Petri Sandboxes");
                if !self.docker_ok {
                    ui.colored_label(
                        egui::Color32::from_rgb(220,120,60),
                        "docker unavailable - sandboxes cannot be created",
                    ).on_hover_text("start the docker daemon, then restart Petri");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("[]").on_hover_text("Sandboxes root: sandboxes/").clicked() {
                        self.show_create_form = false;
                    }
                    if ui.button("+").on_hover_text("New sandbox").clicked() {
                        self.show_create_form = !self.show_create_form;
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_create_form {
                self.create_form(ui);
                ui.separator();
            }

            if self.sandboxes.is_empty() {
                ui.weak("No sandboxes yet. Use + to create one.");
                return;
            }

            // every sandbox is shown expanded - they run independently,
            // so none of them has to be closed for another to be used.
            let mut drop_index:Option<usize> = None;
            egui::ScrollArea::vertical().show(ui, |ui| {
                for index in 0..self.sandboxes.len() {
                    let id = self.sandboxes[index].id();
                    let dropped = ui.push_id(id, |ui| self.sandbox_card(ui,index)).inner;
                    if dropped {
                        drop_index = Some(index);
                    }
                    ui.add_space(6.0);
                }
            });

            if let Some(index) = drop_index {
                let id = self.sandboxes[index].id();
                self.sandboxes.remove(index);
                self.logs.remove(&id);
            }
        });
    }
}
