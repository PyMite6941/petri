// Risk detection. The rules live here; the response lives on PetriSandbox
// (handle_risk), so a risk is contained whether or not the UI is running.

use std::collections::HashMap;
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

// What earns an automatic isolation, and what does not.
//
// Only evidence that the sandbox is doing something it should not be ABLE to
// do gets isolated. Expensive is not the same as dangerous: a legitimate
// compute task sits at ~98% CPU, and freezing it mid-run means the work never
// finishes. That is a broken sandbox, not a secure one.
//
//   network traffic with no network  -> critical. containment has failed.
//   process count near the ceiling   -> critical, once sustained. fork bomb.
//   cpu                              -> warning only, ever.
//   memory                           -> warning. docker OOM-kills it anyway.
const CPU_WARNING:f32 = 90.0;
const MEMORY_WARNING:f32 = 85.0;
// --pids-limit is 128, so these are half and three quarters of the ceiling
const PIDS_WARNING:u32 = 64;
const PIDS_CRITICAL:u32 = 96;
// a sandbox created with the network off should move no bytes at all
const QUIET_NETWORK_BYTES:f64 = 1_024.0;
// how many consecutive samples a resource breach has to survive before it
// counts. one sample is noise - startup alone can peg a core.
const SUSTAINED_SAMPLES:u32 = 5;

pub struct Monitor {
    pub cpu_warning:f32,
    pub memory_warning:f32,
    // consecutive samples with the process count over the critical mark,
    // per container. a fork bomb keeps climbing; a busy moment does not.
    pids_streak:HashMap<String,u32>,
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
            memory_warning: MEMORY_WARNING,
            pids_streak: HashMap::new(),
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

        // track how long the process count has been over the line
        let streak = self.pids_streak.entry(container_name.to_string()).or_insert(0);
        if stats.pids >= PIDS_CRITICAL {
            *streak += 1;
        } else {
            *streak = 0;
        }
        let pids_sustained = *streak >= SUSTAINED_SAMPLES;

        self.evaluate(&stats,network_expected,pids_sustained)
    }

    // forget a container that is gone, so its streak does not linger
    pub fn forget(&mut self,container_name:&str) {
        self.pids_streak.remove(container_name);
    }

    // pure rule evaluation, split out so it can be tested without docker.
    // returns the most severe finding - one alarm is easier to act on than six.
    pub fn evaluate(
        &self,
        stats:&ContainerStats,
        network_expected:bool,
        pids_sustained:bool,
    ) -> Option<SecurityEvent> {
        let mut findings:Vec<SecurityEvent> = Vec::new();

        if stats.net_rx_bytes > QUIET_NETWORK_BYTES {
            if network_expected {
                // allowed, but a record of what crossed the boundary is the
                // point of having a security log at all
                findings.push(SecurityEvent::new(
                    Severity::Info,
                    format!("{:.0} bytes received over the granted network",stats.net_rx_bytes),
                ));
            } else {
                findings.push(SecurityEvent::new(
                    Severity::Critical,
                    format!(
                        "network traffic ({:.0} bytes in) on a sandbox with no network - containment may have failed",
                        stats.net_rx_bytes,
                    ),
                ));
            }
        }

        // cpu never isolates. a sandbox exists to run work, and work is
        // expensive - pausing it for being busy defeats the whole point.
        if stats.cpu_percent >= self.cpu_warning {
            findings.push(SecurityEvent::new(
                Severity::Warning,
                format!("cpu at {:.1}%",stats.cpu_percent),
            ));
        }

        // memory does not isolate either: the cgroup limit already caps it and
        // docker OOM-kills the container if it goes over. saying so is enough.
        if stats.memory_percent >= self.memory_warning {
            findings.push(SecurityEvent::new(
                Severity::Warning,
                format!("memory at {:.1}% of its limit",stats.memory_percent),
            ));
        }

        if stats.pids >= PIDS_CRITICAL && pids_sustained {
            findings.push(SecurityEvent::new(
                Severity::Critical,
                format!("{} processes, sustained - this is a fork bomb",stats.pids),
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
        assert!(monitor.evaluate(&stats(0.5,1.0,1,0.0),false,false).is_none());
    }

    #[test]
    fn a_busy_task_is_never_isolated() {
        // this is the bug that froze real work: 98% cpu is what compute looks
        // like, not what an attack looks like.
        let monitor = Monitor::new();
        let event = monitor.evaluate(&stats(98.0,1.0,1,0.0),false,false).unwrap();
        assert_eq!(event.severity,Severity::Warning);
        assert!(!event.severity.requires_isolation(),"cpu must never freeze a sandbox");
    }

    #[test]
    fn memory_pressure_warns_but_does_not_isolate() {
        let monitor = Monitor::new();
        let event = monitor.evaluate(&stats(1.0,99.0,1,0.0),false,false).unwrap();
        assert!(!event.severity.requires_isolation());
    }

    #[test]
    fn traffic_without_network_is_a_containment_failure() {
        let monitor = Monitor::new();
        let event = monitor.evaluate(&stats(1.0,1.0,1,50_000.0),false,false).unwrap();
        assert_eq!(event.severity,Severity::Critical);
        assert!(event.message.contains("containment"));
    }

    #[test]
    fn granted_network_traffic_is_recorded_but_not_acted_on() {
        let monitor = Monitor::new();
        let event = monitor.evaluate(&stats(1.0,1.0,1,50_000.0),true,false).unwrap();
        assert_eq!(event.severity,Severity::Info);
        assert!(!event.severity.requires_isolation());
    }

    #[test]
    fn a_process_spike_alone_is_only_a_warning() {
        let monitor = Monitor::new();
        let event = monitor.evaluate(&stats(1.0,1.0,100,0.0),false,false).unwrap();
        assert_eq!(event.severity,Severity::Warning);
    }

    #[test]
    fn a_sustained_process_climb_is_a_fork_bomb() {
        let monitor = Monitor::new();
        let event = monitor.evaluate(&stats(1.0,1.0,100,0.0),false,true).unwrap();
        assert_eq!(event.severity,Severity::Critical);
    }

    #[test]
    fn the_worst_finding_wins() {
        let monitor = Monitor::new();
        // busy and out of memory at once, but neither isolates
        let event = monitor.evaluate(&stats(95.0,95.0,1,0.0),false,false).unwrap();
        assert_eq!(event.severity,Severity::Warning);
        // add a containment failure and that is what gets reported
        let event = monitor.evaluate(&stats(95.0,95.0,1,50_000.0),false,false).unwrap();
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
