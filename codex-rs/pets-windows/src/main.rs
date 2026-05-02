use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
struct Cli {}

#[cfg(not(target_os = "windows"))]
fn main() -> Result<()> {
    let _ = Cli::parse();
    anyhow::bail!("codex-pets-windows must be built for Windows");
}

#[cfg(target_os = "windows")]
fn main() -> Result<()> {
    windows_app::run()
}

#[cfg(target_os = "windows")]
mod windows_app {
    use std::collections::VecDeque;
    use std::ffi::c_void;
    use std::io::BufRead;
    use std::io::Write;
    use std::ptr;
    use std::sync::mpsc;
    use std::time::Duration;
    use std::time::Instant;

    use anyhow::Context;
    use anyhow::Result;
    use codex_pets::protocol::HelperCommand;
    use codex_pets::protocol::HelperEvent;
    use codex_pets::protocol::HelperSnapshot;
    use codex_pets::protocol::PetState;
    use windows_sys::Win32::Foundation::COLORREF;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Foundation::LPARAM;
    use windows_sys::Win32::Foundation::LRESULT;
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Foundation::WPARAM;
    use windows_sys::Win32::Graphics::Gdi::AC_SRC_ALPHA;
    use windows_sys::Win32::Graphics::Gdi::AC_SRC_OVER;
    use windows_sys::Win32::Graphics::Gdi::BI_RGB;
    use windows_sys::Win32::Graphics::Gdi::BITMAPINFO;
    use windows_sys::Win32::Graphics::Gdi::BITMAPINFOHEADER;
    use windows_sys::Win32::Graphics::Gdi::BLENDFUNCTION;
    use windows_sys::Win32::Graphics::Gdi::CreateCompatibleDC;
    use windows_sys::Win32::Graphics::Gdi::CreateDIBSection;
    use windows_sys::Win32::Graphics::Gdi::DIB_RGB_COLORS;
    use windows_sys::Win32::Graphics::Gdi::DeleteDC;
    use windows_sys::Win32::Graphics::Gdi::DeleteObject;
    use windows_sys::Win32::Graphics::Gdi::GetDC;
    use windows_sys::Win32::Graphics::Gdi::HBITMAP;
    use windows_sys::Win32::Graphics::Gdi::HDC;
    use windows_sys::Win32::Graphics::Gdi::HGDIOBJ;
    use windows_sys::Win32::Graphics::Gdi::ReleaseDC;
    use windows_sys::Win32::Graphics::Gdi::SelectObject;
    use windows_sys::Win32::Graphics::Gdi::UpdateLayeredWindow;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
    use windows_sys::Win32::UI::WindowsAndMessaging::CREATESTRUCTW;
    use windows_sys::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
    use windows_sys::Win32::UI::WindowsAndMessaging::CreateWindowExW;
    use windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW;
    use windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::DispatchMessageW;
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
    use windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetMessageW;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
    use windows_sys::Win32::UI::WindowsAndMessaging::HMENU;
    use windows_sys::Win32::UI::WindowsAndMessaging::HWND_TOPMOST;
    use windows_sys::Win32::UI::WindowsAndMessaging::IDC_ARROW;
    use windows_sys::Win32::UI::WindowsAndMessaging::KillTimer;
    use windows_sys::Win32::UI::WindowsAndMessaging::LWA_ALPHA;
    use windows_sys::Win32::UI::WindowsAndMessaging::LoadCursorW;
    use windows_sys::Win32::UI::WindowsAndMessaging::MSG;
    use windows_sys::Win32::UI::WindowsAndMessaging::PostQuitMessage;
    use windows_sys::Win32::UI::WindowsAndMessaging::RegisterClassW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNOACTIVATE;
    use windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE;
    use windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOMOVE;
    use windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOSIZE;
    use windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes;
    use windows_sys::Win32::UI::WindowsAndMessaging::SetTimer;
    use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos;
    use windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::TranslateMessage;
    use windows_sys::Win32::UI::WindowsAndMessaging::ULW_ALPHA;
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_DESTROY;
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONDBLCLK;
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONDOWN;
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP;
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE;
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_NCCREATE;
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_TIMER;
    use windows_sys::Win32::UI::WindowsAndMessaging::WNDCLASSW;
    use windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
    use windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
    use windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
    use windows_sys::Win32::UI::WindowsAndMessaging::WS_POPUP;

