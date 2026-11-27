use egui::{
    Align, Align2, Color32, CursorIcon, FontFamily, FontId, Pos2, Rect, RichText, Rounding,
    ScrollArea, Sense, Shadow, Shape, Stroke, Vec2,
};

use crate::state::TrackState;

const SPOTIFY_GREEN: Color32 = Color32::from_rgb(30, 215, 96);

pub enum UiAction {
    ToggleFullscreen,
    Previous,
    TogglePlayPause,
    Next,
    CycleRepeat,
    Seek(u64),
}

fn bold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Bold".into()))
}

fn format_time(ms: u64) -> String {
    let total_sec = ms / 1000;
    format!("{}:{:02}", total_sec / 60, total_sec % 60)
}

pub fn draw_fullscreen_lyrics(
    ctx: &egui::Context,
    track: &TrackState,
    cover_tex: Option<&egui::TextureHandle>,
    bg_tex: Option<&egui::TextureHandle>,
    last_active_idx: &mut Option<usize>,
    is_fullscreen: bool,
) -> Option<UiAction> {
    let mut action: Option<UiAction> = None;

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(Color32::from_rgb(18, 16, 15)))
        .show(ctx, |ui| {
            let full_rect = ui.max_rect();

            // 1. Blurred cover-art backdrop + dark scrim
            if let Some(bg) = bg_tex {
                ui.painter().image(
                    bg.id(),
                    full_rect,
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                ui.painter().rect_filled(
                    full_rect,
                    Rounding::ZERO,
                    Color32::from_black_alpha(150),
                );
            }

            // 2. Centered two-column layout
            let art_size = (full_rect.height() * 0.50).clamp(280.0, 440.0);
            let gap = 64.0;
            let lyrics_width = (full_rect.width() * 0.40).clamp(400.0, 620.0);
            let total_group_w = art_size + gap + lyrics_width;
            let group_left = full_rect.center().x - total_group_w * 0.5;

            // Measure wrapped title & artist height dynamically
            let title_galley = ui.painter().layout(
                track.title.clone(),
                bold_font(20.0),
                Color32::WHITE,
                art_size,
            );
            let artist_galley = ui.painter().layout(
                track.artist.clone(),
                FontId::proportional(14.0),
                Color32::from_white_alpha(180),
                art_size,
            );
            let title_h = title_galley.size().y;
            let artist_h = artist_galley.size().y;
            let meta_gap = 4.0;
            let meta_h = title_h + meta_gap + artist_h;

            // Left block: Cover + Title/Artist + Seekbar + Controls (vertically centered)
            let seekbar_h = 24.0;
            let controls_h = 44.0;
            let total_left_h = art_size + 16.0 + meta_h + 16.0 + seekbar_h + 14.0 + controls_h;
            let art_top = full_rect.center().y - total_left_h * 0.5;
            let art_rect =
                Rect::from_min_size(Pos2::new(group_left, art_top), Vec2::splat(art_size));

            // Soft shadow + Album Cover
            let shadow = Shadow {
                offset: Vec2::new(0.0, 16.0),
                blur: 40.0,
                spread: 4.0,
                color: Color32::from_black_alpha(155),
            };
            ui.painter()
                .add(shadow.as_shape(art_rect, Rounding::same(8.0)));

            if let Some(cover) = cover_tex {
                ui.put(
                    art_rect,
                    egui::Image::new(cover)
                        .fit_to_exact_size(art_rect.size())
                        .rounding(Rounding::same(8.0)),
                );
            } else {
                ui.painter().rect_filled(
                    art_rect,
                    Rounding::same(8.0),
                    Color32::from_rgb(40, 40, 40),
                );
            }

            // Paint measured Title & Artist
            let title_pos = Pos2::new(group_left, art_rect.max.y + 16.0);
            let artist_pos = Pos2::new(group_left, title_pos.y + title_h + meta_gap);
            ui.painter().galley(title_pos, title_galley, Color32::WHITE);
            ui.painter()
                .galley(artist_pos, artist_galley, Color32::from_white_alpha(180));

            let meta_bottom = artist_pos.y + artist_h;

            // 3. Seekbar (positioned dynamically below the artist text, same width as album cover)
            let seek_y = meta_bottom + 16.0 + seekbar_h * 0.5;
            let pos_ms = track.current_position_ms();
            let dur_ms = track.duration_ms.max(1);
            let frac = (pos_ms as f32 / dur_ms as f32).clamp(0.0, 1.0);

            let time_pad = 36.0;
            let bar_left = group_left + time_pad;
            let bar_right = group_left + art_size - time_pad;
            let bar_w = (bar_right - bar_left).max(60.0);
            let track_rect =
                Rect::from_min_size(Pos2::new(bar_left, seek_y - 2.0), Vec2::new(bar_w, 4.0));

            ui.painter().text(
                Pos2::new(group_left, seek_y),
                Align2::LEFT_CENTER,
                format_time(pos_ms),
                FontId::proportional(11.5),
                Color32::from_white_alpha(170),
            );
            ui.painter().text(
                Pos2::new(group_left + art_size, seek_y),
                Align2::RIGHT_CENTER,
                format_time(track.duration_ms),
                FontId::proportional(11.5),
                Color32::from_white_alpha(170),
            );

            let hit_rect = track_rect.expand2(Vec2::new(0.0, 9.0));
            let seek_resp = ui.interact(hit_rect, ui.id().with("seekbar"), Sense::click_and_drag());
            let hovered = seek_resp.hovered() || seek_resp.dragged();

            ui.painter().rect_filled(
                track_rect,
                Rounding::same(2.0),
                Color32::from_white_alpha(70),
            );
            let fill_rect = Rect::from_min_size(
                track_rect.min,
                Vec2::new(bar_w * frac, track_rect.height()),
            );
            ui.painter().rect_filled(
                fill_rect,
                Rounding::same(2.0),
                if hovered { SPOTIFY_GREEN } else { Color32::WHITE },
            );

            if hovered {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                ui.painter()
                    .circle_filled(Pos2::new(fill_rect.max.x, seek_y), 5.5, Color32::WHITE);
            }
            if seek_resp.clicked() || seek_resp.drag_stopped() {
                if let Some(ptr) = seek_resp.interact_pointer_pos() {
                    let ratio = ((ptr.x - track_rect.min.x) / bar_w).clamp(0.0, 1.0);
                    action = Some(UiAction::Seek((ratio * dur_ms as f32) as u64));
                }
            }

            // 4. Controls row: Fullscreen | Previous | Play/Pause | Next | Repeat
            let ctrl_center_y = seek_y + seekbar_h * 0.5 + 14.0 + controls_h * 0.5;
            let center_x = group_left + art_size * 0.5;
            let btn_spacing = 46.0;

            // Fullscreen button (replaces shuffle)
            let fs_center = Pos2::new(center_x - btn_spacing * 2.0, ctrl_center_y);
            let fs_resp = ui.interact(
                Rect::from_center_size(fs_center, Vec2::splat(32.0)),
                ui.id().with("btn_fullscreen"),
                Sense::click(),
            );
            let fs_col = if is_fullscreen {
                SPOTIFY_GREEN
            } else if fs_resp.hovered() {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(175)
            };
            draw_fullscreen_icon(ui.painter(), fs_center, fs_col);
            if fs_resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if fs_resp.clicked() {
                action = Some(UiAction::ToggleFullscreen);
            }

            // Previous button
            let prev_center = Pos2::new(center_x - btn_spacing, ctrl_center_y);
            let prev_resp = ui.interact(
                Rect::from_center_size(prev_center, Vec2::splat(32.0)),
                ui.id().with("btn_prev"),
                Sense::click(),
            );
            let prev_col = if prev_resp.hovered() {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(190)
            };
            draw_skip_icon(ui.painter(), prev_center, prev_col, false);
            if prev_resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if prev_resp.clicked() {
                action = Some(UiAction::Previous);
            }

            // Play / Pause circular button
            let play_center = Pos2::new(center_x, ctrl_center_y);
            let play_resp = ui.interact(
                Rect::from_center_size(play_center, Vec2::splat(42.0)),
                ui.id().with("btn_play_pause"),
                Sense::click(),
            );
            let radius = if play_resp.hovered() { 20.0 } else { 18.5 };
            ui.painter()
                .circle_filled(play_center, radius, Color32::WHITE);
            draw_play_pause_icon(
                ui.painter(),
                play_center,
                Color32::from_rgb(16, 16, 16),
                track.is_playing,
            );
            if play_resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if play_resp.clicked() {
                action = Some(UiAction::TogglePlayPause);
            }

            // Next button
            let next_center = Pos2::new(center_x + btn_spacing, ctrl_center_y);
            let next_resp = ui.interact(
                Rect::from_center_size(next_center, Vec2::splat(32.0)),
                ui.id().with("btn_next"),
                Sense::click(),
            );
            let next_col = if next_resp.hovered() {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(190)
            };
            draw_skip_icon(ui.painter(), next_center, next_col, true);
            if next_resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if next_resp.clicked() {
                action = Some(UiAction::Next);
            }

            // Repeat button
            let rep_center = Pos2::new(center_x + btn_spacing * 2.0, ctrl_center_y);
            let rep_resp = ui.interact(
                Rect::from_center_size(rep_center, Vec2::splat(32.0)),
                ui.id().with("btn_repeat"),
                Sense::click(),
            );
            let rep_active = track.repeat_state != "off";
            let rep_col = if rep_active {
                SPOTIFY_GREEN
            } else if rep_resp.hovered() {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(175)
            };
            draw_repeat_icon(
                ui.painter(),
                rep_center,
                rep_col,
                track.repeat_state == "track",
            );
            if rep_active {
                ui.painter().circle_filled(
                    Pos2::new(rep_center.x, rep_center.y + 13.0),
                    2.2,
                    SPOTIFY_GREEN,
                );
            }
            if rep_resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if rep_resp.clicked() {
                action = Some(UiAction::CycleRepeat);
            }

            // 5. Right column: Synced clickable lyrics
            let right_rect = Rect::from_min_max(
                Pos2::new(art_rect.max.x + gap, full_rect.min.y + 56.0),
                Pos2::new(art_rect.max.x + gap + lyrics_width, full_rect.max.y - 56.0),
            );

            let active_idx = track.active_line_index();
            let line_changed = active_idx != *last_active_idx;
            if line_changed {
                *last_active_idx = active_idx;
            }

            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right_rect), |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        if !track.lyrics.is_empty() {
                            ui.add_space(right_rect.height() * 0.30);

                            for (idx, line) in track.lyrics.iter().enumerate() {
                                let is_active = Some(idx) == active_idx;
                                let (font_size, color) = if is_active {
                                    (32.0, SPOTIFY_GREEN)
                                } else {
                                    (28.0, Color32::from_white_alpha(115))
                                };

                                let label = egui::Label::new(
                                    RichText::new(&line.text)
                                        .font(bold_font(font_size))
                                        .color(color),
                                )
                                .wrap()
                                .sense(Sense::click());

                                let resp = ui.add(label);
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                }
                                if resp.clicked() {
                                    action = Some(UiAction::Seek(line.time_ms));
                                }
                                if is_active && line_changed {
                                    resp.scroll_to_me(Some(Align::Center));
                                }
                                ui.add_space(28.0);
                            }

                            ui.add_space(right_rect.height() * 0.45);
                        } else if let Some(plain) = &track.plain_lyrics {
                            ui.add_space(32.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(plain)
                                        .font(bold_font(26.0))
                                        .color(Color32::from_white_alpha(180)),
                                )
                                .wrap(),
                            );
                        } else {
                            ui.add_space(right_rect.height() * 0.42);
                            ui.label(
                                RichText::new("♪")
                                    .font(bold_font(44.0))
                                    .color(Color32::from_white_alpha(140)),
                            );
                        }
                    });
            });
        });

    if track.is_playing {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }

    action
}

