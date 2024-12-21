// src/ui.rs

use iced::{
    alignment::Alignment,
    executor,
    mouse::Cursor,
    time::every,
    widget::{
        button::Button,
        canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Style, Text as CanvasText},
        scrollable::Scrollable,
        text_input::TextInput,
        Column, Container, Row, Text, // Removed Length and Length::Fixed from here
    },
    Application, Command, Element, Length, // Import Length here
    Rectangle, Renderer, Subscription, Theme,
};
use crate::data_structures::ProcessInfo;
use crate::process_handler::ProcessHandler;
use std::sync::{Arc, Mutex};

const SPIKE_THRESHOLD: f32 = 20.0; // Spike threshold in percentage

pub struct TaskManager {
    process_handler: Arc<Mutex<ProcessHandler>>,
    processes: Vec<ProcessInfo>,
    filtered_processes: Vec<ProcessInfo>,
    cpu_usage_history: Vec<f32>,
    memory_usage_history: Vec<f32>,
    search_query: String,
    sort_field: SortField,
    sort_order: SortOrder,
    show_graphs: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    RefreshComplete(Vec<ProcessInfo>, Vec<f32>, Vec<f32>),
    KillProcess(i32),
    KillComplete(Result<(), String>),
    SearchChanged(String),
    SortBy(SortField),
    ToggleGraphs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortField {
    PID,
    CPU,
    Memory,
    Command,
}

impl Application for TaskManager {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let process_handler = ProcessHandler::new();
        let handler = Arc::new(Mutex::new(process_handler));
        let processes = handler.lock().unwrap().refresh_processes();
        let cpu_usage_history = handler.lock().unwrap().get_cpu_usage_history().to_vec();
        let memory_usage_history = handler.lock().unwrap().get_memory_usage_history().to_vec();