    const WINDOW_WIDTH: i32 = 356;
    const WINDOW_HEIGHT: i32 = 320;
    const MASCOT_WIDTH: i32 = 112;
    const MASCOT_HEIGHT: i32 = 121;
    const CARD_LEFT: i32 = 80;
    const CARD_TOP: i32 = 52;
    const CARD_WIDTH: i32 = 276;
    const CARD_HEIGHT: i32 = 142;
    const TIMER_ID: usize = 1;
    const TIMER_MS: u32 = 80;
    const MOMENTUM_TIMER_MS: f32 = 16.0;
    const MOMENTUM_DAMPING: f32 = 0.88;
    const MOMENTUM_CUTOFF: f32 = 65.0;
    const MOMENTUM_MAX_MS: f32 = 900.0;

    static CODEX: &[u8] = include_bytes!("../assets/codex.webp");
    static DEWEY: &[u8] = include_bytes!("../assets/dewey.webp");
    static FIREBALL: &[u8] = include_bytes!("../assets/fireball.webp");
    static ROCKY: &[u8] = include_bytes!("../assets/rocky.webp");
    static SEEDY: &[u8] = include_bytes!("../assets/seedy.webp");
    static STACKY: &[u8] = include_bytes!("../assets/stacky.webp");
    static BSOD: &[u8] = include_bytes!("../assets/bsod.webp");
    static NULL_SIGNAL: &[u8] = include_bytes!("../assets/null-signal.webp");

