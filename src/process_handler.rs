// src/process_handler.rs

use sysinfo::{CpuExt, PidExt, ProcessExt, System, SystemExt};
use crate::data_structures::ProcessInfo;

pub struct ProcessHandler {
    system: System,
    cpu_usage_history: Vec<f32>,
    memory_usage_history: Vec<f32>,
}

impl ProcessHandler {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self {
            system,
            cpu_usage_history: Vec::new(),
            memory_usage_history: Vec::new(),
        }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_all();

        // Update CPU usage history
        let cpu_usage = self.system.global_cpu_info().cpu_usage();
        self.cpu_usage_history.push(cpu_usage);
        if self.cpu_usage_history.len() > 100 {
            self.cpu_usage_history.remove(0);
        }

        // Update memory usage history
        let total_memory = self.system.total_memory() as f32;
        let used_memory = self.system.used_memory() as f32;
        let memory_usage_percent = (used_memory / total_memory) * 100.0;
        self.memory_usage_history.push(memory_usage_percent);
        if self.memory_usage_history.len() > 100 {
            self.memory_usage_history.remove(0);
        }
    }

    pub fn get_cpu_usage_history(&self) -> &[f32] {
        &self.cpu_usage_history
    }

    pub fn get_memory_usage_history(&self) -> &[f32] {
        &self.memory_usage_history
    }

    pub fn refresh_processes(&mut self) -> Vec<ProcessInfo> {
        self.system.refresh_processes();
        self.system
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: pid.as_u32() as i32,
                user: process
                    .user_id()
                    .map(|uid| uid.to_string())
                    .unwrap_or_else(|| "Unknown".into()),
                cpu_usage: process.cpu_usage(),
                memory_usage: process.memory(),
                command: process.name().to_string(),
            })
            .collect()
    }

    pub fn kill_process(&self, pid: i32) -> Result<(), String> {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        match kill(Pid::from_raw(pid), Signal::SIGTERM) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to kill process {}: {}", pid, e)),
        }
    }
}
