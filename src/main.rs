#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod art;
mod lrclib;
mod spotify;
mod state;
mod ui;

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, RichText};
use spotify::PlayerCommand;
use state::AppStatus;

fn window_pos_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|dir| dir.join("window_pos.txt")))
        .unwrap_or_else(|| std::path::PathBuf::from("window_pos.txt"))
}

fn load_window_pos() -> Option<[f32; 2]> {
    let text = std::fs::read_to_string(window_pos_path()).ok()?;
    let mut parts = text.trim().split(',');
    let x: f32 = parts.next()?.parse().ok()?;
    let y: f32 = parts.next()?.parse().ok()?;
    if x > -10000.0 && y > -10000.0 {
        Some([x, y])
    } else {
        None
    }
}

fn save_window_pos(pos: egui::Pos2) {
    if pos.x > -10000.0 && pos.y > -10000.0 {
        let _ = std::fs::write(window_pos_path(), format!("{},{}", pos.x, pos.y));
    }
}

struct SpotiFrameApp {
    client_id_input: String,
    login_error: Option<String>,
    status: Arc<Mutex<AppStatus>>,
    cmd_tx: Option<Sender<PlayerCommand>>,
    loaded_track_id: String,
    cover_tex: Option<egui::TextureHandle>,
    bg_tex: Option<egui::TextureHandle>,
    idle_gif: Option<art::GifPlayer>,
    last_active_idx: Option<usize>,
    is_fullscreen: bool,
    was_maximized_before_fs: bool,
    needs_initial_fullscreen: bool,
    last_window_pos: Option<egui::Pos2>,
    logo_tex: Option<egui::TextureHandle>,
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let bold_family = FontFamily::Name("Bold".into());

    // Grab egui's built-in fallback fonts (includes NotoEmoji-Regular & emoji-icon-font for ♪)
    let default_fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    // Regular font -> default Proportional (artist name, timestamps, etc.)
    if let Ok(bytes) = std::fs::read("assets/Inter-Regular.ttf") {
        fonts
            .font_data
            .insert("inter_regular".to_owned(), FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "inter_regular".to_owned());
    }

    // Bold font -> custom "Bold" family, inheriting the built-in fallbacks
    let mut bold_list = default_fallbacks;
    if let Ok(bytes) = std::fs::read("assets/Inter-Bold.ttf") {
        fonts
            .font_data
            .insert("inter_bold".to_owned(), FontData::from_owned(bytes));
        bold_list.insert(0, "inter_bold".to_owned());
    }

    // Optional: also add your local NotoEmoji.ttf if present
    if let Ok(bytes) = std::fs::read("assets/NotoEmoji.ttf") {
        fonts
            .font_data
            .insert("noto_emoji".to_owned(), FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .push("noto_emoji".to_owned());
        bold_list.push("noto_emoji".to_owned());
    }

    fonts.families.insert(bold_family, bold_list);
    ctx.set_fonts(fonts);
}

impl SpotiFrameApp {
    fn new(cc: &eframe::CreationContext<'_>, start_fullscreen: bool) -> Self {
        setup_custom_fonts(&cc.egui_ctx);

        let idle_gif = art::GifPlayer::from_file(&cc.egui_ctx, "assets/idle.gif");
        let status = Arc::new(Mutex::new(AppStatus::LoggedOut));
        let mut client_id_input = String::new();
        let mut cmd_tx = None;
        
        let logo_tex = image::load_from_memory(include_bytes!("../assets/icon.png"))
            .ok()
            .map(|img| {
                let rgba = img.into_rgba8();
                let (w, h) = rgba.dimensions();
                let color_img =
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                cc.egui_ctx.load_texture(
                    "app_logo",
                    color_img,
                    egui::TextureOptions::LINEAR,
                )
            });

        if let Some(session) = spotify::load_session() {
            client_id_input = session.client_id.clone();
            *status.lock().unwrap() = AppStatus::Authenticating;
            let st_clone = Arc::clone(&status);
            let ctx = cc.egui_ctx.clone();
            let (tx, rx) = mpsc::channel();
            cmd_tx = Some(tx);

            std::thread::spawn(move || {
                let client = reqwest::blocking::Client::new();
                match spotify::refresh_access_token(
                    &client,
                    &session.client_id,
                    &session.refresh_token,
                ) {
                    Ok((at, rt, exp)) => {
                        *st_clone.lock().unwrap() = AppStatus::Connected {
                            track: None,
                            error: None,
                        };
                        spotify::spawn_poller(session.client_id, at, rt, exp, st_clone, rx, ctx);
                    }
                    Err(_) => {
                        *st_clone.lock().unwrap() = AppStatus::LoggedOut;
                        ctx.request_repaint();
                    }
                }
            });
        }

        Self {
            client_id_input,
            login_error: None,
            status,
            cmd_tx,
            loaded_track_id: String::new(),
            cover_tex: None,
            bg_tex: None,
            logo_tex,
            idle_gif,
            last_active_idx: None,
            is_fullscreen: start_fullscreen,
            was_maximized_before_fs: false,
            needs_initial_fullscreen: start_fullscreen,
            last_window_pos: None,
        }
    }

