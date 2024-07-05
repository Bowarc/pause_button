use eframe::egui;
use winapi::um::{processthreadsapi::OpenProcess, winnt::PROCESS_SUSPEND_RESUME};

const TITLE_BAR_HEIGHT: f32 = 32.0;


#[derive(Clone)]
pub struct Process {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub name: String,
}

enum State {
    ProcessSelect {
        processes: Vec<Process>,
        search_string: String,
        hide_childs: bool,
    },
    Main {
        process: Process,
        paused: bool,
    },
}

pub struct Ui {
    state: State,
    system: sysinfo::System,
}

/// Normal functions
impl Ui {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        use egui::{
            FontFamily::{Monospace, Proportional},
            FontId, TextStyle,
        };

        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles = [
            (TextStyle::Heading, FontId::new(25.0, Proportional)),
            (TextStyle::Body, FontId::new(16.0, Proportional)),
            (TextStyle::Monospace, FontId::new(16.0, Monospace)),
            (TextStyle::Button, FontId::new(16.0, Proportional)),
            (TextStyle::Small, FontId::new(8.0, Proportional)),
        ]
        .into();
        cc.egui_ctx.set_style(style);

        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessRefreshKind::new().with_user(sysinfo::UpdateKind::Always),
        );

        let mut processes = Vec::new();

        let current_pid = sysinfo::get_current_pid().unwrap();

        let current_process = system.process(current_pid).unwrap();

        let current_proccess_user_id = current_process.user_id().unwrap();

        for (pid, p) in system.processes().iter() {
            let Some(id) = p.user_id() else {
                continue;
            };

            if id == current_proccess_user_id {
                processes.push(Process {
                    pid: pid.as_u32(),
                    ppid: p.parent().and_then(|ppid| {
                        if let Some(parent_process) = system.process(ppid) {
                            if parent_process.name() == "explorer.exe" {
                                return None;
                            }
                            Some(ppid.as_u32())
                        } else {
                            None
                        }
                    }),

                    name: p.name().to_string(),
                });
                // println!("{}({})\t {:?}", p.name(), p.pid(), p.parent());
                // if let Some(parent) = p.parent() {
                //     println!(
                //         "\t{}",
                //         system
                //             .process(parent)
                //             .and_then(|p| Some(p.name()))
                //             .unwrap_or("Not found")
                //     )
                // }
            }
        }

        processes.sort_unstable_by_key(|process| process.pid);

        Self {
            state: State::ProcessSelect {
                processes,
                search_string: String::new(),
                hide_childs: false,
            },
            system,
        }
    }
}

/// Interface related functions
impl Ui {
    fn render_title_bar(
        &mut self,
        ui: &mut egui::Ui,
        ectx: &egui::Context,
        title_bar_rect: eframe::epaint::Rect,
        title: &str,
    ) {
        let painter = ui.painter();

        let title_bar_response = ui.interact(
            title_bar_rect,
            egui::Id::new("title_bar"),
            egui::Sense::click(),
        );

        // Paint the title:
        painter.text(
            title_bar_rect.center(),
            eframe::emath::Align2::CENTER_CENTER,
            title,
            eframe::epaint::FontId::proportional(20.0),
            ui.style().visuals.text_color(),
        );

        // Paint the line under the title:
        painter.line_segment(
            [
                title_bar_rect.left_bottom() + eframe::epaint::vec2(1.0, 0.0),
                title_bar_rect.right_bottom() + eframe::epaint::vec2(-1.0, 0.0),
            ],
            ui.visuals().widgets.noninteractive.bg_stroke,
        );

        // Interact with the title bar (drag to move window):
        if title_bar_response.double_clicked() {
            // frame.set_maximized(!frame.info().window_info.maximized);
        } else if title_bar_response.is_pointer_button_down_on() {
            ectx.send_viewport_cmd(egui::viewport::ViewportCommand::StartDrag);
            // frame.drag_window();
        }

        // Show toggle button for light/dark mode
        ui.allocate_ui_at_rect(title_bar_rect, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.visuals_mut().button_frame = false;
                ui.add_space(8.0);
                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });

        // Show some close/maximize/minimize buttons for the native window.
        ui.allocate_ui_at_rect(title_bar_rect, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.visuals_mut().button_frame = false;
                ui.add_space(8.0);

                let button_height = 12.0;

                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("❌").size(button_height),
                    ))
                    .on_hover_text("Close the window")
                    .clicked()
                {
                    ectx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
                }

                let (hover_text, clicked_state) =
                    if ui.input(|i| i.viewport().maximized) == Some(true) {
                        ("Restore window", false)
                    } else {
                        ("Maximize window", true)
                    };

                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("🗗").size(button_height),
                    ))
                    .on_hover_text(hover_text)
                    .clicked()
                {
                    if clicked_state {
                        ectx.send_viewport_cmd(egui::viewport::ViewportCommand::Maximized(true));
                    } else {
                        ectx.send_viewport_cmd(egui::viewport::ViewportCommand::Maximized(false));
                    }
                }

                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("🗕").size(button_height),
                    ))
                    .on_hover_text("Minimize the window")
                    .clicked()
                {
                    ectx.send_viewport_cmd(egui::viewport::ViewportCommand::Minimized(true));
                }
            });
        });
    }

    fn process_select(&mut self, ui: &mut egui::Ui, ectx: &egui::Context) {
        let State::ProcessSelect {
            processes,
            search_string,
            hide_childs,
        } = &mut self.state
        else {
            return;
        };

        let len = processes.len();

        ui.horizontal(|ui| {
            ui.label("Process selection");
            if ui
                .button(if *hide_childs {
                    "Show childs"
                } else {
                    "Hide childs"
                })
                .clicked()
            {
                *hide_childs = !*hide_childs
            }
        });
        ui.text_edit_singleline(search_string);

        let mut selected_process = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for i in 0..len {
                let process = processes.get(i).unwrap();
                if *hide_childs && process.ppid != None {
                    continue;
                }
                if !process
                    .name
                    .to_lowercase()
                    .contains(&search_string.to_lowercase())
                    && !search_string.is_empty()
                {
                    continue;
                }
                let mut parent = String::new();
                if let Some(ppid) = process.ppid {
                    parent = self
                        .system
                        .process(sysinfo::Pid::from_u32(ppid))
                        .and_then(|p| Some(p.name()))
                        .unwrap_or("Not found")
                        .to_string();
                }
                if ui
                    .button(format!(
                        "Name: {}, pid: {}, parent: {parent}",
                        process.name, process.pid
                    ))
                    .clicked()
                {
                    // println!("{}", process.pid);
                    selected_process = Some(process);
                    break;
                }
            }
        });

        if let Some(process) = selected_process {
            self.state = State::Main {
                process: process.clone(),
                paused: false,
            };
            ectx.send_viewport_cmd(egui::viewport::ViewportCommand::InnerSize(egui::Vec2 {
                x: 300.,
                y: 200.,
            }));
        }
    }

    fn main_menu(&mut self, ui: &mut egui::Ui) {
        use egui::RichText;
        let State::Main { process, paused } = &mut self.state else {
            return;
        };
        ui.label(format!("Hooked to {} w/ pid {}", process.name, process.pid));
        ui.with_layout(
            eframe::egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                ui.add_space(10.);
                if ui
                    .button(RichText::new(if *paused { "Resume" } else { "Pause" }).size(50.))
                    .clicked()
                {
                    use ntapi::ntpsapi::{NtResumeProcess, NtSuspendProcess};
                    use winapi::shared::minwindef::FALSE;

                    let handle = unsafe { OpenProcess(PROCESS_SUSPEND_RESUME, FALSE, process.pid) };
                    if *paused {
                        let _res = unsafe { NtResumeProcess(handle) };
                        // println!("Resumed pid {} with return type: {}", process.pid, res);
                    } else {
                        let _res = unsafe { NtSuspendProcess(handle) };
                        // println!("Suspended pid {} with return type: {}", process.pid, res);
                    }
                    *paused = !*paused;
                }
            },
        );
    }
}

impl eframe::App for Ui {
    fn update(&mut self, ectx: &egui::Context, _frame: &mut eframe::Frame) {
        // ectx.set_debug_on_hover(true);

        egui::CentralPanel::default()
            .frame(
                eframe::egui::Frame::none()
                    .fill(ectx.style().visuals.window_fill())
                    .rounding(10.0)
                    .stroke(ectx.style().visuals.widgets.noninteractive.fg_stroke)
                    .outer_margin(0.5),
            )
            .show(ectx, |ui| {
                let app_rect = ui.max_rect();

                // draw the title bar

                let title_bar_rect = {
                    let mut rect = app_rect;
                    rect.max.y = rect.min.y + TITLE_BAR_HEIGHT;
                    rect
                };
                self.render_title_bar(ui, ectx, title_bar_rect, "Pause button");

                // rest of the window
                let bg_content_rect = {
                    let mut rect = app_rect;
                    rect.min.y = title_bar_rect.max.y;
                    rect.max.y = app_rect.max.y;
                    rect
                }
                .shrink(4.0);

                ui.allocate_ui_at_rect(bg_content_rect, |ui| {
                    ui.style_mut().spacing.indent = 10.;
                    match self.state {
                        State::ProcessSelect { .. } => {
                            self.process_select(ui, ectx);
                        }
                        State::Main { .. } => {
                            self.main_menu(ui);
                        }
                    }
                })
            });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }
}

fn main() {
    eframe::run_native(
        "Pause menu",
        eframe::NativeOptions {
            // initial_window_size: Some(eframe::egui::vec2(800.0, 600.0)), /*x800y450 is 16:9*/
            // resizable: false,
            // centered: true,
            // vsync: true,
            // decorated: false,
            // transparent: true,
            // always_on_top: true,
            follow_system_theme: true,
            run_and_return: true,
            centered: true,
            vsync: true,
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size(eframe::egui::vec2(800.0, 600.0))
                .with_decorations(false)
                .with_transparent(true)
                .with_resizable(false)
                .with_title("Pause menu"),

            // default_theme: eframe::Theme::Dark,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::<Ui>::new(Ui::new(cc)))),
    )
    .unwrap();
}
