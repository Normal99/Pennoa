//! Phase 0 spike: pen input and pen-to-photon latency on Wayland (Hyprland/KDE Plasma).
//!
//! * Slint runs with its own winit backend (so accessibility keeps working).
//! * We bind `zwp_tablet_v2` on the same `wl_display` for pressure, tilt and eraser.
//! * Ink is drawn by a tiny wgpu pipeline, either
//!   - `--mode texture`: into a texture that Slint shows as an Image (Slint presents with vsync), or
//!   - `--mode subsurface`: onto our own desynchronised subsurface with its own present mode and
//!     the tearing hint (lowest latency).
//! * `wp_presentation` feedback measures when each frame really reached the screen.
//!
//! Keys: `c` clear, `t` toggle tearing hint, `s` save report, `Esc` quit.

mod clock;
mod ink;
mod wl;

use clap::{Parser, ValueEnum};
use slint::wgpu_30::wgpu;
use slint::winit_030::{CustomApplicationHandler, EventResult, winit};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

slint::slint! {
    #[edition = "1.19"]
    import { Button } from "std-widgets.slint";
    export component PenSpike inherits Window {
        title: "Pennoa · pen spike";
        preferred-width: 1280px;
        preferred-height: 800px;
        background: #F3F4F7;
        in property <image> ink;
        in property <string> stats: "waiting for pen…";
        in property <string> mode-label;
        in property <bool> tearing;
        in property <bool> show-image: true;
        out property <length> toolbar-height: 64px;
        callback clear();
        callback save();
        callback toggle-tearing();
        VerticalLayout {
            Rectangle {
                height: root.toolbar-height;
                background: white;
                HorizontalLayout {
                    padding: 10px;
                    spacing: 10px;
                    Button { text: "Clear"; clicked => { root.clear(); } }
                    Button { text: root.tearing ? "Tearing: ON" : "Tearing: off"; clicked => { root.toggle-tearing(); } }
                    Button { text: "Save report"; clicked => { root.save(); } }
                    Text { text: root.mode-label; vertical-alignment: center; font-weight: 700; }
                    Text { text: root.stats; vertical-alignment: center; font-size: 13px; horizontal-stretch: 1; }
                }
            }
            if root.show-image: Image { source: root.ink; image-fit: fill; }
            if !root.show-image: Rectangle { background: white; }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Serialize)]
enum Mode {
    Texture,
    Subsurface,
}

#[derive(Parser, Debug, Clone, serde::Serialize)]
struct Args {
    /// How wet ink reaches the screen.
    #[arg(long, value_enum, default_value_t = Mode::Subsurface)]
    mode: Mode,
    /// Present modes to try for the canvas subsurface, in order.
    #[arg(long, value_delimiter = ',', default_values_t = ["mailbox".to_string(), "immediate".to_string(), "fifo".to_string()])]
    present: Vec<String>,
    /// Start with the tearing hint enabled (subsurface mode).
    #[arg(long)]
    tearing: bool,
    /// Draw a scripted stroke instead of waiting for a pen (for testing without a tablet).
    #[arg(long)]
    demo: bool,
    /// Where to write the JSON report.
    #[arg(long, default_value = "pen-report.json")]
    report: std::path::PathBuf,
    /// Quit automatically after this many seconds (writes the report first).
    #[arg(long)]
    seconds: Option<u64>,
    /// Base pen width in logical pixels at full pressure.
    #[arg(long, default_value_t = 2.6)]
    width: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct FrameInputs {
    oldest_event_ns: u64,
    newest_event_ns: u64,
    newest_recv_ns: u64,
    samples: u32,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
struct LatencySample {
    /// Compositor input timestamp of the newest pen sample in the frame -> photons.
    input_to_present_ms: f64,
    /// Our process receiving that sample -> photons.
    recv_to_present_ms: f64,
    /// Oldest sample drawn in the frame -> photons.
    oldest_input_to_present_ms: f64,
    samples: u32,
    tearing: bool,
    vsync_flag: bool,
}

struct Shared {
    args: Args,
    app: slint::Weak<PenSpike>,
    wl: Option<wl::Wl>,
    gpu: Option<(wgpu::Instance, wgpu::Device, wgpu::Queue)>,
    ink: Option<ink::InkRenderer>,
    canvas: Option<wl::CanvasSurface>,
    swap: Option<ink::CanvasSwapchain>,
    scale: f64,
    window_logical: (f64, f64),
    toolbar: f64,
    tearing: bool,
    next_tag: u64,
    frame: FrameInputs,
    in_flight: HashMap<u64, (FrameInputs, bool)>,
    latencies: Vec<LatencySample>,
    discarded: u64,
    refresh_ns: u32,
    feedback_requested: u64,
    presented_events: u64,
    raw_latency_debug: Vec<(u64, u64, u64)>,
    pen_near: bool,
    touches_while_pen_near: u64,
    samples_total: u64,
    samples_window: Vec<u64>,
    last_sample: Option<wl::PenSample>,
    demo_t0: Option<u64>,
    errors: Vec<String>,
}

impl Shared {
    fn canvas_logical(&self) -> (f64, f64, f64, f64) {
        (
            0.0,
            self.toolbar,
            self.window_logical.0,
            (self.window_logical.1 - self.toolbar).max(1.0),
        )
    }

    fn canvas_physical_size(&self) -> (u32, u32) {
        let (_, _, w, h) = self.canvas_logical();
        (
            (w * self.scale).round() as u32,
            (h * self.scale).round() as u32,
        )
    }

    /// Create GPU resources once both Wayland and Slint's GPU device exist.
    fn ensure_ready(&mut self) -> Option<slint::Image> {
        let (instance, device, _queue) = self.gpu.clone()?;
        self.wl.as_ref()?;
        let mut new_image = None;
        let (pw, ph) = self.canvas_physical_size();
        if self
            .ink
            .as_ref()
            .is_none_or(|i| i.size != (pw.max(1), ph.max(1)))
        {
            let ink = ink::InkRenderer::new(&device, pw, ph);
            if self.args.mode == Mode::Texture {
                match slint::Image::try_from(ink.texture.clone()) {
                    Ok(img) => new_image = Some(img),
                    Err(e) => self.errors.push(format!("texture import: {e:?}")),
                }
            }
            self.ink = Some(ink);
            if let Some(swap) = &mut self.swap {
                swap.resize(&device, pw, ph);
            }
            if let Some(canvas) = &self.canvas {
                let (x, y, w, h) = self.canvas_logical();
                canvas.resize(x as i32, y as i32, w as i32, h as i32);
            }
        }
        if self.args.mode == Mode::Subsurface && self.canvas.is_none() {
            let (x, y, w, h) = self.canvas_logical();
            let wl = self.wl.as_mut()?;
            match wl.create_canvas(x as i32, y as i32, w as i32, h as i32) {
                Ok(canvas) => {
                    canvas.set_tearing(self.tearing);
                    let wanted: Vec<wgpu::PresentMode> = self
                        .args
                        .present
                        .iter()
                        .filter_map(|p| match p.as_str() {
                            "mailbox" => Some(wgpu::PresentMode::Mailbox),
                            "immediate" => Some(wgpu::PresentMode::Immediate),
                            "fifo" => Some(wgpu::PresentMode::Fifo),
                            "fifo-relaxed" => Some(wgpu::PresentMode::FifoRelaxed),
                            _ => None,
                        })
                        .collect();
                    // SAFETY: the display and our canvas surface live as long as `self`.
                    match unsafe {
                        ink::CanvasSwapchain::new(
                            &instance,
                            &device,
                            wl.display_ptr,
                            canvas.surface_ptr(),
                            pw,
                            ph,
                            &wanted,
                        )
                    } {
                        Ok(swap) => {
                            eprintln!(
                                "canvas subsurface: present mode {:?}",
                                swap.present_mode_used
                            );
                            self.swap = Some(swap);
                        }
                        Err(e) => self.errors.push(format!("canvas swapchain: {e}")),
                    }
                    self.canvas = Some(canvas);
                }
                Err(e) => self.errors.push(format!("canvas subsurface: {e}")),
            }
        }
        new_image
    }

    fn take_frame(&mut self) -> Option<FrameInputs> {
        (self.frame.samples > 0).then(|| std::mem::take(&mut self.frame))
    }

    fn collect_presented(&mut self) {
        let Some(wl) = &mut self.wl else { return };
        let clock_id = wl
            .state
            .report
            .presentation_clock_id
            .unwrap_or(libc::CLOCK_MONOTONIC as u32);
        for p in wl.state.presented.drain(..) {
            self.presented_events += 1;
            let Some((inputs, tearing)) = self.in_flight.remove(&p.tag) else {
                continue;
            };
            if self.raw_latency_debug.len() < 20 {
                self.raw_latency_debug.push((
                    inputs.newest_event_ns,
                    inputs.newest_recv_ns,
                    p.presented_ns.unwrap_or(0),
                ));
            }
            if p.refresh_ns > 0 {
                self.refresh_ns = p.refresh_ns;
            }
            let Some(t) = p.presented_ns.map(|t| clock::to_monotonic(t, clock_id)) else {
                self.discarded += 1;
                continue;
            };
            let ms = |a: u64| (t as f64 - a as f64) / 1e6;
            self.latencies.push(LatencySample {
                input_to_present_ms: ms(inputs.newest_event_ns),
                recv_to_present_ms: ms(inputs.newest_recv_ns),
                oldest_input_to_present_ms: ms(inputs.oldest_event_ns),
                samples: inputs.samples,
                tearing,
                vsync_flag: p.flags & 1 != 0,
            });
        }
    }
}

/// Things to do after releasing the borrow on `Shared` (they may re-enter Slint).
#[derive(Default)]
struct Deferred {
    clicks: Vec<(f32, f32)>,
    redraw: bool,
    image: Option<slint::Image>,
}

struct Handler(Rc<RefCell<Shared>>);

impl Handler {
    fn process(&self) -> Deferred {
        let mut d = Deferred::default();
        let mut s = self.0.borrow_mut();
        if let Some(wl) = &mut s.wl {
            wl.dispatch();
        }
        d.image = s.ensure_ready();
        s.collect_presented();

        let mut samples =
            s.wl.as_mut()
                .map(|wl| std::mem::take(&mut wl.state.samples))
                .unwrap_or_default();
        if s.args.demo {
            samples.extend(demo_samples(&mut s));
        }
        let scale = s.scale;
        let toolbar = s.toolbar;
        let base_r = s.args.width / 2.0;
        for smp in samples {
            s.pen_near = true;
            s.samples_total += 1;
            s.samples_window.push(smp.recv_ns);
            s.last_sample = Some(smp);
            let (cx, cy) = match smp.target {
                wl::Target::Canvas => (smp.x, smp.y),
                wl::Target::Ui if smp.y >= toolbar => (smp.x, smp.y - toolbar),
                wl::Target::Ui => {
                    if smp.stroke_start {
                        d.clicks.push((smp.x as f32, smp.y as f32));
                    }
                    continue;
                }
                wl::Target::Other => continue,
            };
            let Some(ink) = &mut s.ink else { continue };
            if !smp.down {
                ink.end_stroke();
                continue;
            }
            if smp.stroke_start {
                ink.end_stroke();
            }
            let r = if smp.eraser {
                10.0
            } else {
                base_r * (0.3 + 0.7 * smp.pressure)
            };
            ink.add_point(
                (cx * scale) as f32,
                (cy * scale) as f32,
                (r * scale) as f32,
                smp.eraser,
            );
            let f = &mut s.frame;
            if f.samples == 0 {
                f.oldest_event_ns = smp.event_ns;
            }
            f.newest_event_ns = smp.event_ns;
            f.newest_recv_ns = smp.recv_ns;
            f.samples += 1;
        }

        let pending = s.ink.as_ref().is_some_and(|i| i.has_pending());
        match s.args.mode {
            Mode::Texture => d.redraw = pending,
            Mode::Subsurface if pending => {
                let Some((_, device, queue)) = s.gpu.clone() else {
                    return d;
                };
                let tag = s.next_tag;
                s.next_tag += 1;
                let frame = s.take_frame();
                let tearing = s.tearing;
                let shared = &mut *s;
                if let (Some(ink), Some(swap), Some(canvas), Some(wl)) =
                    (&mut shared.ink, &shared.swap, &shared.canvas, &shared.wl)
                {
                    ink.flush(&device, &queue);
                    let fb = wl.request_feedback(&canvas.surface, tag);
                    shared.feedback_requested += fb as u64;
                    if swap.present(&device, &queue, ink)
                        && fb
                        && let Some(f) = frame
                    {
                        shared.in_flight.insert(tag, (f, tearing));
                    }
                }
            }
            Mode::Subsurface => {}
        }
        d
    }

    fn apply(&self, d: Deferred) {
        let app = self.0.borrow().app.upgrade();
        let Some(app) = app else { return };
        if let Some(img) = d.image {
            app.set_ink(img);
        }
        for (x, y) in d.clicks {
            use slint::platform::{PointerEventButton, WindowEvent};
            let position = slint::LogicalPosition::new(x, y);
            let w = app.window();
            w.dispatch_event(WindowEvent::PointerMoved { position });
            w.dispatch_event(WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            });
            w.dispatch_event(WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            });
        }
        if d.redraw {
            app.window().request_redraw();
        }
    }
}

impl CustomApplicationHandler for Handler {
    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        winit_window: Option<&winit::window::Window>,
        _slint_window: Option<&slint::Window>,
        event: &winit::event::WindowEvent,
    ) -> EventResult {
        use winit::event::WindowEvent as WE;
        {
            let mut s = self.0.borrow_mut();
            if let Some(win) = winit_window {
                s.scale = win.scale_factor();
                let size = win.inner_size().to_logical::<f64>(s.scale);
                s.window_logical = (size.width, size.height);
                if s.wl.is_none() {
                    init_wayland(&mut s, win);
                }
            }
            match event {
                WE::Touch(_) if s.pen_near => {
                    s.touches_while_pen_near += 1;
                }
                WE::KeyboardInput { event, .. }
                    if event.state == winit::event::ElementState::Pressed =>
                {
                    use winit::keyboard::{Key, NamedKey};
                    match &event.logical_key {
                        Key::Character(c) if c.as_str() == "c" => {
                            drop(s);
                            self.clear();
                            return EventResult::PreventDefault;
                        }
                        Key::Character(c) if c.as_str() == "t" => {
                            drop(s);
                            self.toggle_tearing();
                            return EventResult::PreventDefault;
                        }
                        Key::Character(c) if c.as_str() == "s" => {
                            drop(s);
                            self.save();
                            return EventResult::PreventDefault;
                        }
                        Key::Named(NamedKey::Escape) => {
                            drop(s);
                            self.save();
                            let _ = slint::quit_event_loop();
                            return EventResult::PreventDefault;
                        }
                        _ => {}
                    }
                }
                WE::CloseRequested => {
                    drop(s);
                    self.save();
                    return EventResult::Propagate;
                }
                _ => {}
            }
        }
        EventResult::Propagate
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) -> EventResult {
        let d = self.process();
        self.apply(d);
        EventResult::Propagate
    }
}