    pub(super) fn run() -> Result<()> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut lines = std::io::stdin().lock().lines();
            while let Some(line) = lines.next() {
                match line {
                    Ok(line) => {
                        if tx.send(line).is_err() {
                            return;
                        }
                    }
                    Err(err) => {
                        let _ = write_event(&HelperEvent::Error {
                            message: format!("failed to read stdin for pets helper: {err}"),
                        });
                        return;
                    }
                }
            }
        });

        let mut app = OverlayApp::new(rx)?;
        write_event(&HelperEvent::Ready)?;
        app.run()
    }

    fn write_event(event: &HelperEvent) -> Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        serde_json::to_writer(&mut handle, event)?;
        handle.write_all(b"\n")?;
        handle.flush()?;
        Ok(())
    }

    struct OverlayApp {
        hwnd: HWND,
        rx: mpsc::Receiver<String>,
        pets: Vec<PetSprite>,
        selected_pet: String,
        snapshot: HelperSnapshot,
        frame_started: Instant,
        dragging: Option<DragState>,
        momentum: Option<MomentumState>,
        terminal_hwnd: Option<HWND>,
        visible: bool,
    }

    impl OverlayApp {
        fn new(rx: mpsc::Receiver<String>) -> Result<Box<Self>> {
            let pets = load_pets()?;
            let snapshot = HelperSnapshot {
                pet: "codex".to_string(),
                state: PetState::Idle,
                title: "Codex".to_string(),
                subtitle: Some("Idle".to_string()),
                detail: None,
            };
            let mut app = Box::new(Self {
                hwnd: 0,
                rx,
                pets,
                selected_pet: "codex".to_string(),
                snapshot,
                frame_started: Instant::now(),
                dragging: None,
                momentum: None,
                terminal_hwnd: None,
                visible: false,
            });
            let hwnd = create_window(app.as_mut())?;
            app.hwnd = hwnd;
            Ok(app)
        }

        fn run(&mut self) -> Result<()> {
            unsafe {
                SetTimer(self.hwnd, TIMER_ID, TIMER_MS, None);
            }
            loop {
                self.drain_commands()?;
                let mut msg = MSG::default();
                let has_message = unsafe { GetMessageW(&mut msg, 0, 0, 0) };
                if has_message == -1 {
                    anyhow::bail!("GetMessageW failed");
                }
                if has_message == 0 {
                    break;
                }
                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            Ok(())
        }

        fn drain_commands(&mut self) -> Result<()> {
            while let Ok(line) = self.rx.try_recv() {
                let command = serde_json::from_str::<HelperCommand>(&line)
                    .with_context(|| format!("failed to parse helper command: {line}"))?;
                match command {
                    HelperCommand::Show {
                        pet,
                        terminal_window_hint,
                    } => {
                        self.selected_pet = normalized_pet(&pet).to_string();
                        self.snapshot.pet = self.selected_pet.clone();
                        self.terminal_hwnd = terminal_window_hint
                            .as_deref()
                            .and_then(find_terminal_window)
                            .or_else(foreground_window);
                        if self.visible {
                            self.hide()?;
                        } else {
                            self.show();
                        }
                    }
                    HelperCommand::Hide => self.hide()?,
                    HelperCommand::SetSnapshot { snapshot } => {
                        self.selected_pet = normalized_pet(&snapshot.pet).to_string();
                        self.snapshot = HelperSnapshot {
                            pet: self.selected_pet.clone(),
                            ..snapshot
                        };
                        self.frame_started = Instant::now();
                        self.render()?;
                    }
                    HelperCommand::Shutdown => unsafe {
                        DestroyWindow(self.hwnd);
                    },
                }
            }
            Ok(())
        }

        fn show(&mut self) {
            self.visible = true;
            unsafe {
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            }
            let _ = self.render();
        }

        fn hide(&mut self) -> Result<()> {
            self.visible = false;
            unsafe {
                ShowWindow(self.hwnd, SW_HIDE);
            }
            write_event(&HelperEvent::Hidden)
        }

        fn render(&mut self) -> Result<()> {
            if !self.visible {
                return Ok(());
            }
            let mut canvas = vec![0_u8; (WINDOW_WIDTH * WINDOW_HEIGHT * 4) as usize];
            draw_card(&mut canvas, &self.snapshot);
            let pet = self
                .pets
                .iter()
                .find(|pet| pet.id == self.selected_pet)
                .unwrap_or(&self.pets[0]);
            let frame = current_frame(self.snapshot.state, self.frame_started.elapsed());
            draw_sprite_frame(&mut canvas, pet, frame, 244, 191);
            update_layered_window(self.hwnd, &canvas, WINDOW_WIDTH, WINDOW_HEIGHT)
        }

        fn on_timer(&mut self) {
            self.drain_commands().ok();
            self.advance_momentum();
            self.render().ok();
        }

        fn begin_drag(&mut self) {
            let mut cursor = POINT::default();
            let mut rect = RECT::default();
            unsafe {
                GetCursorPos(&mut cursor);
                GetWindowRect(self.hwnd, &mut rect);
            }
            self.momentum = None;
            self.dragging = Some(DragState {
                offset_x: cursor.x - rect.left,
                offset_y: cursor.y - rect.top,
                samples: VecDeque::from([DragSample {
                    point: cursor,
                    at: Instant::now(),
                }]),
            });
        }

        fn update_drag(&mut self) {
            let Some(drag) = self.dragging.as_mut() else {
                return;
            };
            let mut cursor = POINT::default();
            unsafe {
                GetCursorPos(&mut cursor);
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    cursor.x - drag.offset_x,
                    cursor.y - drag.offset_y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            drag.samples.push_back(DragSample {
                point: cursor,
                at: Instant::now(),
            });
            while drag.samples.len() > 5 {
                drag.samples.pop_front();
            }
        }

        fn end_drag(&mut self) {
            let Some(drag) = self.dragging.take() else {
                return;
            };
            self.momentum = drag.velocity().map(|(vx, vy)| MomentumState {
                velocity_x: vx,
                velocity_y: vy,
                elapsed_ms: 0.0,
            });
        }

        fn advance_momentum(&mut self) {
            let Some(momentum) = self.momentum.as_mut() else {
                return;
            };
            momentum.elapsed_ms += MOMENTUM_TIMER_MS;
            let speed = (momentum.velocity_x.powi(2) + momentum.velocity_y.powi(2)).sqrt();
            if speed < MOMENTUM_CUTOFF || momentum.elapsed_ms >= MOMENTUM_MAX_MS {
                self.momentum = None;
                return;
            }
            let mut rect = RECT::default();
            unsafe {
                GetWindowRect(self.hwnd, &mut rect);
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    rect.left + (momentum.velocity_x * MOMENTUM_TIMER_MS / 1000.0) as i32,
                    rect.top + (momentum.velocity_y * MOMENTUM_TIMER_MS / 1000.0) as i32,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            momentum.velocity_x *= MOMENTUM_DAMPING;
            momentum.velocity_y *= MOMENTUM_DAMPING;
        }

        fn focus_terminal(&self) {
            if let Some(hwnd) = self.terminal_hwnd {
                unsafe {
                    SetForegroundWindow(hwnd);
                }
            }
        }
    }

    struct DragState {
        offset_x: i32,
        offset_y: i32,
        samples: VecDeque<DragSample>,
    }

    impl DragState {
        fn velocity(&self) -> Option<(f32, f32)> {
            let first = self.samples.front()?;
            let last = self.samples.back()?;
            let dt = last.at.duration_since(first.at).as_secs_f32();
            if dt <= 0.0 {
                return None;
            }
            Some((
                (last.point.x - first.point.x) as f32 / dt,
                (last.point.y - first.point.y) as f32 / dt,
            ))
        }
    }

    struct DragSample {
        point: POINT,
        at: Instant,
    }

    struct MomentumState {
        velocity_x: f32,
        velocity_y: f32,
        elapsed_ms: f32,
    }

    struct PetSprite {
        id: &'static str,
        pixels: Vec<u8>,
        width: u32,
        height: u32,
    }

    #[derive(Clone, Copy)]
    struct Frame {
        row: u32,
        col: u32,
        duration_ms: u64,
    }

    fn create_window(app: *mut OverlayApp) -> Result<HWND> {
        let class_name = wide("CodexPetsOverlay");
        let title = wide("Codex Pets");
        let hinstance = unsafe { GetModuleHandleW(ptr::null()) };
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: hinstance,
            lpszClassName: class_name.as_ptr(),
            hCursor: unsafe { LoadCursorW(0, IDC_ARROW) },
            ..Default::default()
        };
        unsafe {
            RegisterClassW(&wc);
        }
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
                0,
                0 as HMENU,
                hinstance,
                app.cast::<c_void>(),
            )
        };
        if hwnd == 0 {
            anyhow::bail!("failed to create pets overlay window");
        }
        unsafe {
            SetLayeredWindowAttributes(hwnd, 0 as COLORREF, 255, LWA_ALPHA);
        }
        Ok(hwnd)
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_NCCREATE {
            let createstruct = lparam as *const CREATESTRUCTW;
            let app = unsafe { (*createstruct).lpCreateParams as *mut OverlayApp };
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, app as isize);
            }
        }
        let app_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayApp };
        if !app_ptr.is_null() {
            let app = unsafe { &mut *app_ptr };
            match msg {
                WM_TIMER => {
                    app.on_timer();
                    return 0;
                }
                WM_LBUTTONDOWN => {
                    app.begin_drag();
                    return 0;
                }
                WM_MOUSEMOVE => {
                    if app.dragging.is_some() {
                        app.update_drag();
                    }
                    return 0;
                }
                WM_LBUTTONUP => {
                    unsafe {
                        ReleaseCapture();
                    }
                    app.end_drag();
                    app.focus_terminal();
                    return 0;
                }
                WM_LBUTTONDBLCLK => {
                    app.focus_terminal();
                    return 0;
                }
                WM_DESTROY => {
                    unsafe {
                        KillTimer(hwnd, TIMER_ID);
                        PostQuitMessage(0);
                    }
                    return 0;
                }
                _ => {}
            }
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    fn update_layered_window(hwnd: HWND, rgba: &[u8], width: i32, height: i32) -> Result<()> {
        let screen_dc = unsafe { GetDC(0) };
        if screen_dc == 0 {
            anyhow::bail!("GetDC failed");
        }
        let mem_dc = unsafe { CreateCompatibleDC(screen_dc) };
        if mem_dc == 0 {
            unsafe {
                ReleaseDC(0, screen_dc);
            }
            anyhow::bail!("CreateCompatibleDC failed");
        }

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = ptr::null_mut();
        let bitmap =
            unsafe { CreateDIBSection(mem_dc, &bitmap_info, DIB_RGB_COLORS, &mut bits, 0, 0) };
        if bitmap == 0 || bits.is_null() {
            cleanup_gdi(screen_dc, mem_dc, bitmap);
            anyhow::bail!("CreateDIBSection failed");
        }

        let bgra = unsafe { std::slice::from_raw_parts_mut(bits.cast::<u8>(), rgba.len()) };
        for (dst, src) in bgra.chunks_exact_mut(4).zip(rgba.chunks_exact(4)) {
            dst[0] = src[2];
            dst[1] = src[1];
            dst[2] = src[0];
            dst[3] = src[3];
        }
        let old = unsafe { SelectObject(mem_dc, bitmap as HGDIOBJ) };
        let size = windows_sys::Win32::Foundation::SIZE {
            cx: width,
            cy: height,
        };
        let source = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA,
        };
        let ok = unsafe {
            UpdateLayeredWindow(
                hwnd,
                screen_dc,
                ptr::null(),
                &size,
                mem_dc,
                &source,
                0,
                &blend,
                ULW_ALPHA,
            )
        };
        unsafe {
            SelectObject(mem_dc, old);
        }
        cleanup_gdi(screen_dc, mem_dc, bitmap);
        if ok == 0 {
            anyhow::bail!("UpdateLayeredWindow failed");
        }
        Ok(())
    }

    fn cleanup_gdi(screen_dc: HDC, mem_dc: HDC, bitmap: HBITMAP) {
        unsafe {
            if bitmap != 0 {
                DeleteObject(bitmap as HGDIOBJ);
            }
            if mem_dc != 0 {
                DeleteDC(mem_dc);
            }
            if screen_dc != 0 {
                ReleaseDC(0, screen_dc);
            }
        }
    }

    fn load_pets() -> Result<Vec<PetSprite>> {
        [
            ("codex", CODEX),
            ("dewey", DEWEY),
            ("fireball", FIREBALL),
            ("rocky", ROCKY),
            ("seedy", SEEDY),
            ("stacky", STACKY),
            ("bsod", BSOD),
            ("null-signal", NULL_SIGNAL),
        ]
        .into_iter()
        .map(|(id, bytes)| load_pet(id, bytes))
        .collect()
    }

    fn load_pet(id: &'static str, bytes: &[u8]) -> Result<PetSprite> {
        let image = image::load_from_memory(bytes)
            .with_context(|| format!("failed to decode {id} pet spritesheet"))?
            .to_rgba8();
        let (width, height) = image.dimensions();
        Ok(PetSprite {
            id,
            pixels: image.into_raw(),
            width,
            height,
        })
    }

    fn draw_sprite_frame(canvas: &mut [u8], pet: &PetSprite, frame: Frame, x: i32, y: i32) {
        let source_width = pet.width / 8;
        let source_height = pet.height / 9;
        let source_x = frame.col * source_width;
        let source_y = frame.row * source_height;
        for out_y in 0..MASCOT_HEIGHT {
            for out_x in 0..MASCOT_WIDTH {
                let sx = source_x + (out_x as u32 * source_width / MASCOT_WIDTH as u32);
                let sy = source_y + (out_y as u32 * source_height / MASCOT_HEIGHT as u32);
                let src = ((sy * pet.width + sx) * 4) as usize;
                let dx = x + out_x;
                let dy = y + out_y;
                if dx < 0 || dy < 0 || dx >= WINDOW_WIDTH || dy >= WINDOW_HEIGHT {
                    continue;
                }
                let dst = ((dy * WINDOW_WIDTH + dx) * 4) as usize;
                alpha_blend(&mut canvas[dst..dst + 4], &pet.pixels[src..src + 4]);
            }
        }
    }

    fn draw_card(canvas: &mut [u8], snapshot: &HelperSnapshot) {
        fill_round_rect(
            canvas,
            CARD_LEFT,
            CARD_TOP,
            CARD_WIDTH,
            CARD_HEIGHT,
            18,
            [22, 24, 28, 225],
        );
        fill_round_rect(
            canvas,
            CARD_LEFT,
            CARD_TOP,
            CARD_WIDTH,
            28,
            18,
            [45, 49, 58, 230],
        );
        draw_text(
            canvas,
            CARD_LEFT + 14,
            CARD_TOP + 11,
            &snapshot.title,
            2,
            [245, 246, 248, 255],
        );
        if let Some(subtitle) = snapshot.subtitle.as_deref() {
            draw_text(
                canvas,
                CARD_LEFT + 14,
                CARD_TOP + 44,
                subtitle,
                2,
                [210, 220, 235, 255],
            );
        }
        if let Some(detail) = snapshot.detail.as_deref() {
            draw_wrapped_text(
                canvas,
                CARD_LEFT + 14,
                CARD_TOP + 72,
                detail,
                35,
                [170, 181, 196, 255],
            );
        }
    }

    fn fill_round_rect(
        canvas: &mut [u8],
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        radius: i32,
        color: [u8; 4],
    ) {
        for py in y..y + height {
            for px in x..x + width {
                let cx = if px < x + radius {
                    x + radius
                } else if px >= x + width - radius {
                    x + width - radius - 1
                } else {
                    px
                };
                let cy = if py < y + radius {
                    y + radius
                } else if py >= y + height - radius {
                    y + height - radius - 1
                } else {
                    py
                };
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy <= radius * radius {
                    put_pixel(canvas, px, py, color);
                }
            }
        }
    }

    fn draw_wrapped_text(
        canvas: &mut [u8],
        x: i32,
        y: i32,
        text: &str,
        max_chars: usize,
        color: [u8; 4],
    ) {
        let mut current = String::new();
        let mut line_y = y;
        for word in text.split_whitespace() {
            if !current.is_empty() && current.len() + 1 + word.len() > max_chars {
                draw_text(canvas, x, line_y, &current, 1, color);
                line_y += 14;
                current.clear();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
            if line_y > y + 42 {
                break;
            }
        }
        if !current.is_empty() {
            draw_text(canvas, x, line_y, &current, 1, color);
        }
    }

    fn draw_text(canvas: &mut [u8], x: i32, y: i32, text: &str, scale: i32, color: [u8; 4]) {
        let mut cursor_x = x;
        for ch in text.chars().take(48) {
            if ch == '\n' {
                cursor_x = x;
                continue;
            }
            draw_char(canvas, cursor_x, y, ch, scale, color);
            cursor_x += 6 * scale;
        }
    }

    fn draw_char(canvas: &mut [u8], x: i32, y: i32, ch: char, scale: i32, color: [u8; 4]) {
        let glyph = glyph(ch);
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) == 0 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        put_pixel(
                            canvas,
                            x + col * scale + sx,
                            y + row as i32 * scale + sy,
                            color,
                        );
                    }
                }
            }
        }
    }

    fn glyph(ch: char) -> [u8; 7] {
        match ch.to_ascii_uppercase() {
            'A' => [
                0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ],
            'B' => [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
            ],
            'C' => [
                0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
            ],
            'D' => [
                0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
            ],
            'E' => [
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
            ],
            'F' => [
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
            ],
            'G' => [
                0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
            ],
            'H' => [
                0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ],
            'I' => [
                0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
            ],
            'J' => [
                0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
            ],
            'K' => [
                0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
            ],
            'L' => [
                0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
            ],
            'M' => [
                0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
            ],
            'N' => [
                0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
            ],
            'O' => [
                0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ],
            'P' => [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
            ],
            'Q' => [
                0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
            ],
            'R' => [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
            ],
            'S' => [
                0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
            ],
            'T' => [
                0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
            ],
            'U' => [
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ],
            'V' => [
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
            ],
            'W' => [
                0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
            ],
            'X' => [
                0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
            ],
            'Y' => [
                0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
            ],
            'Z' => [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
            ],
            '0' => [
                0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
            ],
            '1' => [
                0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ],
            '2' => [
                0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
            ],
            '3' => [
                0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
            ],
            '4' => [
                0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
            ],
            '5' => [
                0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
            ],
            '6' => [
                0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
            ],
            '7' => [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
            ],
            '8' => [
                0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
            ],
            '9' => [
                0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
            ],
            ':' => [0, 0b00100, 0b00100, 0, 0b00100, 0b00100, 0],
            '-' => [0, 0, 0, 0b11111, 0, 0, 0],
            '_' => [0, 0, 0, 0, 0, 0, 0b11111],
            '/' => [
                0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000,
            ],
            '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
            '%' => [0b11001, 0b11010, 0b00100, 0b01000, 0b10110, 0b00110, 0],
            ' ' => [0, 0, 0, 0, 0, 0, 0],
            _ => [0, 0, 0, 0b11110, 0, 0, 0],
        }
    }

    fn current_frame(state: PetState, elapsed: Duration) -> Frame {
        let frames = frames_for_state(state);
        let total_duration_ms = frames.iter().map(|frame| frame.duration_ms).sum::<u64>();
        let mut elapsed_ms = (elapsed.as_millis() as u64) % total_duration_ms.max(1);
        for frame in frames {
            if elapsed_ms < frame.duration_ms {
                return *frame;
            }
            elapsed_ms -= frame.duration_ms;
        }
        *frames.last().expect("pet animation state has frames")
    }

    fn frames_for_state(state: PetState) -> &'static [Frame] {
        match state {
            PetState::Idle => &IDLE_FRAMES,
            PetState::Running => &RUNNING_FRAMES,
            PetState::Waiting => &WAITING_FRAMES,
            PetState::Review => &REVIEW_FRAMES,
            PetState::Failed => &FAILED_FRAMES,
        }
    }

    const IDLE_FRAMES: [Frame; 6] = [
        Frame {
            row: 0,
            col: 0,
            duration_ms: 280,
        },
        Frame {
            row: 0,
            col: 1,
            duration_ms: 110,
        },
        Frame {
            row: 0,
            col: 2,
            duration_ms: 110,
        },
        Frame {
            row: 0,
            col: 3,
            duration_ms: 140,
        },
        Frame {
            row: 0,
            col: 4,
            duration_ms: 140,
        },
        Frame {
            row: 0,
            col: 5,
            duration_ms: 320,
        },
    ];
    const RUNNING_FRAMES: [Frame; 6] = frames_row(7, 120, 220);
    const WAITING_FRAMES: [Frame; 6] = frames_row(6, 150, 260);
    const REVIEW_FRAMES: [Frame; 6] = frames_row(8, 150, 280);
    const FAILED_FRAMES: [Frame; 8] = frames_row(5, 140, 240);

    const fn frames_row<const N: usize>(
        row: u32,
        duration_ms: u64,
        last_duration_ms: u64,
    ) -> [Frame; N] {
        let mut frames = [Frame {
            row,
            col: 0,
            duration_ms,
        }; N];
        let mut idx = 0;
        while idx < N {
            frames[idx] = Frame {
                row,
                col: idx as u32,
                duration_ms: if idx == N - 1 {
                    last_duration_ms
                } else {
                    duration_ms
                },
            };
            idx += 1;
        }
        frames
    }

    fn alpha_blend(dst: &mut [u8], src: &[u8]) {
        let alpha = src[3] as u16;
        let inv = 255 - alpha;
        dst[0] = ((src[0] as u16 * alpha + dst[0] as u16 * inv) / 255) as u8;
        dst[1] = ((src[1] as u16 * alpha + dst[1] as u16 * inv) / 255) as u8;
        dst[2] = ((src[2] as u16 * alpha + dst[2] as u16 * inv) / 255) as u8;
        dst[3] = alpha.saturating_add((dst[3] as u16 * inv) / 255) as u8;
    }

    fn put_pixel(canvas: &mut [u8], x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x >= WINDOW_WIDTH || y >= WINDOW_HEIGHT {
            return;
        }
        let dst = ((y * WINDOW_WIDTH + x) * 4) as usize;
        alpha_blend(&mut canvas[dst..dst + 4], &color);
    }

    fn normalized_pet(value: &str) -> &str {
        match value {
            "codex" | "dewey" | "fireball" | "rocky" | "seedy" | "stacky" | "bsod"
            | "null-signal" => value,
            _ => "codex",
        }
    }

    fn find_terminal_window(hint: &str) -> Option<HWND> {
        let exact = wide(hint);
        let hwnd = unsafe { FindWindowW(ptr::null(), exact.as_ptr()) };
        if hwnd != 0 {
            return Some(hwnd);
        }
        let fallback = wide("Windows Terminal");
        let hwnd = unsafe { FindWindowW(ptr::null(), fallback.as_ptr()) };
        (hwnd != 0).then_some(hwnd)
    }

    fn foreground_window() -> Option<HWND> {
        let hwnd = unsafe { GetForegroundWindow() };
        (hwnd != 0).then_some(hwnd)
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }
}