// --- Vector Icon Helpers ---

fn draw_fullscreen_icon(painter: &egui::Painter, c: Pos2, color: Color32) {
    let s = Stroke::new(1.8_f32, color);
    let r = 6.5;
    let arm = 4.0;
    // Top-right corner
    painter.line_segment(
        [Pos2::new(c.x + r - arm, c.y - r), Pos2::new(c.x + r, c.y - r)],
        s,
    );
    painter.line_segment(
        [Pos2::new(c.x + r, c.y - r), Pos2::new(c.x + r, c.y - r + arm)],
        s,
    );
    painter.line_segment([Pos2::new(c.x + r, c.y - r), Pos2::new(c.x + 1.5, c.y - 1.5)], s);
    // Bottom-left corner
    painter.line_segment(
        [Pos2::new(c.x - r + arm, c.y + r), Pos2::new(c.x - r, c.y + r)],
        s,
    );
    painter.line_segment(
        [Pos2::new(c.x - r, c.y + r), Pos2::new(c.x - r, c.y + r - arm)],
        s,
    );
    painter.line_segment([Pos2::new(c.x - r, c.y + r), Pos2::new(c.x - 1.5, c.y + 1.5)], s);
}

fn draw_skip_icon(painter: &egui::Painter, c: Pos2, color: Color32, is_next: bool) {
    let dir = if is_next { 1.0 } else { -1.0 };
    let pts = vec![
        Pos2::new(c.x - 5.5 * dir, c.y - 6.5),
        Pos2::new(c.x - 5.5 * dir, c.y + 6.5),
        Pos2::new(c.x + 4.5 * dir, c.y),
    ];
    painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
    let bar_x = c.x + 6.0 * dir;
    painter.rect_filled(
        Rect::from_center_size(Pos2::new(bar_x, c.y), Vec2::new(2.2, 13.0)),
        Rounding::same(1.0),
        color,
    );
}

