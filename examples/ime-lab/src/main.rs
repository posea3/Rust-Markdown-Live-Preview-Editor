mod geometry;

use std::{error::Error, ops::Range, sync::Arc};

use arboard::Clipboard;
use geometry::{RectRenderer, ScreenRect};
use glyphon::{
    Attrs, Buffer, Cache, Color, Cursor as CosmicCursor, Family, FontSystem, Metrics,
    Motion as CosmicMotion, Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport, Wrap,
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
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

const TEXT_LEFT: f32 = 24.0;
const TEXT_TOP: f32 = 24.0;
const FONT_SIZE: f32 = 22.0;
const LINE_HEIGHT: f32 = 31.0;
const CARET_WIDTH: f32 = 2.0;
const SELECTION_COLOR: [f32; 4] = [0.18, 0.38, 0.72, 0.55];
const CARET_COLOR: [f32; 4] = [0.96, 0.97, 0.99, 1.0];

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

        let state = pollster::block_on(WindowState::new(window, event_loop));
        state.window.request_redraw();
        self.state = Some(state);
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
            WindowEvent::Focused(focused) => state.set_focused(focused),
            WindowEvent::ModifiersChanged(modifiers) => {
                state.modifiers = modifiers.state();
            }
            WindowEvent::Ime(ime) => state.handle_ime(ime),
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
            WindowEvent::MouseWheel { delta, .. } => state.scroll(delta),
            WindowEvent::RedrawRequested => {
                if let Err(error) = state.render() {
                    eprintln!("render error: {error}");
                }
            }
            _ => {}
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
    rect_renderer: RectRenderer,
    text_buffer: Buffer,

    session: EditorSession,
    clipboard: Option<Clipboard>,
    modifiers: ModifiersState,
    cursor_position: PhysicalPosition<f64>,
    drag_anchor: Option<Anchor>,
    dragging: bool,

    display_text: String,
    preedit_range: Option<Range<usize>>,
    caret_xy: (f32, f32),
    caret_height: f32,
    preferred_x: Option<f32>,
    ensure_caret_visible: bool,

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
        let rect_renderer = RectRenderer::new(&device, format);

        let mut text_buffer = Buffer::new(&mut font_system, Metrics::new(FONT_SIZE, LINE_HEIGHT));
        text_buffer.set_wrap(Wrap::Word);
        text_buffer.set_size(
            Some(surface_config.width as f32 - TEXT_LEFT * 2.0),
            Some(surface_config.height as f32 - TEXT_TOP * 2.0),
        );

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
            rect_renderer,
            text_buffer,
            session: EditorSession::new(
                "IME Lab\n\n한글 / 日本語 / 中文 / English / العربية\n\n여기에 입력해 보세요.\n\n스크롤 테스트 01\n스크롤 테스트 02\n스크롤 테스트 03\n스크롤 테스트 04\n스크롤 테스트 05\n스크롤 테스트 06\n스크롤 테스트 07\n스크롤 테스트 08\n스크롤 테스트 09\n스크롤 테스트 10\n스크롤 테스트 11\n스크롤 테스트 12\n스크롤 테스트 13\n스크롤 테스트 14\n스크롤 테스트 15\n스크롤 테스트 16\n스크롤 테스트 17\n스크롤 테스트 18\n스크롤 테스트 19\n스크롤 테스트 20\n스크롤 테스트 21\n스크롤 테스트 22\n스크롤 테스트 23\n스크롤 테스트 24",
            )
            .expect("create editor session"),
            clipboard: Clipboard::new().ok(),
            modifiers: ModifiersState::empty(),
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            drag_anchor: None,
            dragging: false,
            display_text: String::new(),
            preedit_range: None,
            caret_xy: (0.0, 0.0),
            caret_height: LINE_HEIGHT,
            preferred_x: None,
            ensure_caret_visible: true,
            window,
        };

        let end = state.session.document().len().expect("document length");
        state.session.set_caret(Anchor::new(end, Affinity::After));
        state.refresh_layout();
        state
    }

    fn set_focused(&mut self, focused: bool) {
        self.window.set_ime_allowed(focused);
        if let Err(error) = self.session.handle(EditorInput::Focused(focused)) {
            eprintln!("focus input error: {error}");
        }
        self.preferred_x = None;
        self.ensure_caret_visible = focused;
        self.request_redraw();
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
        self.text_buffer.set_size(
            Some((size.width as f32 - TEXT_LEFT * 2.0).max(1.0)),
            Some((size.height as f32 - TEXT_TOP * 2.0).max(1.0)),
        );
        self.ensure_caret_visible = true;
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
        self.preferred_x = None;
        self.ensure_caret_visible = true;
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
                    self.move_cursor_horizontal_visual(-1, extend);
                    true
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.move_cursor_horizontal_visual(1, extend);
                    true
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.move_cursor_vertical(-1, extend);
                    true
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.move_cursor_vertical(1, extend);
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
            && let Some(text) = text.filter(|text| !text.chars().all(char::is_control))
        {
            self.insert_text(text);
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
            self.after_caret_action();
            return true;
        }

        if character.eq_ignore_ascii_case("y") {
            if let Err(error) = self.session.redo() {
                eprintln!("redo error: {error}");
            }
            self.after_caret_action();
            return true;
        }

        if character.eq_ignore_ascii_case("a") {
            let end = self.session.document().len().expect("document length");
            let selection = SelectionSet::new(
                vec![SelectionRange {
                    anchor: Anchor::new(TextSize::ZERO, Affinity::Before),
                    head: Anchor::new(end, Affinity::After),
                }],
                0,
            )
            .expect("select all");
            self.session.set_selection(selection);
            self.after_caret_action();
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

        if let Some(clipboard) = self.clipboard.as_mut()
            && let Err(error) = clipboard.set_text(text)
        {
            eprintln!("clipboard copy error: {error}");
            return;
        }

        if cut {
            self.delete(DeleteDirection::Backward);
        }
    }

    fn insert_text(&mut self, text: &str) {
        if let Err(error) = self.session.insert_text(text) {
            eprintln!("insert error: {error}");
        }
        self.after_caret_action();
    }

    fn move_cursor(&mut self, movement: Movement, extend: bool) {
        if let Err(error) = self.session.move_selection(movement, extend) {
            eprintln!("movement error: {error}");
        }
        self.after_caret_action();
    }

    fn move_cursor_horizontal_visual(&mut self, direction: i32, extend: bool) {
        self.session.cancel_composition();
        self.ensure_caret_visible = true;
        self.refresh_layout();

        let primary = self.session.selections().primary();
        let cursor = display_anchor_to_cursor(&self.display_text, primary.head);
        self.text_buffer
            .shape_until_cursor(&mut self.font_system, cursor, false);

        let target = visual_horizontal_target(
            &mut self.text_buffer,
            &mut self.font_system,
            cursor,
            direction,
        );

        let Some(target_cursor) = target else {
            return;
        };
        let Some(target_offset) = cursor_to_display_offset(&self.display_text, target_cursor)
        else {
            return;
        };
        let Ok(target_offset) = TextSize::try_from_usize(target_offset) else {
            return;
        };

        let head = Anchor::new(target_offset, cosmic_to_core_affinity(target_cursor.affinity));
        let selection = if extend {
            SelectionRange {
                anchor: primary.anchor,
                head,
            }
        } else {
            SelectionRange::caret(head)
        };

        match SelectionSet::new(vec![selection], 0) {
            Ok(selection) => self.session.set_selection(selection),
            Err(error) => {
                eprintln!("horizontal selection error: {error}");
                return;
            }
        }

        self.preferred_x = None;
        self.ensure_caret_visible = true;
        self.request_redraw();
    }

    fn move_cursor_vertical(&mut self, direction: i32, extend: bool) {
        self.session.cancel_composition();
        self.ensure_caret_visible = true;
        self.refresh_layout();

        let primary = self.session.selections().primary();
        let cursor = display_anchor_to_cursor(&self.display_text, primary.head);
        self.text_buffer
            .shape_until_cursor(&mut self.font_system, cursor, false);

        let Some((current_x, current_top, current_height)) =
            self.text_buffer.layout_runs().find_map(|run| {
                run.cursor_position(&cursor)
                    .map(|x| (x, run.line_top, run.line_height))
            })
        else {
            return;
        };

        let preferred_x = self.preferred_x.unwrap_or(current_x);
        self.preferred_x = Some(preferred_x);

        let target_y =
            current_top + current_height * 0.5 + direction as f32 * current_height.max(1.0);
        let Some(target_cursor) = self.text_buffer.hit(preferred_x, target_y) else {
            return;
        };
        let Some(target_offset) = cursor_to_display_offset(&self.display_text, target_cursor)
        else {
            return;
        };
        let Ok(target_offset) = TextSize::try_from_usize(target_offset) else {
            return;
        };

        let head = Anchor::new(target_offset, cosmic_to_core_affinity(target_cursor.affinity));
        let selection = if extend {
            SelectionRange {
                anchor: primary.anchor,
                head,
            }
        } else {
            SelectionRange::caret(head)
        };

        match SelectionSet::new(vec![selection], 0) {
            Ok(selection) => self.session.set_selection(selection),
            Err(error) => {
                eprintln!("vertical selection error: {error}");
                return;
            }
        }

        self.ensure_caret_visible = true;
        self.request_redraw();
    }

    fn delete(&mut self, direction: DeleteDirection) {
        if let Err(error) = self.session.delete(direction) {
            eprintln!("delete error: {error}");
        }
        self.after_caret_action();
    }

    fn after_caret_action(&mut self) {
        self.preferred_x = None;
        self.ensure_caret_visible = true;
        self.request_redraw();
    }

    fn scroll(&mut self, delta: MouseScrollDelta) {
        let pixels = match delta {
            MouseScrollDelta::LineDelta(_, y) => -y * LINE_HEIGHT * 3.0,
            MouseScrollDelta::PixelDelta(position) => -position.y as f32,
        };

        if pixels.abs() < f32::EPSILON {
            return;
        }

        let mut scroll = self.text_buffer.scroll();
        scroll.vertical += pixels;
        self.text_buffer.set_scroll(scroll);
        self.text_buffer
            .shape_until_scroll(&mut self.font_system, false);
        self.ensure_caret_visible = false;
        self.request_redraw();
    }

    fn begin_drag_selection(&mut self) {
        let Some(anchor) = self.hit_test_source_anchor() else {
            return;
        };
        self.drag_anchor = Some(anchor);
        self.dragging = true;
        self.session.set_caret(anchor);
        self.preferred_x = None;
        self.ensure_caret_visible = false;
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
        self.preferred_x = None;
        self.ensure_caret_visible = false;
        self.request_redraw();
    }

    fn hit_test_source_anchor(&mut self) -> Option<Anchor> {
        self.refresh_layout();

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

        let source_offset =
            self.session
                .composition()
                .map_or(Some(display_size), |composition| {
                    composition
                        .display_to_source(display_size, Affinity::After)
                        .ok()
                })?;

        Some(Anchor::new(source_offset, Affinity::After))
    }

    fn refresh_layout(&mut self) {
        let next_display_text = match self.session.display_text() {
            Ok(text) => text,
            Err(error) => {
                eprintln!("display projection error: {error}");
                self.session.document().text()
            }
        };
        let next_preedit_range = self
            .session
            .composition()
            .map(|composition| composition.display_preedit_range());

        if next_display_text != self.display_text || next_preedit_range != self.preedit_range {
            let old_scroll = self.text_buffer.scroll();
            self.display_text = next_display_text;
            self.preedit_range = next_preedit_range;

            let normal = Attrs::new()
                .family(Family::SansSerif)
                .color(Color::rgb(210, 214, 220));

            if let Some(range) = self.preedit_range.clone() {
                let preedit = normal.clone().color(Color::rgb(255, 214, 102));
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
                self.text_buffer
                    .set_text(&self.display_text, &normal, Shaping::Advanced, None);
            }

            self.text_buffer.set_scroll(old_scroll);
        }

        let display_caret = self
            .session
            .composition()
            .and_then(|composition| composition.display_cursor_offset().ok())
            .unwrap_or_else(|| self.session.selections().primary().head.offset);
        let cursor = display_offset_to_cursor(&self.display_text, display_caret.to_usize());

        if self.ensure_caret_visible {
            self.text_buffer
                .shape_until_cursor(&mut self.font_system, cursor, false);
            self.ensure_caret_visible = false;
        } else {
            self.text_buffer
                .shape_until_scroll(&mut self.font_system, false);
        }

        if let Some((x, top, height)) = self.text_buffer.layout_runs().find_map(|run| {
            run.cursor_position(&cursor)
                .map(|x| (x, run.line_top, run.line_height))
        }) {
            self.caret_xy = (x, top);
            self.caret_height = height;
        }

        self.window.set_ime_cursor_area(
            PhysicalPosition::new(
                (TEXT_LEFT + self.caret_xy.0).round() as i32,
                (TEXT_TOP + self.caret_xy.1).round() as i32,
            ),
            PhysicalSize::new(CARET_WIDTH.ceil() as u32, self.caret_height.ceil() as u32),
        );

        let composition_label = self.session.composition().map_or("none", |composition| {
            if composition.preedit().is_empty() {
                "empty-preedit"
            } else {
                "preedit"
            }
        });
        let scroll = self.text_buffer.scroll();
        self.window.set_title(&format!(
            "mdedit IME Lab | source={} bytes | composition={composition_label} | scroll={}:{:.0}",
            self.session.document().text().len(),
            scroll.line,
            scroll.vertical
        ));
    }

    fn selection_rectangles(&self) -> Vec<ScreenRect> {
        if self.session.composition().is_some() {
            return Vec::new();
        }

        let selections = self
            .session
            .selections()
            .ranges()
            .iter()
            .filter_map(|selection| {
                let (start, end) = selection.ordered_offsets();
                (start != end).then(|| {
                    (
                        display_offset_to_cursor(&self.display_text, start.to_usize()),
                        display_offset_to_cursor(&self.display_text, end.to_usize()),
                    )
                })
            })
            .collect::<Vec<_>>();

        let mut rects = Vec::new();
        for run in self.text_buffer.layout_runs() {
            for (start, end) in &selections {
                for (x, width) in run.highlight(*start, *end) {
                    rects.push(ScreenRect::new(
                        TEXT_LEFT + x,
                        TEXT_TOP + run.line_top,
                        width.max(1.0),
                        run.line_height,
                        SELECTION_COLOR,
                    ));
                }
            }
        }
        rects
    }

    fn caret_rectangles(&self) -> Vec<ScreenRect> {
        let show_caret = self.session.focused()
            && (self.session.composition().is_some()
                || self.session.selections().primary().is_caret());

        if show_caret {
            vec![ScreenRect::new(
                TEXT_LEFT + self.caret_xy.0,
                TEXT_TOP + self.caret_xy.1,
                CARET_WIDTH,
                self.caret_height,
                CARET_COLOR,
            )]
        } else {
            Vec::new()
        }
    }

    fn text_clip_rect(&self) -> ScreenRect {
        ScreenRect::new(
            TEXT_LEFT,
            TEXT_TOP,
            (self.surface_config.width as f32 - TEXT_LEFT * 2.0).max(1.0),
            (self.surface_config.height as f32 - TEXT_TOP * 2.0).max(1.0),
            [0.0; 4],
        )
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

        self.text_renderer.prepare(
            &self.device,
            &self.queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            [TextArea {
                buffer: &self.text_buffer,
                left: TEXT_LEFT,
                top: TEXT_TOP,
                scale: 1.0,
                bounds: text_bounds,
                default_color: Color::rgb(210, 214, 220),
                custom_glyphs: &[],
            }],
            &mut self.swash_cache,
        )?;

        let selection_rects = self.selection_rectangles();
        let caret_rects = self.caret_rectangles();
        let clip = self.text_clip_rect();
        let selection_batch = self.rect_renderer.prepare(
            &self.device,
            &selection_rects,
            self.surface_config.width,
            self.surface_config.height,
            clip,
        );
        let caret_batch = self.rect_renderer.prepare(
            &self.device,
            &caret_rects,
            self.surface_config.width,
            self.surface_config.height,
            clip,
        );

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
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

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("mdedit selection background"),
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

            if let Some(batch) = selection_batch.as_ref() {
                self.rect_renderer.render(&mut pass, batch);
            }
        }

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("mdedit text"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
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

        if let Some(batch) = caret_batch.as_ref() {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("mdedit caret"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.rect_renderer.render(&mut pass, batch);
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

fn visual_horizontal_target(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    cursor: CosmicCursor,
    direction: i32,
) -> Option<CosmicCursor> {
    let (current_x, current_top, current_height) = buffer
        .layout_runs()
        .find_map(|run| {
            run.cursor_position(&cursor)
                .map(|x| (x, run.line_top, run.line_height))
        })?;

    let y = current_top + current_height * 0.5;
    let width = buffer.size().0.unwrap_or(4096.0).max(1.0);
    let max_steps = width.ceil() as usize + 4;

    for step in 1..=max_steps {
        let x = current_x + direction as f32 * step as f32;
        if x < -2.0 || x > width + 2.0 {
            break;
        }

        if let Some(candidate) = buffer.hit(x, y)
            && !same_cosmic_cursor(candidate, cursor)
        {
            return Some(candidate);
        }
    }

    let motion = if direction < 0 {
        CosmicMotion::Left
    } else {
        CosmicMotion::Right
    };
    buffer
        .cursor_motion(font_system, cursor, None, motion)
        .map(|(cursor, _)| cursor)
}

fn same_cosmic_cursor(left: CosmicCursor, right: CosmicCursor) -> bool {
    left.line == right.line && left.index == right.index && left.affinity == right.affinity
}

fn display_anchor_to_cursor(text: &str, anchor: Anchor) -> CosmicCursor {
    let cursor = display_offset_to_cursor(text, anchor.offset.to_usize());
    CosmicCursor::new_with_affinity(
        cursor.line,
        cursor.index,
        match anchor.affinity {
            Affinity::Before => glyphon::Affinity::Before,
            Affinity::After => glyphon::Affinity::After,
        },
    )
}

fn cosmic_to_core_affinity(affinity: glyphon::Affinity) -> Affinity {
    match affinity {
        glyphon::Affinity::Before => Affinity::Before,
        glyphon::Affinity::After => Affinity::After,
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

    if cursor.line == 0 {
        return Some(cursor.index.min(text.len()));
    }

    for (index, byte) in text.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            line += 1;
            if line == cursor.line {
                let line_start = index + 1;
                let offset = line_start.checked_add(cursor.index)?;
                return Some(offset.min(text.len()));
            }
        }
    }

    None
}
