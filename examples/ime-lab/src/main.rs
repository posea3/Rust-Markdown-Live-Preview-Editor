mod accessibility;
mod geometry;
mod trace_capture;

use std::{env, error::Error, ops::Range, sync::Arc, time::Instant};

use accessibility::{EditorAccessibilityAction, build_tree_update, translate_action};
use accesskit::ActionRequest;
use accesskit_winit::{
    Adapter as AccessKitAdapter, Event as AccessKitEvent, WindowEvent as AccessKitWindowEvent,
};
use arboard::Clipboard;
use cosmic_text::Motion as CosmicMotion;
use geometry::{RectRenderer, ScreenRect};
use glyphon::{
    Attrs, Buffer, Cache, Color, Cursor as CosmicCursor, Family, FontSystem, Metrics, Resolution,
    Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Wrap,
};
use mdedit_core::{
    Affinity, Anchor, DeleteDirection, Movement, Revision, SelectionRange, SelectionSet, TextRange,
    TextSize,
};
use mdedit_input::{EditorInput, EditorSession};
use trace_capture::TraceCapture;
use unicode_segmentation::UnicodeSegmentation;
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
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

const TEXT_LEFT: f32 = 24.0;
const TEXT_TOP: f32 = 24.0;
const FONT_SIZE: f32 = 22.0;
const LINE_HEIGHT: f32 = 31.0;
const CARET_WIDTH: f32 = 2.0;
const DRAG_SCROLL_MARGIN: f32 = 42.0;
const SELECTION_COLOR: [f32; 4] = [0.18, 0.38, 0.72, 0.55];
const CARET_COLOR: [f32; 4] = [0.96, 0.97, 0.99, 1.0];

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::<AccessKitEvent>::with_user_event().build()?;
    let mut app = Application::new(event_loop.create_proxy());
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct Application {
    event_loop_proxy: EventLoopProxy<AccessKitEvent>,
    state: Option<WindowState>,
}

impl Application {
    fn new(event_loop_proxy: EventLoopProxy<AccessKitEvent>) -> Self {
        Self {
            event_loop_proxy,
            state: None,
        }
    }
}