fn draw_play_pause_icon(painter: &egui::Painter, c: Pos2, color: Color32, is_playing: bool) {
    if is_playing {
        painter.rect_filled(
            Rect::from_center_size(Pos2::new(c.x - 3.6, c.y), Vec2::new(3.4, 13.5)),
            Rounding::same(1.0),
            color,
        );
        painter.rect_filled(
            Rect::from_center_size(Pos2::new(c.x + 3.6, c.y), Vec2::new(3.4, 13.5)),
            Rounding::same(1.0),
            color,
        );
    } else {
        let pts = vec![
            Pos2::new(c.x - 4.5, c.y - 7.0),
            Pos2::new(c.x - 4.5, c.y + 7.0),
            Pos2::new(c.x + 7.0, c.y),
        ];
        painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
    }
}

fn draw_repeat_icon(painter: &egui::Painter, c: Pos2, color: Color32, is_track: bool) {
    let s = Stroke::new(1.8_f32, color);
    let rect = Rect::from_center_size(c, Vec2::new(14.0, 11.0));
    painter.rect_stroke(rect, Rounding::same(3.5), s);
    let tip = Pos2::new(c.x - 2.0, rect.max.y);
    painter.line_segment([tip, Pos2::new(tip.x + 3.0, tip.y - 2.8)], s);
    painter.line_segment([tip, Pos2::new(tip.x + 3.0, tip.y + 2.8)], s);

    if is_track {
        painter.text(c, Align2::CENTER_CENTER, "1", FontId::proportional(8.5), color);
    }
}

