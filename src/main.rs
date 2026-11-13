#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod launch;

use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Sense, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub nickname: String,
    pub version: String,
    pub ram_mb: u32,
    pub java_path: String,
    pub game_dir: String,
    #[serde(default = "yes")]
    pub prefer_opengl: bool,
}

fn yes() -> bool { true }

impl Default for Config {
    fn default() -> Self {
        let game_dir = dirs::data_dir().unwrap_or_default().join(".minecraft").to_string_lossy().to_string();
        Self {
            nickname: "Player".into(),
            version: String::new(),
            ram_mb: 4096,
            java_path: if cfg!(windows) { "javaw".into() } else { "java".into() },
            game_dir,
            prefer_opengl: true,
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir().unwrap_or_default().join("BlackLauncher").join("config.json")
}

fn load_config() -> Config {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(c: &Config) {
    let p = config_path();
    if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); }
    if let Ok(s) = serde_json::to_string_pretty(c) { let _ = std::fs::write(p, s); }
}

#[derive(PartialEq, Clone, Copy)]
enum Tab { Home, Settings }

struct App {
    cfg: Config,
    versions: Vec<String>,
    tab: Tab,
    editing_nick: bool,
    status: Arc<Mutex<(String, bool)>>,
}

const BG: Color32 = Color32::from_rgb(12, 12, 14);
const PANEL: Color32 = Color32::from_rgb(18, 18, 21);
const PANEL_2: Color32 = Color32::from_rgb(24, 24, 28);
const BORDER: Color32 = Color32::from_rgb(42, 42, 48);
const MUTED: Color32 = Color32::from_rgb(137, 137, 148);
const WHITE: Color32 = Color32::from_rgb(239, 239, 243);

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut v = egui::Visuals::dark();
        v.panel_fill = Color32::TRANSPARENT;
        v.window_fill = BG;
        v.override_text_color = Some(WHITE);
        v.widgets.noninteractive.bg_fill = PANEL;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        v.widgets.noninteractive.rounding = Rounding::same(9.0);
        for (w, fill) in [
            (&mut v.widgets.inactive, Color32::from_rgb(28, 28, 32)),
            (&mut v.widgets.hovered, Color32::from_rgb(43, 43, 49)),
            (&mut v.widgets.active, Color32::from_rgb(57, 57, 65)),
        ] {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.rounding = Rounding::same(8.0);
            w.bg_stroke = Stroke::new(1.0, BORDER);
        }
        v.selection.bg_fill = Color32::from_rgb(75, 75, 84);
        cc.egui_ctx.set_visuals(v);

        let mut cfg = load_config();
        let versions = launch::list_versions(&cfg.game_dir);
        if cfg.version.is_empty() || !versions.contains(&cfg.version) {
            cfg.version = versions.first().cloned().unwrap_or_default();
        }