impl ApplicationHandler<AccessKitEvent> for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        eprintln!("[mdedit-ime-lab] creating native window");
        let attributes = Window::default_attributes()
            .with_title("mdedit IME Lab")
            .with_inner_size(LogicalSize::new(960.0, 680.0))
            .with_visible(false);
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("create IME lab window"),
        );
        eprintln!("[mdedit-ime-lab] native window created");
        let accessibility_adapter = AccessKitAdapter::with_event_loop_proxy(
            event_loop,
            &window,
            self.event_loop_proxy.clone(),
        );
        eprintln!("[mdedit-ime-lab] AccessKit adapter created");
        window.set_visible(true);
        window.set_ime_allowed(true);

        let state = pollster::block_on(WindowState::new(window, event_loop, accessibility_adapter));
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

        state
            .accessibility_adapter
            .process_event(&state.window, &event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::Focused(focused) => state.set_focused(focused),
            WindowEvent::ModifiersChanged(modifiers) => {
                state.modifiers = modifiers.state();
            }
            WindowEvent::Ime(ime) => state.handle_ime(ime),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if !event.repeat {
                    state.begin_latency_probe(event.logical_key.as_ref());
                }
                state.handle_key(event.logical_key.as_ref(), event.text.as_deref());
                if !event.repeat {
                    state.mark_latency_input_handled();
                }
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
                state.mark_latency_redraw_received();
                let continue_drag_scroll = state.continue_drag_auto_scroll();
                let render_result = state.render();
                state.finish_latency_probe();
                if let Err(error) = render_result {
                    eprintln!("render error: {error}");
                }
                if continue_drag_scroll {
                    state.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AccessKitEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if event.window_id != state.window.id() {
            return;
        }

        match event.window_event {
            AccessKitWindowEvent::InitialTreeRequested => state.update_accessibility_tree(),
            AccessKitWindowEvent::ActionRequested(request) => {
                state.handle_accessibility_action(request);
            }
            AccessKitWindowEvent::AccessibilityDeactivated => {}
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
    accessibility_adapter: AccessKitAdapter,

    session: EditorSession,
    trace_capture: Option<TraceCapture>,
    clipboard: Option<Clipboard>,
    modifiers: ModifiersState,
    cursor_position: PhysicalPosition<f64>,
    drag_anchor: Option<Anchor>,
    dragging: bool,

    display_text: String,
    display_revision: Option<Revision>,
    preedit_text: Option<String>,
    preedit_range: Option<Range<usize>>,
    caret_xy: (f32, f32),
    caret_height: f32,
    preferred_x: Option<f32>,
    ensure_caret_visible: bool,
    layout_dirty: bool,
    text_render_dirty: bool,
    viewport_dirty: bool,
    window_title: String,
    latency_trace_enabled: bool,
    latency_probe: Option<LatencyProbe>,

    // The window is intentionally last so the surface is dropped first.
    window: Arc<Window>,
}

impl WindowState {
    async fn new(
        window: Arc<Window>,
        event_loop: &ActiveEventLoop,
        accessibility_adapter: AccessKitAdapter,
    ) -> Self {
        let physical_size = window.inner_size();

        eprintln!("[mdedit-ime-lab] wgpu: creating instance");
        let mut instance_descriptor = InstanceDescriptor::new_with_display_handle(Box::new(
            event_loop.owned_display_handle(),
        ));
        apply_platform_backend_default(&mut instance_descriptor);
        let instance_descriptor = instance_descriptor.with_env();
        eprintln!(
            "[mdedit-ime-lab] wgpu: enabled backends={:?}",
            instance_descriptor.backends
        );
        let instance = Instance::new(instance_descriptor);
        eprintln!("[mdedit-ime-lab] wgpu: instance created; requesting adapter");
        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .expect("request graphics adapter");
        let adapter_info = adapter.get_info();
        eprintln!(
            "[mdedit-ime-lab] wgpu: adapter={} backend={:?} device_type={:?}",
            adapter_info.name, adapter_info.backend, adapter_info.device_type
        );
        eprintln!("[mdedit-ime-lab] wgpu: requesting device");
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default())
            .await
            .expect("request graphics device");
        eprintln!("[mdedit-ime-lab] wgpu: device created; creating surface");

        let surface = instance
            .create_surface(window.clone())
            .expect("create window surface");
        eprintln!("[mdedit-ime-lab] wgpu: surface created");
        let capabilities = surface.get_capabilities(&adapter);
        let present_mode = choose_present_mode(&capabilities.present_modes);
        eprintln!(
            "[mdedit-ime-lab] wgpu: supported present modes={:?}; selected={:?}",
            capabilities.present_modes, present_mode
        );

        let format = TextureFormat::Bgra8UnormSrgb;
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: physical_size.width.max(1),
            height: physical_size.height.max(1),
            present_mode,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
            // Text-editor interaction is latency-sensitive. Avoid queueing a second
            // frame behind the compositor regardless of the selected present mode.
            desired_maximum_frame_latency: 1,
            color_space: SurfaceColorSpace::Auto,
        };
        eprintln!("[mdedit-ime-lab] wgpu: configuring surface");
        surface.configure(&device, &surface_config);
        eprintln!("[mdedit-ime-lab] wgpu: surface configured; creating text renderer");

        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, format);
        let text_renderer =
            TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);
        let rect_renderer = RectRenderer::new(&device, format);
        eprintln!("[mdedit-ime-lab] wgpu: render resources created");

        let mut text_buffer = Buffer::new(&mut font_system, Metrics::new(FONT_SIZE, LINE_HEIGHT));
        text_buffer.set_wrap(Wrap::Word);
        text_buffer.set_size(
            Some(surface_config.width as f32 - TEXT_LEFT * 2.0),
            Some(surface_config.height as f32 - TEXT_TOP * 2.0),
        );

        let mut session = EditorSession::new(
            "IME Lab\n\n한글 / 日本語 / 中文 / English / العربية\n\n여기에 입력해 보세요.\n\n스크롤 테스트 01\n스크롤 테스트 02\n스크롤 테스트 03\n스크롤 테스트 04\n스크롤 테스트 05\n스크롤 테스트 06\n스크롤 테스트 07\n스크롤 테스트 08\n스크롤 테스트 09\n스크롤 테스트 10\n스크롤 테스트 11\n스크롤 테스트 12\n스크롤 테스트 13\n스크롤 테스트 14\n스크롤 테스트 15\n스크롤 테스트 16\n스크롤 테스트 17\n스크롤 테스트 18\n스크롤 테스트 19\n스크롤 테스트 20\n스크롤 테스트 21\n스크롤 테스트 22\n스크롤 테스트 23\n스크롤 테스트 24",
        )
        .expect("create editor session");
        let end = session.document().len().expect("document length");
        session.set_caret(Anchor::new(end, Affinity::After));
        let trace_capture = TraceCapture::from_env(&session);

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
            accessibility_adapter,
            session,
            trace_capture,
            clipboard: Clipboard::new().ok(),
            modifiers: ModifiersState::empty(),
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            drag_anchor: None,
            dragging: false,
            display_text: String::new(),
            display_revision: None,
            preedit_text: None,
            preedit_range: None,
            caret_xy: (0.0, 0.0),
            caret_height: LINE_HEIGHT,
            preferred_x: None,
            ensure_caret_visible: true,
            layout_dirty: true,
            text_render_dirty: true,
            viewport_dirty: true,
            window_title: String::new(),
            latency_trace_enabled: env_flag("MDEDIT_LATENCY_TRACE"),
            latency_probe: None,
            window,
        };

        state.refresh_layout();
        eprintln!("[mdedit-ime-lab] initialization complete");
        state
    }

    fn set_focused(&mut self, focused: bool) {
        self.window.set_ime_allowed(focused);
        if let Err(error) = self.apply_input(EditorInput::Focused(focused)) {
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
        self.layout_dirty = true;
        self.text_render_dirty = true;
        self.viewport_dirty = true;
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

        if let Err(error) = self.apply_input(input) {
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
                self.apply_input(EditorInput::Redo)
            } else {
                self.apply_input(EditorInput::Undo)
            };
            if let Err(error) = result {
                eprintln!("history error: {error}");
            }
            self.after_caret_action();
            return true;
        }

        if character.eq_ignore_ascii_case("y") {
            if let Err(error) = self.apply_input(EditorInput::Redo) {
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
            if let Err(error) = self.apply_input(EditorInput::SetSelection(selection)) {
                eprintln!("select all error: {error}");
            }
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
        if let Err(error) = self.apply_input(EditorInput::InsertText(text.to_owned())) {
            eprintln!("insert error: {error}");
        }
        self.after_caret_action();
    }

    fn move_cursor(&mut self, movement: Movement, extend: bool) {
        if let Err(error) = self.apply_input(EditorInput::Move { movement, extend }) {
            eprintln!("movement error: {error}");
        }
        self.after_caret_action();
    }

    fn move_cursor_horizontal_visual(&mut self, direction: i32, extend: bool) {
        self.session.cancel_composition();

        let primary = self.session.selections().primary();
        let cursor = display_anchor_to_cursor(&self.display_text, primary.head);

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

        let head = Anchor::new(
            target_offset,
            cosmic_to_core_affinity(target_cursor.affinity),
        );
        let selection = if extend {
            SelectionRange {
                anchor: primary.anchor,
                head,
            }
        } else {
            SelectionRange::caret(head)
        };

        match SelectionSet::new(vec![selection], 0) {
            Ok(selection) => {
                if let Err(error) = self.apply_input(EditorInput::SetSelection(selection)) {
                    eprintln!("horizontal selection input error: {error}");
                    return;
                }
            }
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

        let head = Anchor::new(
            target_offset,
            cosmic_to_core_affinity(target_cursor.affinity),
        );
        let selection = if extend {
            SelectionRange {
                anchor: primary.anchor,
                head,
            }
        } else {
            SelectionRange::caret(head)
        };

        match SelectionSet::new(vec![selection], 0) {
            Ok(selection) => {
                if let Err(error) = self.apply_input(EditorInput::SetSelection(selection)) {
                    eprintln!("vertical selection input error: {error}");
                    return;
                }
            }
            Err(error) => {
                eprintln!("vertical selection error: {error}");
                return;
            }
        }

        self.ensure_caret_visible = true;
        self.request_redraw();
    }

    fn delete(&mut self, direction: DeleteDirection) {
        if let Err(error) = self.apply_input(EditorInput::Delete(direction)) {
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
        self.layout_dirty = false;
        self.text_render_dirty = true;
        self.ensure_caret_visible = false;
        self.request_redraw();
    }

    fn begin_drag_selection(&mut self) {
        let Some(anchor) = self.hit_test_source_anchor() else {
            return;
        };
        self.drag_anchor = Some(anchor);
        self.dragging = true;
        if let Err(error) = self.apply_input(EditorInput::SetSelection(SelectionSet::caret(anchor)))
        {
            eprintln!("drag caret input error: {error}");
            return;
        }
        self.preferred_x = None;
        self.ensure_caret_visible = false;
        self.request_redraw();
    }

    fn extend_drag_selection(&mut self) {
        self.auto_scroll_drag();
        self.update_drag_selection_from_pointer();
        self.request_redraw();
    }

    fn continue_drag_auto_scroll(&mut self) -> bool {
        if !self.dragging || !self.auto_scroll_drag() {
            return false;
        }

        self.update_drag_selection_from_pointer();
        true
    }

    fn update_drag_selection_from_pointer(&mut self) {
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
        if let Err(error) = self.apply_input(EditorInput::SetSelection(selection)) {
            eprintln!("drag selection input error: {error}");
            return;
        }
        self.preferred_x = None;
        self.ensure_caret_visible = false;
    }

    fn auto_scroll_drag(&mut self) -> bool {
        let pointer_y = self.cursor_position.y as f32;
        let viewport_top = TEXT_TOP;
        let viewport_bottom = self.surface_config.height as f32 - TEXT_TOP;

        let signed_factor = if pointer_y < viewport_top + DRAG_SCROLL_MARGIN {
            -((viewport_top + DRAG_SCROLL_MARGIN - pointer_y) / DRAG_SCROLL_MARGIN)
        } else if pointer_y > viewport_bottom - DRAG_SCROLL_MARGIN {
            (pointer_y - (viewport_bottom - DRAG_SCROLL_MARGIN)) / DRAG_SCROLL_MARGIN
        } else {
            return false;
        };

        let factor = signed_factor.abs().clamp(0.15, 2.5) * signed_factor.signum();
        let pixels = LINE_HEIGHT * 0.35 * factor;
        let before = self.text_buffer.scroll();
        let mut scroll = before;
        scroll.vertical += pixels;
        self.text_buffer.set_scroll(scroll);
        self.text_buffer
            .shape_until_scroll(&mut self.font_system, false);
        self.layout_dirty = false;
        self.text_render_dirty = true;
        self.ensure_caret_visible = false;

        self.text_buffer.scroll() != before
    }

    fn hit_test_source_anchor(&mut self) -> Option<Anchor> {
        self.refresh_layout();

        let width = (self.surface_config.width as f32 - TEXT_LEFT * 2.0).max(1.0);
        let height = (self.surface_config.height as f32 - TEXT_TOP * 2.0).max(1.0);
        let x = (self.cursor_position.x as f32 - TEXT_LEFT).clamp(0.0, width - 0.001);
        let y = (self.cursor_position.y as f32 - TEXT_TOP).clamp(0.0, height - 0.001);

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
        let scroll_before = self.text_buffer.scroll();
        let revision = self.session.document().revision();
        let next_preedit_text = self
            .session
            .composition()
            .map(|composition| composition.preedit());
        let display_changed = self.display_revision != Some(revision)
            || self.preedit_text.as_deref() != next_preedit_text;

        if display_changed {
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
            let old_scroll = self.text_buffer.scroll();

            self.display_text = next_display_text;
            self.display_revision = Some(revision);
            self.preedit_text = next_preedit_text.map(str::to_owned);
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
            self.layout_dirty = true;
            self.text_render_dirty = true;
        }

        let display_caret = self
            .session
            .composition()
            .and_then(|composition| composition.display_cursor_offset().ok())
            .unwrap_or_else(|| self.session.selections().primary().head.offset);
        let cursor = display_offset_to_cursor(&self.display_text, display_caret.to_usize());

        if self.ensure_caret_visible {
            let caret_is_visible =
                !self.layout_dirty && self.text_buffer.cursor_position(&cursor).is_some();
            if !caret_is_visible {
                self.text_buffer
                    .shape_until_cursor(&mut self.font_system, cursor, false);
                self.layout_dirty = false;
            }
            self.ensure_caret_visible = false;
        } else if self.layout_dirty {
            self.text_buffer
                .shape_until_scroll(&mut self.font_system, false);
            self.layout_dirty = false;
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

        let scroll = self.text_buffer.scroll();
        if scroll != scroll_before {
            self.text_render_dirty = true;
        }

        let composition_label = self.session.composition().map_or("none", |composition| {
            if composition.preedit().is_empty() {
                "empty-preedit"
            } else {
                "preedit"
            }
        });
        let source_len = self
            .session
            .document()
            .len()
            .map_or(0, |length| length.to_usize());
        let next_title = format!(
            "mdedit IME Lab | source={source_len} bytes | composition={composition_label} | scroll={}:{:.0}",
            scroll.line, scroll.vertical
        );
        if next_title != self.window_title {
            self.window.set_title(&next_title);
            self.window_title = next_title;
        }
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

    fn update_accessibility_tree(&mut self) {
        let session = &self.session;
        let width = self.surface_config.width;
        let height = self.surface_config.height;
        let scale_factor = self.window.scale_factor();

        self.accessibility_adapter.update_if_active(|| {
            build_tree_update(
                &session.document().text(),
                session.selections().primary(),
                width,
                height,
                scale_factor,
                TEXT_LEFT,
                TEXT_TOP,
            )
        });
    }

    fn handle_accessibility_action(&mut self, request: ActionRequest) {
        let Some(action) = translate_action(request, &self.session.document().text()) else {
            return;
        };

        match action {
            EditorAccessibilityAction::Focus => {
                self.window.focus_window();
                self.window.set_ime_allowed(true);
            }
            EditorAccessibilityAction::SetSelection(selection) => {
                match SelectionSet::new(vec![selection], 0) {
                    Ok(selection) => {
                        if let Err(error) = self.apply_input(EditorInput::SetSelection(selection)) {
                            eprintln!("accessibility selection input error: {error}");
                            return;
                        }
                        self.after_caret_action();
                    }
                    Err(error) => eprintln!("accessibility selection error: {error}"),
                }
            }
            EditorAccessibilityAction::ReplaceSelectedText(text) => {
                self.insert_text(&text);
            }
            EditorAccessibilityAction::SetValue(text) => {
                self.replace_document_text(&text);
            }
        }
    }

    fn replace_document_text(&mut self, text: &str) {
        let end = self.session.document().len().expect("document length");
        let selection = SelectionSet::new(
            vec![SelectionRange {
                anchor: Anchor::new(TextSize::ZERO, Affinity::Before),
                head: Anchor::new(end, Affinity::After),
            }],
            0,
        )
        .expect("full document selection");
        if let Err(error) = self.apply_input(EditorInput::SetSelection(selection)) {
            eprintln!("accessibility full selection input error: {error}");
            return;
        }
        self.insert_text(text);
    }

    fn apply_input(&mut self, input: EditorInput) -> Result<bool, mdedit_input::SessionError> {
        if let Some(capture) = self.trace_capture.as_mut() {
            capture.record(&input);
        }
        self.session.handle(input)
    }

    fn render(&mut self) -> Result<(), Box<dyn Error>> {
        self.refresh_layout();
        self.update_accessibility_tree();

        if self.viewport_dirty {
            self.viewport.update(
                &self.queue,
                Resolution {
                    width: self.surface_config.width,
                    height: self.surface_config.height,
                },
            );
            self.viewport_dirty = false;
        }

        let text_bounds = TextBounds {
            left: TEXT_LEFT as i32,
            top: TEXT_TOP as i32,
            right: self.surface_config.width as i32 - TEXT_LEFT as i32,
            bottom: self.surface_config.height as i32 - TEXT_TOP as i32,
        };

        let text_prepared = self.text_render_dirty;
        if text_prepared {
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
            self.text_render_dirty = false;
        }

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
        if text_prepared {
            self.atlas.trim();
        }
        Ok(())
    }

    fn begin_latency_probe(&mut self, key: Key<&str>) {
        if !self.latency_trace_enabled {
            return;
        }

        let label = match key {
            Key::Named(NamedKey::ArrowLeft) => "arrow-left",
            Key::Named(NamedKey::ArrowRight) => "arrow-right",
            Key::Named(NamedKey::ArrowUp) => "arrow-up",
            Key::Named(NamedKey::ArrowDown) => "arrow-down",
            _ => return,
        };

        self.latency_probe = Some(LatencyProbe {
            label,
            input_received: Instant::now(),
            input_handled: None,
            redraw_received: None,
        });
    }

    fn mark_latency_input_handled(&mut self) {
        if let Some(probe) = self.latency_probe.as_mut() {
            probe.input_handled = Some(Instant::now());
        }
    }

    fn mark_latency_redraw_received(&mut self) {
        if let Some(probe) = self.latency_probe.as_mut() {
            probe.redraw_received = Some(Instant::now());
        }
    }

    fn finish_latency_probe(&mut self) {
        let Some(probe) = self.latency_probe.take() else {
            return;
        };

        let presented = Instant::now();
        let handled = probe.input_handled.unwrap_or(probe.input_received);
        let redraw = probe.redraw_received.unwrap_or(handled);
        eprintln!(
            "[mdedit-latency] {} handle={:.3}ms redraw_wait={:.3}ms render_present={:.3}ms total={:.3}ms",
            probe.label,
            duration_ms(handled.duration_since(probe.input_received)),
            duration_ms(redraw.duration_since(handled)),
            duration_ms(presented.duration_since(redraw)),
            duration_ms(presented.duration_since(probe.input_received)),
        );
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

#[derive(Clone, Copy, Debug)]
struct LatencyProbe {
    label: &'static str,
    input_received: Instant,
    input_handled: Option<Instant>,
    redraw_received: Option<Instant>,
}

fn duration_ms(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn env_flag(name: &str) -> bool {
    env::var(name).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn choose_present_mode(supported: &[PresentMode]) -> PresentMode {
    let requested = env::var("MDEDIT_PRESENT_MODE").ok();
    let Some(requested) = requested.as_deref() else {
        return PresentMode::Fifo;
    };

    let normalized = requested.trim().to_ascii_lowercase();
    let requested_mode = match normalized.as_str() {
        "fifo" => PresentMode::Fifo,
        "auto-vsync" | "autovsync" => PresentMode::AutoVsync,
        "auto-no-vsync" | "autonovsync" => PresentMode::AutoNoVsync,
        "mailbox" => PresentMode::Mailbox,
        "immediate" => PresentMode::Immediate,
        other => {
            eprintln!("[mdedit-ime-lab] unknown MDEDIT_PRESENT_MODE={other:?}; using Fifo");
            return PresentMode::Fifo;
        }
    };

    match requested_mode {
        PresentMode::Mailbox | PresentMode::Immediate | PresentMode::FifoRelaxed
            if !supported.contains(&requested_mode) =>
        {
            eprintln!(
                "[mdedit-ime-lab] requested present mode {requested_mode:?} is unsupported; using AutoNoVsync"
            );
            PresentMode::AutoNoVsync
        }
        _ => requested_mode,
    }
}

#[derive(Clone, Copy, Debug)]
struct VisualCaretCell {
    left: f32,
    right: f32,
    left_cursor: CosmicCursor,
    right_cursor: CosmicCursor,
}

fn visual_horizontal_target(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    cursor: CosmicCursor,
    direction: i32,
) -> Option<CosmicCursor> {
    let target = buffer.layout_runs().find_map(|run| {
        let current_x = run.cursor_position(&cursor)?;
        let mut cells = Vec::with_capacity(run.glyphs.len());

        for glyph in run.glyphs {
            let cluster = &run.text[glyph.start..glyph.end];
            let grapheme_count = cluster.grapheme_indices(true).count().max(1);
            let cell_width = glyph.w / grapheme_count as f32;

            for (ordinal, (relative_start, grapheme)) in cluster.grapheme_indices(true).enumerate()
            {
                let start = glyph.start + relative_start;
                let end = start + grapheme.len();
                let left = glyph.x + cell_width * ordinal as f32;
                let right = left + cell_width;

                let (left_cursor, right_cursor) = if glyph.level.is_rtl() {
                    (
                        CosmicCursor::new_with_affinity(run.line_i, end, glyphon::Affinity::Before),
                        CosmicCursor::new_with_affinity(
                            run.line_i,
                            start,
                            glyphon::Affinity::After,
                        ),
                    )
                } else {
                    (
                        CosmicCursor::new_with_affinity(
                            run.line_i,
                            start,
                            glyphon::Affinity::After,
                        ),
                        CosmicCursor::new_with_affinity(run.line_i, end, glyphon::Affinity::Before),
                    )
                };

                cells.push(VisualCaretCell {
                    left,
                    right,
                    left_cursor,
                    right_cursor,
                });
            }
        }

        visual_neighbor(&cells, current_x, cursor, direction)
    });

    if target.is_some() {
        return target;
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

fn visual_neighbor(
    cells: &[VisualCaretCell],
    current_x: f32,
    cursor: CosmicCursor,
    direction: i32,
) -> Option<CosmicCursor> {
    const EPSILON: f32 = 0.01;

    let mut best: Option<(f32, f32, CosmicCursor)> = None;

    for cell in cells {
        let (eligible, distance, span, near_cursor, far_cursor) = if direction < 0 {
            (
                cell.left < current_x - EPSILON,
                (current_x - cell.right).max(0.0),
                (current_x - cell.left).abs(),
                cell.right_cursor,
                cell.left_cursor,
            )
        } else {
            (
                cell.right > current_x + EPSILON,
                (cell.left - current_x).max(0.0),
                (cell.right - current_x).abs(),
                cell.left_cursor,
                cell.right_cursor,
            )
        };

        if !eligible {
            continue;
        }

        let candidate = if same_cosmic_cursor(near_cursor, cursor) {
            far_cursor
        } else {
            near_cursor
        };
        if same_cosmic_cursor(candidate, cursor) {
            continue;
        }

        let is_better = best.is_none_or(|(best_distance, best_span, _)| {
            distance < best_distance - EPSILON
                || ((distance - best_distance).abs() <= EPSILON && span < best_span)
        });
        if is_better {
            best = Some((distance, span, candidate));
        }
    }

    best.map(|(_, _, cursor)| cursor)
}

fn apply_platform_backend_default(_descriptor: &mut InstanceDescriptor) {
    #[cfg(target_os = "windows")]
    {
        _descriptor.backends = wgpu::Backends::DX12;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cursor(index: usize, affinity: glyphon::Affinity) -> CosmicCursor {
        CosmicCursor::new_with_affinity(0, index, affinity)
    }

    #[test]
    fn visual_neighbor_moves_between_adjacent_ltr_cells_without_pixel_scanning() {
        let c0 = cursor(0, glyphon::Affinity::After);
        let c1 = cursor(1, glyphon::Affinity::Before);
        let c1_after = cursor(1, glyphon::Affinity::After);
        let c2 = cursor(2, glyphon::Affinity::Before);
        let cells = [
            VisualCaretCell {
                left: 0.0,
                right: 10.0,
                left_cursor: c0,
                right_cursor: c1,
            },
            VisualCaretCell {
                left: 10.0,
                right: 20.0,
                left_cursor: c1_after,
                right_cursor: c2,
            },
        ];

        assert_eq!(visual_neighbor(&cells, 0.0, c0, 1), Some(c1));
        assert_eq!(visual_neighbor(&cells, 20.0, c2, -1), Some(c1_after));
    }

    #[test]
    fn visual_neighbor_preserves_same_x_bidi_boundary_transition() {
        let left_start = cursor(0, glyphon::Affinity::After);
        let left_end = cursor(1, glyphon::Affinity::Before);
        let right_start = cursor(4, glyphon::Affinity::After);
        let right_end = cursor(3, glyphon::Affinity::Before);
        let cells = [
            VisualCaretCell {
                left: 0.0,
                right: 10.0,
                left_cursor: left_start,
                right_cursor: left_end,
            },
            VisualCaretCell {
                left: 10.0,
                right: 20.0,
                left_cursor: right_start,
                right_cursor: right_end,
            },
        ];

        assert_eq!(
            visual_neighbor(&cells, 10.0, left_end, 1),
            Some(right_start)
        );
        assert_eq!(
            visual_neighbor(&cells, 10.0, right_start, -1),
            Some(left_end)
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_defaults_wgpu_to_dx12_before_environment_override() {
        let mut descriptor = InstanceDescriptor::new_without_display_handle();
        apply_platform_backend_default(&mut descriptor);
        assert_eq!(descriptor.backends, wgpu::Backends::DX12);
    }
}