pub fn draw_custom_titlebar(ctx: &egui::Context, is_fullscreen: bool) {
    if is_fullscreen {
        return;
    }

    let screen_rect = ctx.screen_rect();
    let bar_h = 36.0;
    let btn_w = 44.0;
    let controls_w = btn_w * 3.0;
    let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

    egui::Area::new(egui::Id::new("custom_titlebar"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let bar_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(screen_rect.width(), bar_h));
            ui.allocate_rect(bar_rect, Sense::hover());

            // 1. Draggable empty area (everything left of the 3 window buttons)
            let drag_rect = Rect::from_min_max(
                bar_rect.min,
                Pos2::new(bar_rect.max.x - controls_w, bar_rect.max.y),
            );
            let drag_resp = ui.interact(
                drag_rect,
                ui.id().with("titlebar_drag"),
                Sense::click_and_drag(),
            );
            if drag_resp.drag_started_by(egui::PointerButton::Primary) {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag_resp.double_clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
            }

            // 2. Window control buttons (Minimize, Maximize/Restore, Close)
            let painter = ui.painter();
            let icon_stroke = Stroke::new(1.3_f32, Color32::from_white_alpha(200));

            // Minimize
            let min_rect = Rect::from_min_size(
                Pos2::new(bar_rect.max.x - btn_w * 3.0, 0.0),
                Vec2::new(btn_w, bar_h),
            );
            let min_resp = ui.interact(min_rect, ui.id().with("btn_min"), Sense::click());
            if min_resp.hovered() {
                painter.rect_filled(min_rect, Rounding::ZERO, Color32::from_white_alpha(25));
                ctx.set_cursor_icon(CursorIcon::PointingHand);
            }
            let mc = min_rect.center();
            painter.line_segment(
                [Pos2::new(mc.x - 5.0, mc.y), Pos2::new(mc.x + 5.0, mc.y)],
                icon_stroke,
            );
            if min_resp.clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }

            // Maximize / Restore
            let max_rect = Rect::from_min_size(
                Pos2::new(bar_rect.max.x - btn_w * 2.0, 0.0),
                Vec2::new(btn_w, bar_h),
            );
            let max_resp = ui.interact(max_rect, ui.id().with("btn_max"), Sense::click());
            if max_resp.hovered() {
                painter.rect_filled(max_rect, Rounding::ZERO, Color32::from_white_alpha(25));
                ctx.set_cursor_icon(CursorIcon::PointingHand);
            }
            let xc = max_rect.center();
            painter.rect_stroke(
                Rect::from_center_size(xc, Vec2::splat(9.5)),
                Rounding::same(1.0),
                icon_stroke,
            );
            if max_resp.clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
            }

            // Close
            let close_rect = Rect::from_min_size(
                Pos2::new(bar_rect.max.x - btn_w, 0.0),
                Vec2::new(btn_w, bar_h),
            );
            let close_resp = ui.interact(close_rect, ui.id().with("btn_close"), Sense::click());
            if close_resp.hovered() {
                painter.rect_filled(
                    close_rect,
                    Rounding::ZERO,
                    Color32::from_rgba_unmultiplied(232, 17, 35, 210),
                );
                ctx.set_cursor_icon(CursorIcon::PointingHand);
            }
            let cc = close_rect.center();
            let r = 4.5;
            let close_stroke = if close_resp.hovered() {
                Stroke::new(1.4_f32, Color32::WHITE)
            } else {
                icon_stroke
            };
            painter.line_segment(
                [Pos2::new(cc.x - r, cc.y - r), Pos2::new(cc.x + r, cc.y + r)],
                close_stroke,
            );
            painter.line_segment(
                [Pos2::new(cc.x + r, cc.y - r), Pos2::new(cc.x - r, cc.y + r)],
                close_stroke,
            );
            if close_resp.clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
}