        (
            TaskManager {
                process_handler: handler,
                processes: processes.clone(),
                filtered_processes: processes,
                cpu_usage_history,
                memory_usage_history,
                search_query: String::new(),
                sort_field: SortField::PID,
                sort_order: SortOrder::Ascending,
                show_graphs: true,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Rust Task Manager")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Refresh => {
                let handler = Arc::clone(&self.process_handler);
                Command::perform(
                    async move {
                        let mut handler = handler.lock().unwrap();
                        handler.refresh();
                        let processes = handler.refresh_processes();
                        let cpu_usage_history = handler.get_cpu_usage_history().to_vec();
                        let memory_usage_history = handler.get_memory_usage_history().to_vec();
                        Message::RefreshComplete(
                            processes,
                            cpu_usage_history,
                            memory_usage_history,
                        )
                    },
                    |msg| msg,
                )
            }
            Message::RefreshComplete(processes, cpu_usage_history, memory_usage_history) => {
                self.processes = processes;
                self.cpu_usage_history = cpu_usage_history;
                self.memory_usage_history = memory_usage_history;
                self.apply_filter_and_sort();
                Command::none()
            }
            Message::KillProcess(pid) => {
                let handler = Arc::clone(&self.process_handler);
                Command::perform(
                    async move {
                        let handler = handler.lock().unwrap();
                        let result = handler.kill_process(pid);
                        Message::KillComplete(result)
                    },
                    |msg| msg,
                )
            }
            Message::KillComplete(result) => {
                match result {
                    Ok(_) => {
                        println!("Process killed successfully.");
                    }
                    Err(e) => {
                        println!("{}", e);
                    }
                }
                Command::perform(async { Message::Refresh }, |msg| msg)
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                self.apply_filter_and_sort();
                Command::none()
            }
            Message::SortBy(field) => {
                if self.sort_field == field {
                    self.sort_order = match self.sort_order {
                        SortOrder::Ascending => SortOrder::Descending,
                        SortOrder::Descending => SortOrder::Ascending,
                    };
                } else {
                    self.sort_field = field;
                    self.sort_order = SortOrder::Ascending;
                }
                self.apply_filter_and_sort();
                Command::none()
            }
            Message::ToggleGraphs => {
                self.show_graphs = !self.show_graphs;
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let header = Row::new()
            .padding(10)
            .spacing(20)
            .align_items(Alignment::Center)
            .push(Text::new("Rust Task Manager").size(30))
            .push(
                TextInput::new(
                    "Search by PID or Command...",
                    &self.search_query,
                )
                .on_input(Message::SearchChanged)
                .padding(10)
                .size(20)
                .width(Length::Fixed(300.0)), // Use Length::Fixed here
            )
            .push(
                Button::new(Text::new(if self.show_graphs { "Hide Graphs" } else { "Show Graphs" }))
                    .on_press(Message::ToggleGraphs)
                    .padding(10),
            )
            .push(
                Button::new(Text::new("Refresh"))
                    .on_press(Message::Refresh)
                    .padding(10),
            );

        let cpu_usage_chart = Canvas::new(CpuUsageChart::new(self.cpu_usage_history.clone()))
            .width(Length::FillPortion(1))
            .height(Length::Fixed(200.0));

        let memory_usage_chart =
            Canvas::new(MemoryUsageChart::new(self.memory_usage_history.clone()))
                .width(Length::FillPortion(1))
                .height(Length::Fixed(200.0));

        let charts_row = Row::new()
            .push(cpu_usage_chart)
            .push(memory_usage_chart)
            .spacing(20)
            .padding(10)
            .height(Length::Fixed(220.0));

        let header_row = Row::new()
            .spacing(20)
            .padding(10)
            .push(
                Button::new(Text::new("PID"))
                    .on_press(Message::SortBy(SortField::PID))
                    .padding(5),
            )
            .push(Text::new("User").width(Length::Fixed(100.0)))
            .push(
                Button::new(Text::new("CPU %"))
                    .on_press(Message::SortBy(SortField::CPU))
                    .padding(5),
            )
            .push(
                Button::new(Text::new("Memory"))
                    .on_press(Message::SortBy(SortField::Memory))
                    .padding(5),
            )
            .push(
                Button::new(Text::new("Command"))
                    .on_press(Message::SortBy(SortField::Command))
                    .padding(5),
            )
            .push(Text::new("Actions").width(Length::Fixed(80.0)));

        let process_list = self.filtered_processes.iter().fold(
            Column::new().spacing(10).padding(10),
            |column, process| {
                column.push(
                    Container::new(
                        Row::new()
                            .spacing(20)
                            .align_items(Alignment::Center)
                            .push(
                                Text::new(process.pid.to_string()).width(Length::Fixed(60.0)),
                            )
                            .push(Text::new(&process.user).width(Length::Fixed(100.0)))
                            .push(
                                Text::new(format!("{:.2}%", process.cpu_usage))
                                    .width(Length::Fixed(80.0)),
                            )
                            .push(
                                Text::new(format!("{} KB", process.memory_usage))
                                    .width(Length::Fixed(100.0)),
                            )
                            .push(Text::new(&process.command).width(Length::Fill))
                            .push(
                                Button::new(Text::new("Kill"))
                                    .on_press(Message::KillProcess(process.pid))
                                    .padding(5),
                            ),
                    )
                    .padding(5),
                )
            },
        );

        let scrollable_content = Scrollable::new(process_list);

        let mut content = Column::new()
            .push(header);

        if self.show_graphs {
            content = content.push(charts_row);
        }

        content = content
            .push(header_row)
            .push(scrollable_content);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .center_x()
            .center_y()
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        every(std::time::Duration::from_secs(1)).map(|_| Message::Refresh)
    }
}

impl TaskManager {
    fn apply_filter_and_sort(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_processes = self.processes.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_processes = self
                .processes
                .iter()
                .filter(|p| {
                    p.pid.to_string().contains(&query)
                        || p.command.to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }

        match self.sort_field {
            SortField::PID => {
                if self.sort_order == SortOrder::Ascending {
                    self.filtered_processes.sort_by_key(|p| p.pid);
                } else {
                    self.filtered_processes
                        .sort_by_key(|p| std::cmp::Reverse(p.pid));
                }
            }
            SortField::CPU => {
                if self.sort_order == SortOrder::Ascending {
                    self.filtered_processes.sort_by(|a, b| {
                        a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap()
                    });
                } else {
                    self.filtered_processes.sort_by(|a, b| {
                        b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap()
                    });
                }
            }
            SortField::Memory => {
                if self.sort_order == SortOrder::Ascending {
                    self.filtered_processes
                        .sort_by(|a, b| a.memory_usage.cmp(&b.memory_usage));
                } else {
                    self.filtered_processes
                        .sort_by(|a, b| b.memory_usage.cmp(&a.memory_usage));
                }
            }
            SortField::Command => {
                if self.sort_order == SortOrder::Ascending {
                    self.filtered_processes
                        .sort_by(|a, b| a.command.cmp(&b.command));
                } else {
                    self.filtered_processes
                        .sort_by(|a, b| b.command.cmp(&a.command));
                }
            }
        }
    }
}

// CPU Usage Chart with Spike Detection
struct CpuUsageChart {
    cpu_usage_history: Vec<f32>,
}

impl CpuUsageChart {
    fn new(cpu_usage_history: Vec<f32>) -> Self {
        Self { cpu_usage_history }
    }
}

impl<Message> canvas::Program<Message> for CpuUsageChart {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        if self.cpu_usage_history.len() < 2 {
            return vec![frame.into_geometry()];
        }

        let max_value = 100.0;
        let min_value = 0.0;

        let step_x = bounds.width / (self.cpu_usage_history.len() - 1) as f32;
        let scale_y = bounds.height / (max_value - min_value);

        // Draw grid lines
        for i in 0..=5 {
            let y = i as f32 * bounds.height / 5.0;
            frame.stroke(
                &Path::line(
                    iced::Point::new(0.0, y),
                    iced::Point::new(bounds.width, y),
                ),
                Stroke {
                    style: Style::Solid(iced::Color::from_rgb(0.9, 0.9, 0.9)),
                    width: 1.0,
                    ..Stroke::default()
                },
            );
        }

