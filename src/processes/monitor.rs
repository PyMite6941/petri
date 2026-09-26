// Risk detection. The rules live here; the response lives on PetriSandbox
// (handle_risk), so a risk is contained whether or not the UI is running.

use std::process::Command;

// ---------------------------------------------------------------- severity

// How bad is it, and what does Petri do about it?
//
//   Info     - worth recording, nothing else. Normal operation.
//   Warning  - suspicious, but not conclusive. Logged and surfaced, but the
//              sandbox keeps running. A tool that isolates on every twitch
//              gets ignored, and an ignored alarm is worse than none.
//   Critical - contain now, ask questions after. Isolates automatically.
//
// The response belongs to the severity, not to each call site, so there is one
// place to change what "critical" means.
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl Severity {
    // the whole reason the levels exist: only Critical stops the sandbox
    pub fn requires_isolation(&self) -> bool {
        matches!(self,Severity::Critical)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Severity::Info => "INFO",
            Severity::Warning => "WARNING",
            Severity::Critical => "CRITICAL",
        }
    }
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

// ------------------------------------------------------------------ stats

// one sample of what a container is doing right now
#[derive(Debug,Clone,Copy,Default)]
pub struct ContainerStats {
    pub cpu_percent:f32,
    pub memory_percent:f32,
    pub pids:u32,
    pub net_rx_bytes:f64,
}

impl ContainerStats {
    // docker stats --no-stream prints one sample and exits. the streaming form
    // would need its own thread, so the one-shot version is what gets polled.
    pub fn sample(container_name:&str) -> Option<Self> {
        let output = Command::new("docker")
            .args([
                "stats","--no-stream",
                "--format","{{.CPUPerc}}|{{.MemPerc}}|{{.PIDs}}|{{.NetIO}}",
                container_name,
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let fields:Vec<&str> = text.trim().split('|').collect();
        if fields.len() < 4 {
            return None;
        }
        Some(Self {
            cpu_percent: parse_percent(fields[0]),
            memory_percent: parse_percent(fields[1]),
            pids: fields[2].trim().parse().unwrap_or(0),
            // NetIO looks like "2.05kB / 126B"; the first half is received
            net_rx_bytes: parse_size(fields[3].split('/').next().unwrap_or("0")),
        })
    }
}

// "12.34%" -> 12.34
fn parse_percent(text:&str) -> f32 {
    text.trim().trim_end_matches('%').parse().unwrap_or(0.0)
}

// "2.05kB" -> 2050.0. docker uses decimal units here, not binary.
fn parse_size(text:&str) -> f64 {
    let text = text.trim();
    let split = text.find(|c:char| c.is_alphabetic()).unwrap_or(text.len());
    let (number,unit) = text.split_at(split);
    let value:f64 = number.trim().parse().unwrap_or(0.0);
    let multiplier = match unit.trim().to_ascii_lowercase().as_str() {
        "b" => 1.0,
        "kb" => 1_000.0,
        "mb" => 1_000_000.0,
        "gb" => 1_000_000_000.0,
        "kib" => 1_024.0,
        "mib" => 1_048_576.0,
        "gib" => 1_073_741_824.0,
        _ => 1.0,
    };
    value * multiplier
}

// ---------------------------------------------------------------- monitor

// thresholds. warning says look at this; critical says stop it.
const CPU_WARNING:f32 = 60.0;
const CPU_CRITICAL:f32 = 90.0;
const MEMORY_WARNING:f32 = 75.0;
const MEMORY_CRITICAL:f32 = 90.0;
// --pids-limit is 128, so these are half and three quarters of the ceiling
const PIDS_WARNING:u32 = 64;
const PIDS_CRITICAL:u32 = 96;
// a sandbox created with the network off should move no bytes at all
const QUIET_NETWORK_BYTES:f64 = 1_024.0;

pub struct Monitor {
    pub cpu_warning:f32,
    pub cpu_critical:f32,
    pub memory_warning:f32,
    pub memory_critical:f32,
    // set from the UI to prove the risk -> isolate -> log path end to end
    // without waiting for a real rule to fire
    pub force_trigger:bool,
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            cpu_warning: CPU_WARNING,
            cpu_critical: CPU_CRITICAL,
            memory_warning: MEMORY_WARNING,
            memory_critical: MEMORY_CRITICAL,
            force_trigger: false,
        }
    }

    // None is the normal case - most checks find nothing.
    //
    // network_expected says whether this sandbox was granted the network. a
    // sandbox that was built with --network none but is moving bytes is not a
    // noisy neighbour, it is a containment failure, so that one is critical
    // regardless of volume.
    pub fn check(&mut self,container_name:&str,network_expected:bool) -> Option<SecurityEvent> {
        if self.force_trigger {
            self.force_trigger = false;
            return Some(SecurityEvent::new(
                Severity::Critical,
                "manually triggered test risk",
            ));
        }

        let stats = ContainerStats::sample(container_name)?;
        self.evaluate(&stats,network_expected)
    }

