// src/cli.rs

use clap::{Parser, Subcommand};
use crate::process_handler::ProcessHandler;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

#[derive(Parser)]
#[command(name = "linux_task_manager")]
#[command(about = "A CLI-based Linux Task Manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all running processes
    List {
        /// Sort by field: pid, cpu, memory, command
        #[arg(short, long, default_value = "pid")]
        sort_by: String,

        /// Sort order: asc, desc
        #[arg(short, long, default_value = "asc")]
        order: String,

        /// Filter by command name or PID
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Kill a process by PID
    Kill {
        /// PID of the process to kill
        #[arg(short, long)]
        pid: i32,

        /// Signal to send (default: SIGTERM)
        #[arg(short, long, default_value = "SIGTERM")]
        signal: String,
    },
}

pub fn run_cli() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::List { sort_by, order, filter } => {
            let mut handler = ProcessHandler::new();
            let mut processes = handler.refresh_processes();

            if let Some(query) = filter {
                let query = query.to_lowercase();
                processes = processes.into_iter()
                    .filter(|p|
                        p.pid.to_string().contains(&query) ||
                        p.command.to_lowercase().contains(&query)
                    )
                    .collect();
            }

            match sort_by.as_str() {
                "pid" => {
                    if order == "asc" {
                        processes.sort_by_key(|p| p.pid);
                    } else {
                        processes.sort_by_key(|p| std::cmp::Reverse(p.pid));
                    }
                }
                "cpu" => {
                    if order == "asc" {
                        processes.sort_by(|a, b| a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap());
                    } else {
                        processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
                    }
                }
                "memory" => {
                    if order == "asc" {
                        processes.sort_by(|a, b| a.memory_usage.cmp(&b.memory_usage));
                    } else {
                        processes.sort_by(|a, b| b.memory_usage.cmp(&a.memory_usage));
                    }
                }
                "command" => {
                    if order == "asc" {
                        processes.sort_by(|a, b| a.command.cmp(&b.command));
                    } else {
                        processes.sort_by(|a, b| b.command.cmp(&a.command));
                    }
                }
                _ => {
                    eprintln!("Invalid sort field: {}", sort_by);
                    std::process::exit(1);
                }
            }

            println!("{:<10} {:<15} {:<10} {:<10} {}", "PID", "User", "CPU%", "Memory", "Command");
            for p in processes {
                println!("{:<10} {:<15} {:<10.2} {:<10} {}", p.pid, p.user, p.cpu_usage, p.memory_usage, p.command);
            }
        }

        Commands::Kill { pid, signal } => {
            let sig = match signal.as_str() {
                "SIGTERM" => Signal::SIGTERM,
                "SIGKILL" => Signal::SIGKILL,
                "SIGHUP" => Signal::SIGHUP,
                _ => {
                    eprintln!("Unsupported signal: {}", signal);
                    std::process::exit(1);
                }
            };

            let result = signal::kill(Pid::from_raw(*pid), sig);
            match result {
                Ok(_) => println!("Successfully sent {} to PID {}", signal, pid),
                Err(e) => eprintln!("Failed to send signal: {}", e),
            }
        }
    }
}