        // Draw axes
        frame.stroke(
            &Path::line(
                iced::Point::new(0.0, bounds.height),
                iced::Point::new(bounds.width, bounds.height),
            ),
            Stroke::default().with_width(1.0),
        );
        frame.stroke(
            &Path::line(
                iced::Point::new(0.0, 0.0),
                iced::Point::new(0.0, bounds.height),
            ),
            Stroke::default().with_width(1.0),
        );

        // Draw labels
        frame.fill_text(CanvasText {
            content: "CPU Usage (%)".to_string(),
            position: iced::Point::new(5.0, 20.0),
            color: iced::Color::from_rgb(0.2, 0.2, 0.2),
            size: 18.0,
            ..CanvasText::default()
        });

        // Initialize previous point and value
        let mut previous_value = self.cpu_usage_history[0];
        let mut previous_point = iced::Point::new(
            0.0,
            bounds.height - (previous_value - min_value) * scale_y,
        );

        for (i, &current_value) in self.cpu_usage_history.iter().enumerate().skip(1) {
            let x = i as f32 * step_x;
            let y = bounds.height - (current_value - min_value) * scale_y;
            let current_point = iced::Point::new(x, y);

            // Calculate percentage change
            let percentage_change = if previous_value.abs() > std::f32::EPSILON {
                ((current_value - previous_value) / previous_value.abs()) * 100.0
            } else {
                0.0
            };

            // Set line color based on spike detection
            let line_color = if percentage_change.abs() > SPIKE_THRESHOLD {
                iced::Color::from_rgb(1.0, 0.0, 0.0) // Red color for spikes
            } else {
                iced::Color::from_rgb(0.0, 0.5, 0.5) // Normal color
            };

            // Draw line segment
            frame.stroke(
                &Path::line(previous_point, current_point),
                Stroke {
                    style: Style::Solid(line_color),
                    width: 2.0,
                    ..Stroke::default()
                },
            );

            previous_value = current_value;
            previous_point = current_point;
        }

        vec![frame.into_geometry()]
    }
}

// Memory Usage Chart with Spike Detection
struct MemoryUsageChart {
    memory_usage_history: Vec<f32>,
}

impl MemoryUsageChart {
    fn new(memory_usage_history: Vec<f32>) -> Self {
        Self { memory_usage_history }
    }
}

impl<Message> canvas::Program<Message> for MemoryUsageChart {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        if self.memory_usage_history.len() < 2 {
            return vec![frame.into_geometry()];
        }

        let max_value = 100.0;
        let min_value = 0.0;

        let step_x = bounds.width / (self.memory_usage_history.len() - 1) as f32;
        let scale_y = bounds.height / (max_value - min_value);

        // Draw grid lines
        for i in 0..=5 {
            let y = i as f32 * bounds.height / 5.0;
            frame.stroke(
                &Path::line(
                    iced::Point::new(0.0, y),
                    iced::Point::new(bounds.width, y),
                ),
                Stroke {
                    style: Style::Solid(iced::Color::from_rgb(0.9, 0.9, 0.9)),
                    width: 1.0,
                    ..Stroke::default()
                },
            );
        }

        // Draw axes
        frame.stroke(
            &Path::line(
                iced::Point::new(0.0, bounds.height),
                iced::Point::new(bounds.width, bounds.height),
            ),
            Stroke::default().with_width(1.0),
        );
        frame.stroke(
            &Path::line(
                iced::Point::new(0.0, 0.0),
                iced::Point::new(0.0, bounds.height),
            ),
            Stroke::default().with_width(1.0),
        );

        // Draw labels
        frame.fill_text(CanvasText {
            content: "Memory Usage (%)".to_string(),
            position: iced::Point::new(5.0, 20.0),
            color: iced::Color::from_rgb(0.2, 0.2, 0.2),
            size: 18.0,
            ..CanvasText::default()
        });

        // Initialize previous point and value
        let mut previous_value = self.memory_usage_history[0];
        let mut previous_point = iced::Point::new(
            0.0,
            bounds.height - (previous_value - min_value) * scale_y,
        );

        for (i, &current_value) in self.memory_usage_history.iter().enumerate().skip(1) {
            let x = i as f32 * step_x;
            let y = bounds.height - (current_value - min_value) * scale_y;
            let current_point = iced::Point::new(x, y);

            // Calculate percentage change
            let percentage_change = if previous_value.abs() > std::f32::EPSILON {
                ((current_value - previous_value) / previous_value.abs()) * 100.0
            } else {
                0.0
            };

            // Set line color based on spike detection
            let line_color = if percentage_change.abs() > SPIKE_THRESHOLD {
                iced::Color::from_rgb(1.0, 0.0, 0.0) // Red color for spikes
            } else {
                iced::Color::from_rgb(0.5, 0.0, 0.5) // Normal color
            };

            // Draw line segment
            frame.stroke(
                &Path::line(previous_point, current_point),
                Stroke {
                    style: Style::Solid(line_color),
                    width: 2.0,
                    ..Stroke::default()
                },
            );

            previous_value = current_value;
            previous_point = current_point;
        }

        vec![frame.into_geometry()]
    }
}