pub fn draw_login_screen(
    ctx: &egui::Context,
    client_id: &mut String,
    login_error: Option<&str>,
    redirect_uri: &str,
    logo_tex: Option<&egui::TextureHandle>,
) -> bool {
    let mut should_login = false;

    let bg_color = Color32::from_rgb(18, 21, 22);
    let input_bg = Color32::from_rgb(30, 34, 37);
    let muted_text = Color32::from_rgb(165, 172, 178);
    let badge_bg = Color32::from_rgb(24, 118, 56);
    let has_id = !client_id.trim().is_empty();

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(bg_color))
        .show(ctx, |ui| {
            let full_rect = ui.max_rect();

            let left_w = 340.0;
            let gap = 56.0;
            let right_w = 400.0;
            let total_w = left_w + gap + right_w;
            let content_h = 370.0;

            let start_x = (full_rect.center().x - total_w * 0.5).max(full_rect.min.x + 24.0);
            let start_y = (full_rect.center().y - content_h * 0.5).max(full_rect.min.y + 48.0);

            // --- LEFT COLUMN: Logo, Title, Client ID, Sign-in Button ---
            let left_rect = Rect::from_min_size(
                Pos2::new(start_x, start_y + 20.0),
                Vec2::new(left_w, content_h),
            );

            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(left_rect), |ui| {
                ui.vertical_centered(|ui| {
                    if let Some(tex) = logo_tex {
                        ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::splat(72.0)));
                    }

                    ui.add_space(16.0);
                    ui.label(
                        RichText::new("SpotiFrame")
                            .font(bold_font(32.0))
                            .color(Color32::WHITE),
                    );
                    ui.add_space(22.0);

                    // Rounded dark Client ID input
                    egui::Frame::none()
                        .fill(input_bg)
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(14.0, 11.0))
                        .show(ui, |ui| {
                            ui.set_width(left_w - 28.0);
                            let resp = ui.add(
                                egui::TextEdit::singleline(client_id)
                                    .frame(false)
                                    .desired_width(f32::INFINITY)
                                    .font(FontId::proportional(15.0))
                                    .text_color(Color32::WHITE)
                                    .hint_text(
                                        RichText::new("Client ID")
                                            .font(FontId::proportional(15.0))
                                            .color(Color32::from_rgb(130, 138, 145)),
                                    ),
                            );
                            if resp.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                && has_id
                            {
                                should_login = true;
                            }
                        });

                    ui.add_space(16.0);

                    // Pill-shaped Sign in with Spotify button
                    let btn_w = 270.0;
                    let btn_h = 46.0;
                    let (btn_rect, btn_resp) = ui.allocate_exact_size(
                        Vec2::new(btn_w, btn_h),
                        if has_id { Sense::click() } else { Sense::hover() },
                    );

                    let btn_fill = if !has_id {
                        Color32::from_rgb(24, 72, 42)
                    } else if btn_resp.hovered() {
                        Color32::from_rgb(45, 228, 110)
                    } else {
                        SPOTIFY_GREEN
                    };
                    let btn_text_col = if has_id {
                        Color32::from_rgb(14, 16, 17)
                    } else {
                        Color32::from_rgb(18, 34, 24)
                    };

                    ui.painter()
                        .rect_filled(btn_rect, Rounding::same(btn_h * 0.5), btn_fill);
                    ui.painter().text(
                        btn_rect.center(),
                        Align2::CENTER_CENTER,
                        "Sign in with Spotify",
                        bold_font(16.0),
                        btn_text_col,
                    );

                    if has_id && btn_resp.hovered() {
                        ctx.set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if has_id && btn_resp.clicked() {
                        should_login = true;
                    }

                    ui.add_space(18.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Sign in through your browser. SpotiFrame never sees your\npassword. Playback control needs Spotify Premium.",
                            )
                            .font(FontId::proportional(12.5))
                            .color(muted_text),
                        )
                        .wrap(),
                    );

                    if let Some(err) = login_error {
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(err)
                                .font(FontId::proportional(13.0))
                                .color(Color32::LIGHT_RED),
                        );
                    }
                });
            });

            // --- RIGHT COLUMN: Register SpotiFrame Guide ---
            let right_rect = Rect::from_min_size(
                Pos2::new(start_x + left_w + gap, start_y),
                Vec2::new(right_w, content_h + 40.0),
            );

            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right_rect), |ui| {
                ui.label(
                    RichText::new("Register SpotiFrame")
                        .font(bold_font(21.0))
                        .color(Color32::WHITE),
                );
                ui.add_space(8.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Spotify asks you to register SpotiFrame once, on an account with Premium. It takes about two minutes and gives you a Client ID.",
                        )
                        .font(FontId::proportional(13.5))
                        .color(muted_text),
                    )
                    .wrap(),
                );
                ui.add_space(18.0);

                // Step 1
                ui.horizontal_top(|ui| {
                    draw_step_badge(ui, "1", badge_bg);
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Open Spotify's developer dashboard.")
                                .font(FontId::proportional(14.5))
                                .color(Color32::WHITE),
                        );
                        ui.add_space(4.0);
                        let link = ui.add(
                            egui::Label::new(
                                RichText::new("developer.spotify.com/dashboard")
                                    .font(bold_font(14.0))
                                    .color(SPOTIFY_GREEN),
                            )
                            .sense(Sense::click()),
                        );
                        if link.hovered() {
                            ctx.set_cursor_icon(CursorIcon::PointingHand);
                        }
                        if link.clicked() {
                            ctx.open_url(egui::OpenUrl::new_tab(
                                "https://developer.spotify.com/dashboard",
                            ));
                        }
                    });
                });
                ui.add_space(14.0);

                // Step 2
                ui.horizontal_top(|ui| {
                    draw_step_badge(ui, "2", badge_bg);
                    ui.add_space(10.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Click Create app, Any name and description will do.",
                            )
                            .font(FontId::proportional(14.5))
                            .color(Color32::WHITE),
                        )
                        .wrap(),
                    );
                });
                ui.add_space(14.0);

                // Step 3
                ui.horizontal_top(|ui| {
                    draw_step_badge(ui, "3", badge_bg);
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Under Redirect URIs, add exactly:")
                                .font(FontId::proportional(14.5))
                                .color(Color32::WHITE),
                        );
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            egui::Frame::none()
                                .fill(input_bg)
                                .rounding(Rounding::same(6.0))
                                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(redirect_uri)
                                            .font(FontId::monospace(13.5))
                                            .color(Color32::WHITE),
                                    );
                                });
                            ui.add_space(8.0);

                            let (copy_rect, copy_resp) =
                                ui.allocate_exact_size(Vec2::new(66.0, 32.0), Sense::click());
                            let copy_bg = if copy_resp.hovered() {
                                Color32::from_white_alpha(20)
                            } else {
                                Color32::TRANSPARENT
                            };
                            ui.painter().rect_filled(copy_rect, Rounding::same(16.0), copy_bg);
                            ui.painter().rect_stroke(
                                copy_rect,
                                Rounding::same(16.0),
                                Stroke::new(1.0_f32, Color32::from_rgb(100, 108, 115)),
                            );
                            ui.painter().text(
                                copy_rect.center(),
                                Align2::CENTER_CENTER,
                                "Copy",
                                bold_font(13.0),
                                Color32::WHITE,
                            );
                            if copy_resp.hovered() {
                                ctx.set_cursor_icon(CursorIcon::PointingHand);
                            }
                            if copy_resp.clicked() {
                                ui.output_mut(|o| o.copied_text = redirect_uri.to_string());
                            }
                        });
                    });
                });
                ui.add_space(14.0);

                // Step 4
                ui.horizontal_top(|ui| {
                    draw_step_badge(ui, "4", badge_bg);
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Tick Web API, accept the terms, and save.")
                            .font(FontId::proportional(14.5))
                            .color(Color32::WHITE),
                    );
                });
                ui.add_space(14.0);

                // Step 5
                ui.horizontal_top(|ui| {
                    draw_step_badge(ui, "5", badge_bg);
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Paste the Client ID into the field and sign in.")
                            .font(FontId::proportional(14.5))
                            .color(Color32::WHITE),
                    );
                });
            });
        });

    should_login
}

fn draw_step_badge(ui: &mut egui::Ui, num: &str, bg: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 11.0, bg);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        num,
        bold_font(12.5),
        Color32::from_rgb(130, 235, 165),
    );
}