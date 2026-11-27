use egui::{ColorImage, Context, TextureHandle, TextureOptions};

use std::io::Cursor;
use std::time::{Duration, Instant};

use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;

pub fn load_cover_and_backdrop(
    ctx: &Context,
    bytes: &[u8],
) -> Option<(TextureHandle, TextureHandle)> {
    let img = image::load_from_memory(bytes).ok()?;

    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let sharp = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    let cover_tex = ctx.load_texture("cover", sharp, TextureOptions::LINEAR);

    let small = img.thumbnail(128, 128).blur(14.0).to_rgba8();
    let (bw, bh) = small.dimensions();
    let blurred = ColorImage::from_rgba_unmultiplied([bw as usize, bh as usize], &small);
    let bg_tex = ctx.load_texture("backdrop", blurred, TextureOptions::LINEAR);

    Some((cover_tex, bg_tex))
}

pub struct GifPlayer {
    frames: Vec<(TextureHandle, Duration)>,
    current: usize,
    last_frame_at: Instant,
}

impl GifPlayer {
    pub fn from_file(ctx: &Context, path: &str) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        let decoder = GifDecoder::new(Cursor::new(bytes)).ok()?;
        let raw_frames = decoder.into_frames().collect_frames().ok()?;

        let mut frames = Vec::with_capacity(raw_frames.len());
        for (i, frame) in raw_frames.into_iter().enumerate() {
            let raw_delay = Duration::from(frame.delay());
            let delay = if raw_delay.is_zero() {
                Duration::from_millis(100)
            } else {
                raw_delay.max(Duration::from_millis(20))
            };

            let rgba = frame.into_buffer();
            let (w, h) = rgba.dimensions();
            let color_img = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let tex = ctx.load_texture(
                format!("idle_gif_{i}"),
                color_img,
                TextureOptions::LINEAR,
            );
            frames.push((tex, delay));
        }

        if frames.is_empty() {
            return None;
        }

        Some(Self {
            frames,
            current: 0,
            last_frame_at: Instant::now(),
        })
    }

    /// Advances the animation if needed and returns the current frame texture.
    pub fn current_texture(&mut self, ctx: &Context) -> &TextureHandle {
        let (_, delay) = &self.frames[self.current];
        let elapsed = self.last_frame_at.elapsed();

        if elapsed >= *delay {
            self.current = (self.current + 1) % self.frames.len();
            self.last_frame_at = Instant::now();
            ctx.request_repaint_after(self.frames[self.current].1);
        } else {
            ctx.request_repaint_after(*delay - elapsed);
        }

        &self.frames[self.current].0
    }
}