        Self {
            cfg,
            versions,
            tab: Tab::Home,
            editing_nick: false,
            status: Arc::new(Mutex::new((String::new(), false))),
        }
    }

    fn busy(&self) -> bool {
        self.status.lock().map(|s| s.1).unwrap_or(false)
    }

    fn play(&mut self) {
        save_config(&self.cfg);
        *self.status.lock().unwrap() = ("Подготовка к запуску…".into(), true);
        let status = self.status.clone();
        let cfg = self.cfg.clone();

        std::thread::spawn(move || {
            let set = |t: String, b: bool| *status.lock().unwrap() = (t, b);
            match launch::launch(&cfg) {
                Ok(mut child) => {
                    set("Игра запущена".into(), true);
                    match child.wait() {
                        Ok(st) if st.success() => set("Игра закрыта".into(), false),
                        other => {
                            let code = other.ok().and_then(|x| x.code());
                            set(format!("Игра завершилась с ошибкой (код {:?}). Лог: {}", code, launch::log_path().display()), false);
                        }
                    }
                }
                Err(e) => set(format!("Ошибка запуска: {e}"), false),
            }
        });
    }

    fn refresh_versions(&mut self) {
        self.versions = launch::list_versions(&self.cfg.game_dir);
        if !self.versions.contains(&self.cfg.version) {
            self.cfg.version = self.versions.first().cloned().unwrap_or_default();
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] { [0.0, 0.0, 0.0, 0.0] }

    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.busy() { ctx.request_repaint_after(Duration::from_millis(300)); }

        let rect = ctx.screen_rect().shrink(1.0);
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(rect, Rounding::same(14.0), BG);
        painter.rect_stroke(rect, Rounding::same(14.0), Stroke::new(1.0, BORDER));

        egui::TopBottomPanel::top("top_bar")
            .exact_height(52.0)
            .frame(Frame::none().fill(PANEL).inner_margin(Margin::symmetric(18.0, 8.0)))
            .show(ctx, |ui| {
                let drag = ui.interact(ui.max_rect(), egui::Id::new("window_drag"), Sense::click_and_drag());
                if drag.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("BLACK").size(16.0).strong().color(WHITE));
                    ui.label(RichText::new("LAUNCHER").size(10.0).strong().color(MUTED));
                    ui.label(RichText::new("  /  v0.1.0").size(11.0).color(MUTED));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized([28.0, 27.0], egui::Button::new("×").frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.add_sized([28.0, 27.0], egui::Button::new("□").frame(false)).clicked() {
                            let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!full));
                        }
                        if ui.add_sized([28.0, 27.0], egui::Button::new("—").frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });

        egui::SidePanel::left("sidebar")
            .exact_width(205.0)
            .resizable(false)
            .frame(Frame::none().fill(PANEL).inner_margin(Margin::same(16.0)))
            .show(ctx, |ui| {
                ui.add_space(8.0);

                let (avatar_rect, _) = ui.allocate_exact_size(Vec2::splat(58.0), Sense::hover());
                let center = avatar_rect.center();
                ui.painter().circle_filled(center, 28.0, PANEL_2);
                ui.painter().circle_stroke(center, 28.0, Stroke::new(1.0, BORDER));
                ui.painter().line_segment(
                    [center + Vec2::new(-8.0, -8.0), center + Vec2::new(8.0, 8.0)],
                    Stroke::new(2.0, MUTED),
                );
                ui.painter().line_segment(
                    [center + Vec2::new(8.0, -8.0), center + Vec2::new(-8.0, 8.0)],
                    Stroke::new(2.0, MUTED),
                );

                ui.add_space(9.0);
                ui.label(RichText::new("ВАШ ПРОФИЛЬ").size(9.0).strong().color(MUTED));
                if self.editing_nick {
                    let response = ui.add(egui::TextEdit::singleline(&mut self.cfg.nickname).desired_width(165.0));
                    if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.editing_nick = false;
                        if self.cfg.nickname.trim().is_empty() { self.cfg.nickname = "Player".into(); }
                        save_config(&self.cfg);
                    }
                    response.request_focus();
                } else {
                    let nick = if self.cfg.nickname.trim().is_empty() { "Player" } else { &self.cfg.nickname };
                    let response = ui.add(egui::Label::new(RichText::new(nick).size(15.0).strong()).sense(Sense::click()));
                    if response.double_clicked() { self.editing_nick = true; }
                    response.on_hover_text("Двойной клик — изменить ник");
                }
                ui.add_space(28.0);
                ui.label(RichText::new("МЕНЮ").size(9.0).strong().color(MUTED));
                ui.add_space(8.0);

                for (tab, label, hint) in [
                    (Tab::Home, "Главная", "Обзор и запуск игры"),
                    (Tab::Settings, "Настройки", "Параметры лаунчера"),
                ] {
                    let selected = self.tab == tab;
                    let fill = if selected { PANEL_2 } else { Color32::TRANSPARENT };
                    let color = if selected { WHITE } else { MUTED };
                    let response = ui.add_sized(
                        [172.0, 42.0],
                        egui::Button::new(RichText::new(label).size(14.0).color(color))
                            .fill(fill)
                            .stroke(if selected { Stroke::new(1.0, BORDER) } else { Stroke::NONE }),
                    );
                    if response.clicked() { self.tab = tab; }
                    response.on_hover_text(hint);
                    ui.add_space(5.0);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.label(RichText::new("BLACK LAUNCHER").size(9.0).color(MUTED));
                    ui.label(RichText::new("Локальный профиль").size(10.0).color(MUTED));
                });
            });

        egui::CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::same(28.0)))
            .show(ctx, |ui| {
                match self.tab {
                    Tab::Home => {
                        ui.vertical_centered(|ui| {
                            ui.add_space(35.0);
                            ui.label(RichText::new("ТВОЯ ИГРА. ТВОИ ПРАВИЛА.").size(10.0).strong().color(MUTED));
                            ui.add_space(10.0);
                            ui.label(RichText::new("Добро пожаловать").size(30.0).strong().color(WHITE));
                            ui.label(RichText::new("Выбери версию и возвращайся в игру").size(13.0).color(MUTED));
                            ui.add_space(38.0);

                            let width = ui.available_width().min(470.0);
                            ui.allocate_ui_with_layout(
                                Vec2::new(width, ui.available_height()),
                                egui::Layout::top_down(egui::Align::Center),
                                |ui| {
                                    Frame::none()
                                        .fill(PANEL)
                                        .stroke(Stroke::new(1.0, BORDER))
                                        .rounding(Rounding::same(12.0))
                                        .inner_margin(Margin::same(18.0))
                                        .show(ui, |ui| {
                                            ui.set_width((width - 38.0).max(220.0));
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("ВЕРСИЯ ИГРЫ").size(10.0).strong().color(MUTED));
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if ui.small_button("Обновить список").clicked() { self.refresh_versions(); }
                                                });
                                            });
                                            ui.add_space(10.0);
                                            egui::ComboBox::from_id_salt("version_picker")
                                                .width(ui.available_width())
                                                .selected_text(if self.cfg.version.is_empty() {
                                                "Версии не найдены".to_string()
                                                } else {
                                                        self.cfg.version.clone()
                                            })
                                                .show_ui(ui, |ui| {
                                                    for version in self.versions.clone() {
                                                        ui.selectable_value(&mut self.cfg.version, version.clone(), version);
                                                    }
                                                });
                                            ui.add_space(16.0);
                                            let busy = self.busy();
                                            let button = egui::Button::new(
                                                RichText::new(if busy { "ЗАПУСК..." } else { "ИГРАТЬ" })
                                                    .size(16.0).strong().color(Color32::from_rgb(15, 15, 17))
                                            )
                                            .fill(Color32::from_rgb(238, 238, 242))
                                            .rounding(Rounding::same(9.0))
                                            .min_size(Vec2::new(ui.available_width(), 48.0));
                                            if ui.add_enabled(!busy && !self.cfg.version.is_empty(), button).clicked() {
                                                self.play();
                                            }
                                            if self.versions.is_empty() {
                                                ui.add_space(9.0);
                                                ui.label(RichText::new("Папка versions пуста. Добавь установленную версию игры.").size(10.0).color(MUTED));
                                            }
                                        });

                                    ui.add_space(14.0);
                                    let status = self.status.lock().map(|s| s.0.clone()).unwrap_or_default();
                                    if !status.is_empty() {
                                        Frame::none().fill(PANEL).rounding(Rounding::same(8.0))
                                            .inner_margin(Margin::same(12.0)).show(ui, |ui| {
                                                ui.set_width((width - 24.0).max(200.0));
                                                ui.label(RichText::new(status).size(11.0).color(MUTED));
                                            });
                                    }
                                    ui.add_space(10.0);
                                    if ui.link(RichText::new("Открыть игровой лог").size(11.0).color(MUTED)).clicked() {
                                        let _ = std::process::Command::new(if cfg!(windows) { "notepad" } else { "xdg-open" })
                                            .arg(launch::log_path()).spawn();
                                    }
                                },
                            );
                        });
                    }
                    Tab::Settings => {
                        ui.label(RichText::new("Настройки").size(27.0).strong());
                        ui.label(RichText::new("Настрой лаунчер под себя").size(12.0).color(MUTED));
                        ui.add_space(24.0);

                        Frame::none().fill(PANEL).stroke(Stroke::new(1.0, BORDER))
                            .rounding(Rounding::same(12.0)).inner_margin(Margin::same(18.0))
                            .show(ui, |ui| {
                                ui.set_max_width(620.0);
                                ui.label(RichText::new("АККАУНТ").size(10.0).strong().color(MUTED));
                                ui.add_space(8.0);
                                ui.label("Никнейм (офлайн-профиль)");
                                ui.add(egui::TextEdit::singleline(&mut self.cfg.nickname).desired_width(360.0));
                                ui.add_space(18.0);

                                ui.label(RichText::new("ПРОИЗВОДИТЕЛЬНОСТЬ").size(10.0).strong().color(MUTED));
                                ui.add_space(8.0);
                                ui.label("Оперативная память");
                                ui.add(egui::Slider::new(&mut self.cfg.ram_mb, 1024..=16384).step_by(512.0).suffix(" МБ"));
                                ui.add_space(18.0);

                                ui.label(RichText::new("ПУТИ И ФАЙЛЫ").size(10.0).strong().color(MUTED));
                                ui.add_space(8.0);
                                ui.label("Java (javaw или полный путь)");
                                ui.add(egui::TextEdit::singleline(&mut self.cfg.java_path).desired_width(520.0));
                                ui.add_space(10.0);
                                ui.label("Папка игры (.minecraft)");
                                ui.add(egui::TextEdit::singleline(&mut self.cfg.game_dir).desired_width(520.0));
                                ui.add_space(14.0);
                                ui.checkbox(&mut self.cfg.prefer_opengl, "Предпочитать OpenGL (для старых видеокарт)");
                                ui.add_space(18.0);
                                ui.horizontal(|ui| {
                                    if ui.button("Сохранить настройки").clicked() { save_config(&self.cfg); }
                                    if ui.button("Обновить версии").clicked() { self.refresh_versions(); }
                                });
                            });
                    }
                }
            });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 650.0])
            .with_min_inner_size([760.0, 520.0])
            .with_resizable(true)
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };
    eframe::run_native("Black Launcher", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