impl Handler {
    fn clear(&self) {
        let mut s = self.0.borrow_mut();
        if let Some(ink) = &mut s.ink {
            ink.clear();
        }
        s.latencies.clear();
        s.discarded = 0;
        drop(s);
        let d = self.process();
        self.apply(d);
    }

    fn toggle_tearing(&self) {
        let mut s = self.0.borrow_mut();
        s.tearing = !s.tearing;
        let on = s.tearing;
        if let Some(c) = &s.canvas {
            c.set_tearing(on);
        }
        let app = s.app.upgrade();
        drop(s);
        if let Some(app) = app {
            app.set_tearing(on);
        }
    }

    fn save(&self) {
        let s = self.0.borrow();
        let report = build_report(&s);
        match serde_json::to_string_pretty(&report) {
            Ok(json) => match std::fs::write(&s.args.report, json) {
                Ok(()) => eprintln!("report written to {}", s.args.report.display()),
                Err(e) => eprintln!("could not write report: {e}"),
            },
            Err(e) => eprintln!("report: {e}"),
        }
    }
}

fn init_wayland(s: &mut Shared, win: &winit::window::Window) {
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
    let (Ok(dh), Ok(wh)) = (win.display_handle(), win.window_handle()) else {
        return;
    };
    match (dh.as_raw(), wh.as_raw()) {
        (RawDisplayHandle::Wayland(d), RawWindowHandle::Wayland(w)) => {
            // SAFETY: winit owns both and keeps them alive for the window's lifetime,
            // which spans the whole event loop of this program.
            match unsafe { wl::Wl::new(d.display.as_ptr(), w.surface.as_ptr()) } {
                Ok(wl) => s.wl = Some(wl),
                Err(e) => s.errors.push(format!("wayland init: {e}")),
            }
        }
        _ => {
            s.errors
                .push("not running on Wayland: pen pressure needs a Wayland session".into());
            // Avoid retrying on every event.
            s.wl = None;
            s.args.mode = Mode::Texture;
        }
    }
}

