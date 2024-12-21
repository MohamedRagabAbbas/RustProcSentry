// src/main.rs

mod data_structures;
mod process_handler;
mod ui;
use iced::Application;

use ui::TaskManager;

fn main() {
    TaskManager::run(iced::Settings::default()).unwrap();
}