    // pure rule evaluation, split out so it can be tested without docker.
    // returns the most severe finding - one alarm is easier to act on than six.
    pub fn evaluate(&self,stats:&ContainerStats,network_expected:bool) -> Option<SecurityEvent> {
        let mut findings:Vec<SecurityEvent> = Vec::new();

        if !network_expected && stats.net_rx_bytes > QUIET_NETWORK_BYTES {
            findings.push(SecurityEvent::new(
                Severity::Critical,
                format!(
                    "network traffic ({:.0} bytes in) on a sandbox with no network - containment may have failed",
                    stats.net_rx_bytes,
                ),
            ));
        }

        if stats.cpu_percent >= self.cpu_critical {
            findings.push(SecurityEvent::new(
                Severity::Critical,
                format!("cpu at {:.1}% (limit {:.1}%)",stats.cpu_percent,self.cpu_critical),
            ));
        } else if stats.cpu_percent >= self.cpu_warning {
            findings.push(SecurityEvent::new(
                Severity::Warning,
                format!("cpu at {:.1}%",stats.cpu_percent),
            ));
        }

        if stats.memory_percent >= self.memory_critical {
            findings.push(SecurityEvent::new(
                Severity::Critical,
                format!(
                    "memory at {:.1}% of its limit - the container is about to be killed",
                    stats.memory_percent,
                ),
            ));
        } else if stats.memory_percent >= self.memory_warning {
            findings.push(SecurityEvent::new(
                Severity::Warning,
                format!("memory at {:.1}%",stats.memory_percent),
            ));
        }

        if stats.pids >= PIDS_CRITICAL {
            findings.push(SecurityEvent::new(
                Severity::Critical,
                format!("{} processes - close to the limit, looks like a fork bomb",stats.pids),
            ));
        } else if stats.pids >= PIDS_WARNING {
            findings.push(SecurityEvent::new(
                Severity::Warning,
                format!("{} processes and climbing",stats.pids),
            ));
        }

        findings.into_iter().max_by_key(|event| event.severity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(cpu:f32,memory:f32,pids:u32,net:f64) -> ContainerStats {
        ContainerStats { cpu_percent: cpu, memory_percent: memory, pids, net_rx_bytes: net }
    }

    #[test]
    fn idle_container_is_quiet() {
        let monitor = Monitor::new();
        assert!(monitor.evaluate(&stats(0.5,1.0,1,0.0),false).is_none());
    }

    #[test]
    fn busy_cpu_is_critical() {
        let monitor = Monitor::new();
        let event = monitor.evaluate(&stats(99.0,1.0,1,0.0),false).unwrap();
        assert_eq!(event.severity,Severity::Critical);
    }

    #[test]
    fn moderate_cpu_is_only_a_warning() {
        let monitor = Monitor::new();
        let event = monitor.evaluate(&stats(70.0,1.0,1,0.0),false).unwrap();
        assert_eq!(event.severity,Severity::Warning);
        assert!(!event.severity.requires_isolation());
    }

    #[test]
    fn traffic_without_network_is_a_containment_failure() {
        let monitor = Monitor::new();
        let event = monitor.evaluate(&stats(1.0,1.0,1,50_000.0),false).unwrap();
        assert_eq!(event.severity,Severity::Critical);
        assert!(event.message.contains("containment"));
    }

    #[test]
    fn traffic_is_fine_when_the_network_was_granted() {
        let monitor = Monitor::new();
        assert!(monitor.evaluate(&stats(1.0,1.0,1,50_000.0),true).is_none());
    }

    #[test]
    fn many_processes_looks_like_a_fork_bomb() {
        let monitor = Monitor::new();
        let event = monitor.evaluate(&stats(1.0,1.0,100,0.0),false).unwrap();
        assert_eq!(event.severity,Severity::Critical);
    }

    #[test]
    fn the_worst_finding_wins() {
        let monitor = Monitor::new();
        // a warning and a critical at once - critical is what gets reported
        let event = monitor.evaluate(&stats(70.0,95.0,1,0.0),false).unwrap();
        assert_eq!(event.severity,Severity::Critical);
    }

    #[test]
    fn only_critical_isolates() {
        assert!(Severity::Critical.requires_isolation());
        assert!(!Severity::Warning.requires_isolation());
        assert!(!Severity::Info.requires_isolation());
    }

    #[test]
    fn sizes_parse() {
        assert_eq!(super::parse_size("2.05kB"),2050.0);
        assert_eq!(super::parse_size("126B"),126.0);
        assert_eq!(super::parse_size("1MiB"),1_048_576.0);
    }

    #[test]
    fn percents_parse() {
        assert_eq!(super::parse_percent("12.34%"),12.34);
        assert_eq!(super::parse_percent("0.00%"),0.0);
    }
}
