// src/data_structures.rs

#[derive(Debug, Clone)] // Added Debug here
pub struct ProcessInfo {
    pub pid: i32,
    pub user: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub command: String,
}
