use std::error::Error;
use std::sync::Arc;

use arboard::Clipboard;
use glyphon::{
    Attrs, Buffer, Cache, Color, Cursor as CosmicCursor, Family, FontSystem, Metrics, Resolution,
    Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Wrap,
};
use mdedit_core::{
    Affinity, Anchor, DeleteDirection, Movement, SelectionRange, SelectionSet, TextRange, TextSize,
};
use mdedit_input::{EditorInput, EditorSession};
use wgpu::{
    CommandEncoderDescriptor, CompositeAlphaMode, DeviceDescriptor, Instance, InstanceDescriptor,
    LoadOp, MultisampleState, Operations, PresentMode, RenderPassColorAttachment,
    RenderPassDescriptor, RequestAdapterOptions, SurfaceColorSpace, SurfaceConfiguration,
    TextureFormat, TextureUsages, TextureViewDescriptor,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, Ime, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

const TEXT_LEFT: f32 = 24.0;
const TEXT_TOP: f32 = 24.0;
const FONT_SIZE: f32 = 22.0;
const LINE_HEIGHT: f32 = 31.0;

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = Application::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct Application {
    state: Option<WindowState>,
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("mdedit IME Lab")
            .with_inner_size(LogicalSize::new(960.0, 680.0));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("create IME lab window"),
        );
        window.set_ime_allowed(true);

        self.state = Some(pollster::block_on(WindowState::new(window, event_loop)));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::Focused(focused) => {
                state.window.set_ime_allowed(focused);
                state
                    .session
                    .handle(EditorInput::Focused(focused))
                    .expect("focus input");
                state.request_redraw();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                state.modifiers = modifiers.state();
            }
            WindowEvent::Ime(ime) => {
                state.handle_ime(ime);
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                state.handle_key(event.logical_key.as_ref(), event.text.as_deref());
            }
            WindowEvent::CursorMoved { position, .. } => {
                state.cursor_position = position;
                if state.dragging {
                    state.extend_drag_selection();
                }
            }
            WindowEvent::MouseInput {
                state: button_state,
                button: MouseButton::Left,
                ..
            } => match button_state {
                ElementState::Pressed => state.begin_drag_selection(),
                ElementState::Released => state.dragging = false,
            },
            WindowEvent::RedrawRequested => {
                if let Err(error) = state.render() {
                    eprintln!("render error: {error}");
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }
}

struct WindowState {
    instance: Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: SurfaceConfiguration,

    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_buffer: Buffer,
    caret_buffer: Buffer,

    session: EditorSession,
    clipboard: Option<Clipboard>,
    modifiers: ModifiersState,
    cursor_position: PhysicalPosition<f64>,
    drag_anchor: Option<Anchor>,
    dragging: bool,

    display_text: String,
    caret_xy: (f32, f32),

    // The window is intentionally last so the surface is dropped first.
    window: Arc<Window>,
}

impl WindowState {
    async fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> Self {
        let physical_size = window.inner_size();

        let instance = Instance::new(InstanceDescriptor::new_with_display_handle(Box::new(
            event_loop.owned_display_handle(),
        )));
        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .expect("request graphics adapter");
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default())
            .await
            .expect("request graphics device");

        let surface = instance
            .create_surface(window.clone())
            .expect("create window surface");
        let format = TextureFormat::Bgra8UnormSrgb;
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: physical_size.width.max(1),
            height: physical_size.height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &surface_config);

        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, format);
        let text_renderer =
            TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);

        let mut text_buffer = Buffer::new(&mut font_system, Metrics::new(FONT_SIZE, LINE_HEIGHT));
        text_buffer.set_wrap(Wrap::Word);
        text_buffer.set_size(
            Some(surface_config.width as f32 - TEXT_LEFT * 2.0),
            Some(surface_config.height as f32 - TEXT_TOP * 2.0),
        );

        let mut caret_buffer = Buffer::new(&mut font_system, Metrics::new(FONT_SIZE, LINE_HEIGHT));
        caret_buffer.set_wrap(Wrap::None);
        caret_buffer.set_size(Some(24.0), Some(LINE_HEIGHT));
        caret_buffer.set_text(
            "│",
            &Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
            None,
        );
        caret_buffer.shape_until_scroll(&mut font_system, false);

        let mut state = Self {
            instance,
            device,
            queue,
            surface,
            surface_config,
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            text_buffer,
            caret_buffer,
            session: EditorSession::new(
                "IME Lab\n\n한글 / 日本語 / 中文 / English / العربية\n\n여기에 입력해 보세요.",
            )
            .expect("create editor session"),
            clipboard: Clipboard::new().ok(),
            modifiers: ModifiersState::empty(),
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            drag_anchor: None,
            dragging: false,
            display_text: String::new(),
            caret_xy: (0.0, 0.0),
            window,
        };

        let end = state
            .session
            .document()
            .len()
            .expect("document length");
        state
            .session
            .set_caret(Anchor::new(end, Affinity::After));
        state.refresh_layout();
        state
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
        self.text_buffer.set_size(
            Some(size.width as f32 - TEXT_LEFT * 2.0),
            Some(size.height as f32 - TEXT_TOP * 2.0),
        );
        self.request_redraw();
    }

    fn handle_ime(&mut self, ime: Ime) {
        let input = match ime {
            Ime::Enabled => EditorInput::ImeEnabled,
            Ime::Preedit(text, selection) => EditorInput::ImePreedit {
                text,
                selection: selection.map(|(start, end)| start..end),
            },
            Ime::Commit(text) => EditorInput::ImeCommit(text),
            Ime::Disabled => EditorInput::ImeDisabled,
        };

        if let Err(error) = self.session.handle(input) {
            eprintln!("IME input error: {error}");
        }
        self.request_redraw();
    }

    fn handle_key(&mut self, key: Key<&str>, text: Option<&str>) {
        let shortcut = self.modifiers.control_key() || self.modifiers.super_key();
        let extend = self.modifiers.shift_key();

        let handled = if shortcut {
            self.handle_shortcut(key)
        } else {
            match key {
                Key::Named(NamedKey::ArrowLeft) => {
                    self.move_cursor(Movement::GraphemeBackward, extend);
                    true
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.move_cursor(Movement::GraphemeForward, extend);
                    true
                }
                Key::Named(NamedKey::Home) => {
                    self.move_cursor(Movement::LineStart, extend);
                    true
                }
                Key::Named(NamedKey::End) => {
                    self.move_cursor(Movement::LineEnd, extend);
                    true
                }
                Key::Named(NamedKey::Backspace) => {
                    self.delete(DeleteDirection::Backward);
                    true
                }
                Key::Named(NamedKey::Delete) => {
                    self.delete(DeleteDirection::Forward);
                    true
                }
                Key::Named(NamedKey::Enter) => {
                    self.insert_text("\n");
                    true
                }
                Key::Named(NamedKey::Tab) => {
                    self.insert_text("\t");
                    true
                }
                _ => false,
            }
        };

        if !handled
            && !self.modifiers.control_key()
            && !self.modifiers.super_key()
            && !self.modifiers.alt_key()
        {
            if let Some(text) = text.filter(|text| !text.chars().all(char::is_control)) {
                self.insert_text(text);
            }
        }
    }

    fn handle_shortcut(&mut self, key: Key<&str>) -> bool {
        let Key::Character(character) = key else {
            return false;
        };

        if character.eq_ignore_ascii_case("z") {
            let result = if self.modifiers.shift_key() {
                self.session.redo()
            } else {
                self.session.undo()
            };
            if let Err(error) = result {
                eprintln!("history error: {error}");
            }
            self.request_redraw();
            return true;
        }

        if character.eq_ignore_ascii_case("y") {
            if let Err(error) = self.session.redo() {
                eprintln!("redo error: {error}");
            }
            self.request_redraw();
            return true;
        }

        if character.eq_ignore_ascii_case("a") {
            let end = self
                .session
                .document()
                .len()
                .expect("document length");
            let selection = SelectionSet::new(
                vec![SelectionRange {
                    anchor: Anchor::new(TextSize::ZERO, Affinity::Before),
                    head: Anchor::new(end, Affinity::After),
                }],
                0,
            )
            .expect("select all");
            self.session.set_selection(selection);
            self.request_redraw();
            return true;
        }

        if character.eq_ignore_ascii_case("c") {
            self.copy_selection(false);
            return true;
        }

        if character.eq_ignore_ascii_case("x") {
            self.copy_selection(true);
            return true;
        }

        if character.eq_ignore_ascii_case("v") {
            if let Some(clipboard) = self.clipboard.as_mut() {
                match clipboard.get_text() {
                    Ok(text) => self.insert_text(&text),
                    Err(error) => eprintln!("clipboard paste error: {error}"),
                }
            }
            return true;
        }

        false
    }

    fn copy_selection(&mut self, cut: bool) {
        let selection = self.session.selections().primary();
        let (start, end) = selection.ordered_offsets();
        if start == end {
            return;
        }

        let range = TextRange::new(start, end).expect("ordered selection");
        let Ok(text) = self.session.document().slice(range) else {
            return;
        };

        if let Some(clipboard) = self.clipboard.as_mut() {
            if let Err(error) = clipboard.set_text(text) {
                eprintln!("clipboard copy error: {error}");
                return;
            }
        }

        if cut {
            self.delete(DeleteDirection::Backward);
        }
    }

    fn insert_text(&mut self, text: &str) {
        if let Err(error) = self.session.insert_text(text) {
            eprintln!("insert error: {error}");
        }
        self.request_redraw();
    }

    fn move_cursor(&mut self, movement: Movement, extend: bool) {
        if let Err(error) = self.session.move_selection(movement, extend) {
            eprintln!("movement error: {error}");
        }
        self.request_redraw();
    }

    fn delete(&mut self, direction: DeleteDirection) {
        if let Err(error) = self.session.delete(direction) {
            eprintln!("delete error: {error}");
        }
        self.request_redraw();
    }

    fn begin_drag_selection(&mut self) {
        let Some(anchor) = self.hit_test_source_anchor() else {
            return;
        };
        self.drag_anchor = Some(anchor);
        self.dragging = true;
        self.session.set_caret(anchor);
        self.request_redraw();
    }

    fn extend_drag_selection(&mut self) {
        let Some(drag_anchor) = self.drag_anchor else {
            return;
        };
        let Some(head) = self.hit_test_source_anchor() else {
            return;
        };

        let selection = SelectionSet::new(
            vec![SelectionRange {
                anchor: drag_anchor,
                head,
            }],
            0,
        )
        .expect("drag selection");
        self.session.set_selection(selection);
        self.request_redraw();
    }

    fn hit_test_source_anchor(&mut self) -> Option<Anchor> {
        let x = self.cursor_position.x as f32 - TEXT_LEFT;
        let y = self.cursor_position.y as f32 - TEXT_TOP;
        if x < 0.0 || y < 0.0 {
            return None;
        }

        self.text_buffer
            .shape_until_scroll(&mut self.font_system, false);
        let cursor = self.text_buffer.hit(x, y)?;
        let display_offset = cursor_to_display_offset(&self.display_text, cursor)?;
        let display_size = TextSize::try_from_usize(display_offset).ok()?;

        let source_offset = self
            .session
            .composition()
            .map_or(Some(display_size), |composition| {
                composition
                    .display_to_source(display_size, Affinity::After)
                    .ok()
            })?;

        Some(Anchor::new(source_offset, Affinity::After))
    }

    fn refresh_layout(&mut self) {
        self.display_text = match self.session.display_text() {
            Ok(text) => text,
            Err(error) => {
                eprintln!("display projection error: {error}");
                self.session.document().text()
            }
        };

        let normal = Attrs::new()
            .family(Family::SansSerif)
            .color(Color::rgb(210, 214, 220));
        let selected = normal.clone().color(Color::rgb(255, 255, 255));
        let preedit = normal.clone().color(Color::rgb(255, 214, 102));

        if let Some(composition) = self.session.composition() {
            let range = composition.display_preedit_range();
            let mut spans = Vec::new();
            if range.start > 0 {
                spans.push((&self.display_text[..range.start], normal.clone()));
            }
            if range.start < range.end {
                spans.push((&self.display_text[range.clone()], preedit));
            }
            if range.end < self.display_text.len() {
                spans.push((&self.display_text[range.end..], normal.clone()));
            }
            if spans.is_empty() {
                spans.push(("", normal.clone()));
            }
            self.text_buffer
                .set_rich_text(spans, &normal, Shaping::Advanced, None);
        } else {
            let mut spans = Vec::new();
            let mut cursor = 0usize;

            for selection in self.session.selections().ranges() {
                let (start, end) = selection.ordered_offsets();
                let start = start.to_usize().min(self.display_text.len());
                let end = end.to_usize().min(self.display_text.len());

                if cursor < start {
                    spans.push((&self.display_text[cursor..start], normal.clone()));
                }
                if start < end {
                    spans.push((&self.display_text[start..end], selected.clone()));
                }
                cursor = cursor.max(end);
            }

            if cursor < self.display_text.len() {
                spans.push((&self.display_text[cursor..], normal.clone()));
            }
            if spans.is_empty() {
                spans.push((&self.display_text[..], normal.clone()));
            }

            self.text_buffer
                .set_rich_text(spans, &normal, Shaping::Advanced, None);
        }

        self.text_buffer
            .shape_until_scroll(&mut self.font_system, false);

        let display_caret = self
            .session
            .composition()
            .and_then(|composition| composition.display_cursor_offset().ok())
            .unwrap_or_else(|| self.session.selections().primary().head.offset);
        let cursor = display_offset_to_cursor(&self.display_text, display_caret.to_usize());

        self.caret_xy = self
            .text_buffer
            .cursor_position(&cursor)
            .unwrap_or((0.0, 0.0));

        self.window.set_ime_cursor_area(
            PhysicalPosition::new(
                (TEXT_LEFT + self.caret_xy.0).round() as i32,
                (TEXT_TOP + self.caret_xy.1).round() as i32,
            ),
            PhysicalSize::new(2_u32, LINE_HEIGHT.ceil() as u32),
        );

        let composition_label = self
            .session
            .composition()
            .map_or("none", |composition| {
                if composition.preedit().is_empty() {
                    "empty-preedit"
                } else {
                    "preedit"
                }
            });
        self.window.set_title(&format!(
            "mdedit IME Lab | source={} bytes | composition={composition_label}",
            self.session.document().text().len()
        ));
    }

    fn render(&mut self) -> Result<(), Box<dyn Error>> {
        self.refresh_layout();

        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.surface_config.width,
                height: self.surface_config.height,
            },
        );

        let text_bounds = TextBounds {
            left: TEXT_LEFT as i32,
            top: TEXT_TOP as i32,
            right: self.surface_config.width as i32 - TEXT_LEFT as i32,
            bottom: self.surface_config.height as i32 - TEXT_TOP as i32,
        };

        let mut areas = vec![TextArea {
            buffer: &self.text_buffer,
            left: TEXT_LEFT,
            top: TEXT_TOP,
            scale: 1.0,
            bounds: text_bounds,
            default_color: Color::rgb(210, 214, 220),
            custom_glyphs: &[],
        }];

        if self.session.focused() && self.session.selections().primary().is_caret() {
            areas.push(TextArea {
                buffer: &self.caret_buffer,
                left: TEXT_LEFT + self.caret_xy.0 - 2.0,
                top: TEXT_TOP + self.caret_xy.1,
                scale: 1.0,
                bounds: text_bounds,
                default_color: Color::rgb(255, 255, 255),
                custom_glyphs: &[],
            });
        }

        self.text_renderer.prepare(
            &self.device,
            &self.queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash_cache,
        )?;

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
                self.surface.configure(&self.device, &self.surface_config);
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone())?;
                self.surface.configure(&self.device, &self.surface_config);
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("wgpu surface validation error".into());
            }
        };

        let view = frame
            .texture
            .create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("mdedit-ime-lab"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.055,
                            g: 0.065,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut pass)?;
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        self.atlas.trim();
        Ok(())
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

fn display_offset_to_cursor(text: &str, offset: usize) -> CosmicCursor {
    let offset = offset.min(text.len());
    let mut line = 0usize;
    let mut line_start = 0usize;

    for (index, byte) in text.as_bytes()[..offset].iter().enumerate() {
        if *byte == b'\n' {
            line += 1;
            line_start = index + 1;
        }
    }

    CosmicCursor::new(line, offset.saturating_sub(line_start))
}

fn cursor_to_display_offset(text: &str, cursor: CosmicCursor) -> Option<usize> {
    let mut line = 0usize;
    let mut line_start = 0usize;

    if cursor.line == 0 {
        return Some(cursor.index.min(text.len()));
    }

    for (index, byte) in text.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            line += 1;
            line_start = index + 1;
            if line == cursor.line {
                let offset = line_start.checked_add(cursor.index)?;
                return Some(offset.min(text.len()));
            }
        }
    }

    None
}