/// A scripted figure-eight stroke at 240 Hz for testing without a pen.
fn demo_samples(s: &mut Shared) -> Vec<wl::PenSample> {
    let now = clock::monotonic_ns();
    let t0 = *s.demo_t0.get_or_insert(now);
    let last = s.last_sample.map_or(t0, |l| l.recv_ns.max(t0));
    let mut out = Vec::new();
    let mut t = last + 4_166_667;
    while t <= now {
        let u = (t - t0) as f64 / 1e9;
        let (w, h) = (s.window_logical.0, s.window_logical.1 - s.toolbar);
        out.push(wl::PenSample {
            x: w / 2.0 + (u * 1.3).sin() * w * 0.35,
            y: s.toolbar + h / 2.0 + (u * 2.6).sin() * h * 0.3,
            pressure: 0.5 + 0.4 * (u * 5.0).sin(),
            tilt_x: 0.0,
            tilt_y: 0.0,
            down: true,
            eraser: false,
            target: wl::Target::Ui,
            event_ns: t,
            recv_ns: t,
            stroke_start: out.is_empty() && s.last_sample.is_none(),
        });
        t += 4_166_667;
    }
    if s.args.mode == Mode::Subsurface {
        let tb = s.toolbar;
        for o in &mut out {
            o.target = wl::Target::Canvas;
            o.y -= tb;
        }
    }
    out
}