    fn start_login(&mut self, ctx: &egui::Context) {
        let client_id = self.client_id_input.trim().to_string();
        if client_id.is_empty() {
            self.login_error = Some("Please enter your Spotify Client ID.".to_string());
            return;
        }

        self.login_error = None;
        *self.status.lock().unwrap() = AppStatus::Authenticating;

        let st_clone = Arc::clone(&self.status);
        let ctx_clone = ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.cmd_tx = Some(tx);

        std::thread::spawn(move || match spotify::authenticate_pkce(&client_id) {
            Ok((at, rt, exp)) => {
                *st_clone.lock().unwrap() = AppStatus::Connected {
                    track: None,
                    error: None,
                };
                spotify::spawn_poller(client_id, at, rt, exp, st_clone, rx, ctx_clone);
            }
            Err(e) => {
                *st_clone.lock().unwrap() = AppStatus::Connected {
                    track: None,
                    error: Some(e),
                };
                ctx_clone.request_repaint();
            }
        });
    }

    fn set_fullscreen(&mut self, ctx: &egui::Context, enable: bool) {
        if enable {
            let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            self.was_maximized_before_fs = is_max;
            if is_max {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            }
            self.is_fullscreen = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        } else {
            self.is_fullscreen = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            if self.was_maximized_before_fs {
                self.was_maximized_before_fs = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
        }
    }
}

impl eframe::App for SpotiFrameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let is_minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        if !is_minimized {
            if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
                if rect.min.x > -10000.0 && rect.min.y > -10000.0 {
                    self.last_window_pos = Some(rect.min);
                }
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::F11) || i.key_pressed(egui::Key::F)) {
            self.set_fullscreen(ctx, !self.is_fullscreen);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.is_fullscreen {
            self.set_fullscreen(ctx, false);
        }

        if self.needs_initial_fullscreen {
            self.needs_initial_fullscreen = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }

        let snapshot = {
            let guard = self.status.lock().unwrap();
            match &*guard {
                AppStatus::LoggedOut => None,
                AppStatus::Authenticating => Some(Err("Signing in with Spotify...".to_string())),
                AppStatus::Connected { track, error } => Some(Ok((track.clone(), error.clone()))),
            }
        };

        match snapshot {
            None => {
                if ui::draw_login_screen(
                    ctx,
                    &mut self.client_id_input,
                    self.login_error.as_deref(),
                    "http://127.0.0.1:8888/callback",
                    self.logo_tex.as_ref(),
                ) {
                    self.start_login(ctx);
                }
            }
            Some(Err(msg)) => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::from_rgb(16, 16, 20)))
                    .show(ctx, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new(msg)
                                    .font(FontId::proportional(22.0))
                                    .color(Color32::WHITE),
                            );
                        });
                    });
            }
            Some(Ok((Some(track), _))) => {
                if track.track_id != self.loaded_track_id {
                    self.loaded_track_id = track.track_id.clone();
                    self.last_active_idx = None;
                    if let Some(bytes) = &track.cover_bytes {
                        if let Some((c, b)) = art::load_cover_and_backdrop(ctx, bytes) {
                            self.cover_tex = Some(c);
                            self.bg_tex = Some(b);
                        }
                    } else {
                        self.cover_tex = None;
                        self.bg_tex = None;
                    }
                }

                if let Some(action) = ui::draw_fullscreen_lyrics(
                    ctx,
                    &track,
                    self.cover_tex.as_ref(),
                    self.bg_tex.as_ref(),
                    &mut self.last_active_idx,
                    self.is_fullscreen,
                ) {
                    match action {
                        ui::UiAction::ToggleFullscreen => {
                            self.set_fullscreen(ctx, !self.is_fullscreen);
                        }
                        ui::UiAction::Previous => {
                            if let Some(tx) = &self.cmd_tx {
                                let _ = tx.send(PlayerCommand::Previous);
                            }
                        }
                        ui::UiAction::TogglePlayPause => {
                            if let Some(tx) = &self.cmd_tx {
                                let _ = tx.send(PlayerCommand::TogglePlayPause(track.is_playing));
                            }
                        }
                        ui::UiAction::Next => {
                            if let Some(tx) = &self.cmd_tx {
                                let _ = tx.send(PlayerCommand::Next);
                            }
                        }
                        ui::UiAction::CycleRepeat => {
                            if let Some(tx) = &self.cmd_tx {
                                let _ = tx.send(PlayerCommand::CycleRepeat(
                                    track.repeat_state.clone(),
                                ));
                            }
                        }
                        ui::UiAction::Seek(ms) => {
                            if let Some(tx) = &self.cmd_tx {
                                let _ = tx.send(PlayerCommand::Seek(ms));
                            }
                        }
                    }
                }
            }
            Some(Ok((None, error))) => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::BLACK))
                    .show(ctx, |ui| {
                        ui.centered_and_justified(|ui| {
                            if let Some(err) = error {
                                ui.label(
                                    RichText::new(err)
                                        .font(FontId::proportional(20.0))
                                        .color(Color32::LIGHT_RED),
                                );
                            } else if let Some(gif) = &mut self.idle_gif {
                                let tex = gif.current_texture(ctx);
                                let max_size = ui.available_size() * 0.65;
                                ui.add(
                                    egui::Image::new(tex)
                                        .max_size(max_size)
                                        .rounding(egui::Rounding::same(12.0)),
                                );
                            } else {
                                ui.label(
                                    RichText::new(
                                        "Nothing playing on Spotify. Start a song on any device.",
                                    )
                                    .font(FontId::proportional(22.0))
                                    .color(Color32::from_white_alpha(180)),
                                );
                            }
                        });
                    });
            }
        }

        ui::draw_custom_titlebar(ctx, self.is_fullscreen);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(pos) = self.last_window_pos {
            save_window_pos(pos);
        }
    }
}

fn main() -> eframe::Result<()> {
    let start_fullscreen = std::env::args().any(|arg| {
        matches!(
            arg.to_lowercase().as_str(),
            "-fullscreen" | "--fullscreen" | "-f"
        )
    });

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("SpotiFrame")
        .with_inner_size([1280.0, 760.0])
        .with_decorations(false)
        .with_icon(load_app_icon());

    if let Some(pos) = load_window_pos() {
        viewport = viewport.with_position(pos);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "SpotiFrame",
        options,
        Box::new(move |cc| Ok(Box::new(SpotiFrameApp::new(cc, start_fullscreen)))),
    )
}

fn load_app_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/icon.png");
    let image = image::load_from_memory(bytes)
        .expect("Failed to load assets/icon.png")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}