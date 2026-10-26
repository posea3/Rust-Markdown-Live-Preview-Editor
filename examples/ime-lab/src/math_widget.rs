use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use latex_rust::{MathFont, PngOptions, latex_to_png};
use wgpu::{Device, Queue};
use winit::window::Window;

use crate::image_widget::{ImageDrawRequest, ImageWidgetHost};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathHostState {
    Missing,
    Loading,
    Ready,
    Failed,
}

pub struct MathWidgetHost {
    pending: HashSet<String>,
    ready: HashSet<String>,
    failed: HashMap<String, String>,
    sender: Sender<RenderedMath>,
    receiver: Receiver<RenderedMath>,
    wake_window: Arc<Window>,
}

impl MathWidgetHost {
    pub fn new(wake_window: Arc<Window>) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            pending: HashSet::new(),
            ready: HashSet::new(),
            failed: HashMap::new(),
            sender,
            receiver,
            wake_window,
        }
    }

    pub fn request(&mut self, tex: &str) {
        if tex.trim().is_empty()
            || self.pending.contains(tex)
            || self.ready.contains(tex)
            || self.failed.contains_key(tex)
        {
            return;
        }

        self.pending.insert(tex.to_owned());

        let sender = self.sender.clone();
        let tex = tex.to_owned();
        let wake_window = self.wake_window.clone();
        thread::spawn(move || {
            let message = match render_display_math_rgba(&tex) {
                Ok((width, height, rgba)) => RenderedMath::Ready {
                    tex,
                    width,
                    height,
                    rgba,
                },
                Err(error) => RenderedMath::Failed { tex, error },
            };
            let _ = sender.send(message);
            wake_window.request_redraw();
        });
    }

    pub fn drain_ready(&mut self, textures: &mut ImageWidgetHost, device: &Device, queue: &Queue) {
        while let Ok(rendered) = self.receiver.try_recv() {
            match rendered {
                RenderedMath::Ready {
                    tex,
                    width,
                    height,
                    rgba,
                } => {
                    self.pending.remove(&tex);
                    let result = textures.upload_rgba(
                        device,
                        queue,
                        math_texture_key(&tex),
                        width,
                        height,
                        &rgba,
                    );
                    match result {
                        Ok(()) => {
                            self.failed.remove(&tex);
                            self.ready.insert(tex);
                        }
                        Err(error) => {
                            self.ready.remove(&tex);
                            eprintln!(
                                "[mdedit-ime-lab] math widget upload failed {tex:?}: {error}"
                            );
                            self.failed.insert(tex, error);
                        }
                    }
                }
                RenderedMath::Failed { tex, error } => {
                    self.pending.remove(&tex);
                    eprintln!("[mdedit-ime-lab] math widget render failed {tex:?}: {error}");
                    self.failed.insert(tex, error);
                }
            }
        }
    }

    pub fn state(&self, tex: &str, textures: &ImageWidgetHost) -> MathHostState {
        if self.ready.contains(tex) && textures.contains_texture(&math_texture_key(tex)) {
            MathHostState::Ready
        } else if self.pending.contains(tex) {
            MathHostState::Loading
        } else if self.failed.contains_key(tex) {
            MathHostState::Failed
        } else {
            MathHostState::Missing
        }
    }

    pub fn intrinsic_size(&self, tex: &str, textures: &ImageWidgetHost) -> Option<(u32, u32)> {
        textures.intrinsic_size(&math_texture_key(tex))
    }

    pub fn draw_request(&self, tex: &str, rect: crate::geometry::ScreenRect) -> ImageDrawRequest {
        ImageDrawRequest::new(math_texture_key(tex), rect)
    }
}

enum RenderedMath {
    Ready {
        tex: String,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    Failed {
        tex: String,
        error: String,
    },
}

fn math_texture_key(tex: &str) -> String {
    format!("mdedit-math-display:{tex}")
}

fn render_display_math_rgba(tex: &str) -> Result<(u32, u32, Vec<u8>), String> {
    let font = MathFont::stix_two_math().map_err(|error| format!("math font: {error}"))?;
    let png = latex_to_png(tex, &font, &PngOptions::new())
        .map_err(|error| format!("math render: {error}"))?;
    let decoded =
        image::load_from_memory(&png).map_err(|error| format!("math PNG decode: {error}"))?;
    let mut rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();

    if width == 0 || height == 0 {
        return Err("math renderer produced zero-sized output".to_owned());
    }

    for pixel in rgba.pixels_mut() {
        if pixel.0[3] != 0 {
            pixel.0[0] = 232;
            pixel.0[1] = 234;
            pixel.0[2] = 240;
        }
    }

    Ok((width, height, rgba.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_math_renders_to_non_empty_rgba() {
        let (width, height, rgba) =
            render_display_math_rgba(r"\frac{1}{2} + \sqrt{x}").expect("render math");

        assert!(width > 0);
        assert!(height > 0);
        assert_eq!(rgba.len(), width as usize * height as usize * 4);
        assert!(rgba.chunks_exact(4).any(|pixel| pixel[3] != 0));
    }

    #[test]
    fn invalid_math_fails_without_fake_render() {
        assert!(render_display_math_rgba(r"\definitely_unknown_command{x}").is_err());
    }

    #[test]
    fn math_texture_keys_are_namespaced() {
        assert_eq!(
            math_texture_key("x^2"),
            "mdedit-math-display:x^2".to_owned()
        );
    }
}