#[derive(serde::Serialize)]
struct Summary {
    frames: usize,
    p50_ms: f64,
    p95_ms: f64,
    min_ms: f64,
    max_ms: f64,
}

fn summarize(mut v: Vec<f64>) -> Option<Summary> {
    v.retain(|x| x.is_finite() && *x > -100.0 && *x < 2000.0);
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let pick = |q: f64| v[((v.len() as f64 - 1.0) * q).round() as usize];
    Some(Summary {
        frames: v.len(),
        p50_ms: pick(0.5),
        p95_ms: pick(0.95),
        min_ms: v[0],
        max_ms: v[v.len() - 1],
    })
}

fn build_report(s: &Shared) -> serde_json::Value {
    let by = |tearing: bool, f: fn(&LatencySample) -> f64| {
        summarize(
            s.latencies
                .iter()
                .filter(|l| l.tearing == tearing)
                .map(f)
                .collect(),
        )
    };
    let rate = {
        let w = &s.samples_window;
        if w.len() > 10 {
            let span = (w[w.len() - 1] - w[w.len().saturating_sub(240)]) as f64 / 1e9;
            let n = w.len().min(240) as f64;
            if span > 0.0 { Some(n / span) } else { None }
        } else {
            None
        }
    };
    serde_json::json!({
        "pennoa_spike": "pen",
        "args": s.args,
        "session": {
            "wayland_display": std::env::var("WAYLAND_DISPLAY").ok(),
            "desktop": std::env::var("XDG_CURRENT_DESKTOP").ok(),
            "hyprland": std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok(),
            "scale": s.scale,
            "window_logical": s.window_logical,
        },
        "canvas_present_mode": s.swap.as_ref().map(|sw| format!("{:?}", sw.present_mode_used)),
        "tearing_control_available": s.wl.as_ref().map(|w| w.has_tearing_control()),
        "device": s.wl.as_ref().map(|w| w.state.report.clone()),
        "samples_total": s.samples_total,
        "sample_rate_hz": rate,
        "touches_while_pen_near": s.touches_while_pen_near,
        "frames_discarded": s.discarded,
        "feedback_requested": s.feedback_requested,
        "presented_events": s.presented_events,
        "debug_first_frames_event_recv_presented_ns": s.raw_latency_debug,
        "display_refresh_hz": (s.refresh_ns > 0).then(|| 1e9 / s.refresh_ns as f64),
        "latency": {
            "input_to_present_vsync": by(false, |l| l.input_to_present_ms),
            "input_to_present_tearing": by(true, |l| l.input_to_present_ms),
            "recv_to_present_vsync": by(false, |l| l.recv_to_present_ms),
            "recv_to_present_tearing": by(true, |l| l.recv_to_present_ms),
            "oldest_input_to_present_vsync": by(false, |l| l.oldest_input_to_present_ms),
            "oldest_input_to_present_tearing": by(true, |l| l.oldest_input_to_present_ms),
        },
        "note": "input_to_present uses the compositor's input timestamp (assumed CLOCK_MONOTONIC, presentation clock id 1). Add the display's scan-out/response time (~5-10 ms) for true pen-to-photon.",
        "errors": s.errors,
    })
}

