// Risk detection. The rules live here; the response lives on PetriSandbox
// (handle_risk), so a risk is contained whether or not the UI is running.

use std::process::Command;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug,Clone)]
pub struct SecurityEvent {
    pub message:String,
    pub severity:Severity,
}

impl SecurityEvent {
    pub fn new(severity:Severity,message:impl Into<String>) -> Self {
        Self { severity, message: message.into() }
    }
}

// a sandbox burning this much CPU is doing something worth looking at
const CPU_CRITICAL:f32 = 90.0;

pub struct Monitor {
    pub cpu_limit:f32,
    // set from the UI to prove the risk -> isolate -> log path end to end
    // without waiting for a real rule to fire
    pub force_trigger:bool,
}

impl Monitor {
    pub fn new() -> Self {
        Self { cpu_limit: CPU_CRITICAL, force_trigger: false }
    }

    // None is the normal case - most checks find nothing.
    pub fn check(&mut self,container_name:&str) -> Option<SecurityEvent> {
        if self.force_trigger {
            self.force_trigger = false;
            return Some(SecurityEvent::new(
                Severity::Critical,
                "manually triggered test risk",
            ));
        }

        match Self::cpu_percent(container_name) {
            Some(cpu) if cpu >= self.cpu_limit => Some(SecurityEvent::new(
                Severity::Critical,
                format!("cpu at {:.1}% (limit {:.1}%)",cpu,self.cpu_limit),
            )),
            _ => None,
        }
    }

    // docker stats --no-stream prints one sample and exits. the streaming form
    // would need its own thread, so the one-shot version is what gets polled.
    fn cpu_percent(container_name:&str) -> Option<f32> {
        let output = Command::new("docker")
            .args(["stats","--no-stream","--format","{{.CPUPerc}}",container_name])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        // the value comes back like "12.34%"
        text.trim().trim_end_matches('%').parse::<f32>().ok()
    }
}