fn stats_line(s: &Shared) -> String {
    let lat = summarize(
        s.latencies
            .iter()
            .rev()
            .take(240)
            .map(|l| l.input_to_present_ms)
            .collect(),
    );
    let pen = s.last_sample.map_or("no pen yet".to_string(), |p| {
        format!(
            "p {:.2}  tilt {:+.0}°/{:+.0}°{}",
            p.pressure,
            p.tilt_x,
            p.tilt_y,
            if p.eraser { "  ERASER" } else { "" }
        )
    });
    let dev =
        s.wl.as_ref()
            .and_then(|w| w.state.report.tablets.first().cloned())
            .unwrap_or_else(|| "no tablet seen".into());
    let lat = lat.map_or("latency: draw something".into(), |l| {
        format!(
            "input→screen p50 {:.1} ms · p95 {:.1} ms ({} frames)",
            l.p50_ms, l.p95_ms, l.frames
        )
    });
    let err = s
        .errors
        .last()
        .map(|e| format!("  ⚠ {e}"))
        .unwrap_or_default();
    format!("{dev} · {pen} · {lat}{err}")
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let shared = Rc::new(RefCell::new(Shared {
        tearing: args.tearing,
        args: args.clone(),
        app: slint::Weak::default(),
        wl: None,
        gpu: None,
        ink: None,
        canvas: None,
        swap: None,
        scale: 1.0,
        window_logical: (1280.0, 800.0),
        toolbar: 64.0,
        next_tag: 1,
        frame: FrameInputs::default(),
        in_flight: HashMap::new(),
        latencies: Vec::new(),
        discarded: 0,
        refresh_ns: 0,
        feedback_requested: 0,
        presented_events: 0,
        raw_latency_debug: Vec::new(),
        pen_near: false,
        touches_while_pen_near: 0,
        samples_total: 0,
        samples_window: Vec::new(),
        last_sample: None,
        demo_t0: None,
        errors: Vec::new(),
    }));

    slint::BackendSelector::new()
        .backend_name("winit".into())
        .require_wgpu_30(slint::wgpu_30::WGPUConfiguration::default())
        .with_winit_custom_application_handler(Handler(shared.clone()))
        .select()?;

    let app = PenSpike::new()?;
    shared.borrow_mut().app = app.as_weak();
    app.set_tearing(args.tearing);
    app.set_show_image(args.mode == Mode::Texture);
    app.set_mode_label(match args.mode {
        Mode::Texture => "Texture mode".into(),
        Mode::Subsurface => "Subsurface mode".into(),
    });

    {
        let h = Handler(shared.clone());
        app.on_clear(move || h.clear());
        let h = Handler(shared.clone());
        app.on_toggle_tearing(move || h.toggle_tearing());
        let h = Handler(shared.clone());
        app.on_save(move || h.save());
    }

    // Slint gives us its GPU device; in texture mode we also draw ink right before it renders.
    let notifier_shared = shared.clone();
    app.window().set_rendering_notifier(move |state, api| {
        let mut s = notifier_shared.borrow_mut();
        match (state, api) {
            (
                slint::RenderingState::RenderingSetup,
                slint::GraphicsAPI::WGPU30 {
                    instance,
                    device,
                    queue,
                    ..
                },
            ) => {
                s.gpu = Some((instance.clone(), device.clone(), queue.clone()));
            }
            (
                slint::RenderingState::BeforeRendering,
                slint::GraphicsAPI::WGPU30 { device, queue, .. },
            ) if s.args.mode == Mode::Texture => {
                let Some(ink) = &mut s.ink else { return };
                ink.flush(device, queue);
                if let Some(frame) = s.take_frame() {
                    let tag = s.next_tag;
                    s.next_tag += 1;
                    let tearing = s.tearing;
                    if let Some(wl) = &s.wl
                        && wl.request_feedback(&wl.ui_surface, tag)
                    {
                        s.feedback_requested += 1;
                        s.in_flight.insert(tag, (frame, tearing));
                    }
                }
            }
            _ => {}
        }
    })?;

    // Stats line, and a wake-up tick for --demo.
    let tick_shared = shared.clone();
    let app_weak = app.as_weak();
    let stats_timer = slint::Timer::default();
    stats_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(250),
        move || {
            let line = stats_line(&tick_shared.borrow());
            if let Some(app) = app_weak.upgrade() {
                app.set_stats(line.into());
            }
        },
    );
    let demo_timer = slint::Timer::default();
    if args.demo {
        demo_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(4),
            || {},
        );
    }

    let quit_timer = slint::Timer::default();
    if let Some(secs) = args.seconds {
        quit_timer.start(
            slint::TimerMode::SingleShot,
            std::time::Duration::from_secs(secs),
            || {
                let _ = slint::quit_event_loop();
            },
        );
    }

    app.run()?;
    Handler(shared.clone()).save();
    // Tear down GPU and Wayland objects now, not during thread-local destruction.
    {
        let mut s = shared.borrow_mut();
        s.swap = None;
        s.canvas = None;
        s.ink = None;
        s.gpu = None;
        s.wl = None;
    }
    drop(app);
    Ok(())
}
