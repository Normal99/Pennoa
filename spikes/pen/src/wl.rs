//! Our own Wayland client objects on Slint's (winit's) `wl_display`:
//! tablet-v2 pen input, presentation-time feedback, tearing control and an
//! optional canvas subsurface.
//!
//! Events are read from the socket by winit's event loop; we only dispatch our
//! own event queue (`dispatch_pending`) from the winit application handler.

use crate::clock;
use std::collections::HashMap;
use wayland_client::backend::ObjectId;
use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::{
    wl_compositor::WlCompositor, wl_output::WlOutput, wl_region::WlRegion, wl_registry::WlRegistry,
    wl_seat::WlSeat, wl_subcompositor::WlSubcompositor, wl_subsurface::WlSubsurface,
    wl_surface::WlSurface,
};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum, delegate_noop, event_created_child,
};
use wayland_protocols::wp::presentation_time::client::{wp_presentation, wp_presentation_feedback};
use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2::ZwpTabletManagerV2, zwp_tablet_pad_dial_v2::ZwpTabletPadDialV2,
    zwp_tablet_pad_group_v2, zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2,
    zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2, zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2,
    zwp_tablet_pad_v2, zwp_tablet_pad_v2::ZwpTabletPadV2, zwp_tablet_seat_v2,
    zwp_tablet_seat_v2::ZwpTabletSeatV2, zwp_tablet_tool_v2, zwp_tablet_tool_v2::ZwpTabletToolV2,
    zwp_tablet_v2, zwp_tablet_v2::ZwpTabletV2,
};
use wayland_protocols::wp::tearing_control::v1::client::{
    wp_tearing_control_manager_v1::WpTearingControlManagerV1,
    wp_tearing_control_v1::{PresentationHint, WpTearingControlV1},
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};

/// Which of our surfaces a pen sample landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Slint's main window surface (toolbar, or the canvas in texture mode).
    Ui,
    /// Our canvas subsurface (subsurface mode).
    Canvas,
    Other,
}

/// One complete pen frame (all axes updated together), surface-local logical px.
#[derive(Debug, Clone, Copy)]
pub struct PenSample {
    pub x: f64,
    pub y: f64,
    pub pressure: f64,
    pub tilt_x: f64,
    pub tilt_y: f64,
    pub down: bool,
    pub eraser: bool,
    pub target: Target,
    /// Compositor event time, reconstructed to CLOCK_MONOTONIC nanoseconds.
    pub event_ns: u64,
    /// When our process dispatched the event (CLOCK_MONOTONIC ns).
    pub recv_ns: u64,
    pub stroke_start: bool,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct DeviceReport {
    pub tablets: Vec<String>,
    pub tools: Vec<String>,
    pub tool_capabilities: Vec<String>,
    pub pads: usize,
    pub pressure_min: Option<f64>,
    pub pressure_max: Option<f64>,
    pub tilt_seen: bool,
    pub eraser_seen: bool,
    pub buttons_seen: Vec<u32>,
    pub frames: u64,
    pub presentation_clock_id: Option<u32>,
    pub globals: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct Presented {
    pub tag: u64,
    pub presented_ns: Option<u64>,
    pub refresh_ns: u32,
    pub flags: u32,
}

#[derive(Default)]
struct ToolState {
    kind: Option<zwp_tablet_tool_v2::Type>,
    in_prox: Option<Target>,
    down: bool,
    x: f64,
    y: f64,
    pressure: f64,
    tilt: (f64, f64),
    stroke_started: bool,
}

pub struct WlState {
    pub ui_surface: ObjectId,
    pub canvas_surface: Option<ObjectId>,
    tools: HashMap<ObjectId, ToolState>,
    pub samples: Vec<PenSample>,
    pub presented: Vec<Presented>,
    pub report: DeviceReport,
}

impl WlState {
    fn target(&self, surface: &WlSurface) -> Target {
        let id = surface.id();
        if id == self.ui_surface {
            Target::Ui
        } else if Some(&id) == self.canvas_surface.as_ref() {
            Target::Canvas
        } else {
            Target::Other
        }
    }
}

/// Our Wayland connection objects.
pub struct Wl {
    pub conn: Connection,
    pub queue: EventQueue<WlState>,
    pub qh: QueueHandle<WlState>,
    pub state: WlState,
    pub display_ptr: *mut std::ffi::c_void,
    pub ui_surface: WlSurface,
    presentation: Option<wp_presentation::WpPresentation>,
    tearing_mgr: Option<WpTearingControlManagerV1>,
    compositor: Option<WlCompositor>,
    subcompositor: Option<WlSubcompositor>,
    viewporter: Option<WpViewporter>,
    _globals: GlobalList,
    _tablet_seat: Option<ZwpTabletSeatV2>,
}

/// A canvas subsurface with its own buffers, positioned over the canvas area.
pub struct CanvasSurface {
    pub surface: WlSurface,
    pub subsurface: WlSubsurface,
    pub viewport: Option<WpViewport>,
    pub tearing: Option<WpTearingControlV1>,
}

impl Wl {
    /// # Safety
    /// `display` and `surface` must be the live `wl_display` / `wl_surface` of
    /// Slint's window and must outlive this object.
    pub unsafe fn new(
        display: *mut std::ffi::c_void,
        surface: *mut std::ffi::c_void,
    ) -> anyhow::Result<Self> {
        // SAFETY: caller guarantees a live wl_display owned by winit.
        let backend =
            unsafe { wayland_backend::client::Backend::from_foreign_display(display.cast()) };
        let conn = Connection::from_backend(backend);
        // SAFETY: caller guarantees a live wl_surface proxy on that display.
        let surface_id = unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.cast()) }?;
        let ui_surface = WlSurface::from_id(&conn, surface_id.clone())?;

        let (globals, mut queue) = registry_queue_init::<WlState>(&conn)?;
        let qh = queue.handle();
        let mut state = WlState {
            ui_surface: surface_id,
            canvas_surface: None,
            tools: HashMap::new(),
            samples: Vec::new(),
            presented: Vec::new(),
            report: DeviceReport::default(),
        };
        state.report.globals = globals
            .contents()
            .clone_list()
            .into_iter()
            .map(|g| format!("{} v{}", g.interface, g.version))
            .collect();

        let seat: Option<WlSeat> = globals.bind(&qh, 1..=7, ()).ok();
        let tablet_mgr: Option<ZwpTabletManagerV2> = globals.bind(&qh, 1..=2, ()).ok();
        let tablet_seat = match (&tablet_mgr, &seat) {
            (Some(m), Some(s)) => Some(m.get_tablet_seat(s, &qh, ())),
            _ => None,
        };
        let presentation = globals.bind(&qh, 1..=2, ()).ok();
        let tearing_mgr = globals.bind(&qh, 1..=1, ()).ok();
        let compositor = globals.bind(&qh, 4..=6, ()).ok();
        let subcompositor = globals.bind(&qh, 1..=1, ()).ok();
        let viewporter = globals.bind(&qh, 1..=1, ()).ok();
        queue.roundtrip(&mut state)?;

        if tablet_mgr.is_none() {
            eprintln!("warning: compositor has no zwp_tablet_manager_v2 — no pen pressure/tilt");
        }
        Ok(Self {
            conn,
            queue,
            qh,
            state,
            display_ptr: display,
            ui_surface,
            presentation,
            tearing_mgr,
            compositor,
            subcompositor,
            viewporter,
            _globals: globals,
            _tablet_seat: tablet_seat,
        })
    }

    /// Process events winit has already read from the socket.
    pub fn dispatch(&mut self) {
        if let Err(e) = self.queue.dispatch_pending(&mut self.state) {
            eprintln!("wayland dispatch error: {e}");
        }
        let _ = self.conn.flush();
    }

    /// Ask for presentation feedback on the next commit of `surface`.
    pub fn request_feedback(&self, surface: &WlSurface, tag: u64) -> bool {
        match &self.presentation {
            Some(p) => {
                p.feedback(surface, &self.qh, tag);
                let _ = self.conn.flush();
                true
            }
            None => false,
        }
    }

    pub fn has_tearing_control(&self) -> bool {
        self.tearing_mgr.is_some()
    }

    /// Create a desynchronised subsurface over the canvas area (logical px).
    pub fn create_canvas(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> anyhow::Result<CanvasSurface> {
        let compositor = self
            .compositor
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no wl_compositor"))?;
        let sub = self
            .subcompositor
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no wl_subcompositor"))?;
        let surface = compositor.create_surface(&self.qh, ());
        let subsurface = sub.get_subsurface(&surface, &self.ui_surface, &self.qh, ());
        subsurface.set_position(x, y);
        subsurface.place_above(&self.ui_surface);
        subsurface.set_desync();
        let viewport = self.viewporter.as_ref().map(|v| {
            let vp = v.get_viewport(&surface, &self.qh, ());
            vp.set_destination(w, h);
            vp
        });
        let tearing = self
            .tearing_mgr
            .as_ref()
            .map(|m| m.get_tearing_control(&surface, &self.qh, ()));
        self.state.canvas_surface = Some(surface.id());
        // The parent must commit for the subsurface position to apply; Slint commits on its next frame.
        let _ = self.conn.flush();
        Ok(CanvasSurface {
            surface,
            subsurface,
            viewport,
            tearing,
        })
    }
}

impl CanvasSurface {
    pub fn set_tearing(&self, on: bool) {
        if let Some(t) = &self.tearing {
            t.set_presentation_hint(if on {
                PresentationHint::Async
            } else {
                PresentationHint::Vsync
            });
        }
    }

    pub fn resize(&self, x: i32, y: i32, w: i32, h: i32) {
        self.subsurface.set_position(x, y);
        if let Some(vp) = &self.viewport {
            vp.set_destination(w, h);
        }
    }

    pub fn surface_ptr(&self) -> *mut std::ffi::c_void {
        self.surface.id().as_ptr().cast()
    }
}

// ---------------------------------------------------------------------------
// Dispatch implementations
// ---------------------------------------------------------------------------

impl Dispatch<WlRegistry, GlobalListContents> for WlState {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wayland_client::protocol::wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(WlState: ignore WlSeat);
delegate_noop!(WlState: ignore WlOutput);
delegate_noop!(WlState: ignore WlSurface);
delegate_noop!(WlState: ignore WlRegion);
delegate_noop!(WlState: WlCompositor);
delegate_noop!(WlState: WlSubcompositor);
delegate_noop!(WlState: WlSubsurface);
delegate_noop!(WlState: WpViewporter);
delegate_noop!(WlState: WpViewport);
delegate_noop!(WlState: WpTearingControlManagerV1);
delegate_noop!(WlState: WpTearingControlV1);
delegate_noop!(WlState: ZwpTabletManagerV2);
delegate_noop!(WlState: ignore ZwpTabletPadRingV2);
delegate_noop!(WlState: ignore ZwpTabletPadStripV2);
delegate_noop!(WlState: ignore ZwpTabletPadDialV2);

impl Dispatch<wp_presentation::WpPresentation, ()> for WlState {
    fn event(
        state: &mut Self,
        _: &wp_presentation::WpPresentation,
        event: wp_presentation::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_presentation::Event::ClockId { clk_id } = event {
            state.report.presentation_clock_id = Some(clk_id);
        }
    }
}

impl Dispatch<wp_presentation_feedback::WpPresentationFeedback, u64> for WlState {
    fn event(
        state: &mut Self,
        _: &wp_presentation_feedback::WpPresentationFeedback,
        event: wp_presentation_feedback::Event,
        tag: &u64,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wp_presentation_feedback::Event::Presented {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                refresh,
                flags,
                ..
            } => {
                let secs = (u64::from(tv_sec_hi) << 32) | u64::from(tv_sec_lo);
                let flags = match flags {
                    WEnum::Value(f) => f.bits(),
                    WEnum::Unknown(u) => u,
                };
                state.presented.push(Presented {
                    tag: *tag,
                    presented_ns: Some(secs * 1_000_000_000 + u64::from(tv_nsec)),
                    refresh_ns: refresh,
                    flags,
                });
            }
            wp_presentation_feedback::Event::Discarded => {
                state.presented.push(Presented {
                    tag: *tag,
                    presented_ns: None,
                    refresh_ns: 0,
                    flags: 0,
                });
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwpTabletSeatV2, ()> for WlState {
    fn event(
        state: &mut Self,
        _: &ZwpTabletSeatV2,
        event: zwp_tablet_seat_v2::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwp_tablet_seat_v2::Event::ToolAdded { id } => {
                state.tools.insert(id.id(), ToolState::default());
            }
            zwp_tablet_seat_v2::Event::PadAdded { .. } => state.report.pads += 1,
            _ => {}
        }
    }

    event_created_child!(WlState, ZwpTabletSeatV2, [
        zwp_tablet_seat_v2::EVT_TABLET_ADDED_OPCODE => (ZwpTabletV2, ()),
        zwp_tablet_seat_v2::EVT_TOOL_ADDED_OPCODE => (ZwpTabletToolV2, ()),
        zwp_tablet_seat_v2::EVT_PAD_ADDED_OPCODE => (ZwpTabletPadV2, ()),
    ]);
}

impl Dispatch<ZwpTabletV2, ()> for WlState {
    fn event(
        state: &mut Self,
        _: &ZwpTabletV2,
        event: zwp_tablet_v2::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwp_tablet_v2::Event::Name { name } => state.report.tablets.push(name),
            zwp_tablet_v2::Event::Id { vid, pid } => {
                if let Some(last) = state.report.tablets.last_mut() {
                    last.push_str(&format!(" [{vid:04x}:{pid:04x}]"));
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwpTabletPadV2, ()> for WlState {
    fn event(
        _: &mut Self,
        _: &ZwpTabletPadV2,
        _: zwp_tablet_pad_v2::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }

    event_created_child!(WlState, ZwpTabletPadV2, [
        zwp_tablet_pad_v2::EVT_GROUP_OPCODE => (ZwpTabletPadGroupV2, ()),
    ]);
}

impl Dispatch<ZwpTabletPadGroupV2, ()> for WlState {
    fn event(
        _: &mut Self,
        _: &ZwpTabletPadGroupV2,
        _: zwp_tablet_pad_group_v2::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }

    event_created_child!(WlState, ZwpTabletPadGroupV2, [
        zwp_tablet_pad_group_v2::EVT_RING_OPCODE => (ZwpTabletPadRingV2, ()),
        zwp_tablet_pad_group_v2::EVT_STRIP_OPCODE => (ZwpTabletPadStripV2, ()),
        zwp_tablet_pad_group_v2::EVT_DIAL_OPCODE => (ZwpTabletPadDialV2, ()),
    ]);
}

impl Dispatch<ZwpTabletToolV2, ()> for WlState {
    fn event(
        state: &mut Self,
        tool: &ZwpTabletToolV2,
        event: zwp_tablet_tool_v2::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use zwp_tablet_tool_v2::Event as E;
        let id = tool.id();
        // Resolve the proximity target before borrowing the tool mutably.
        let prox_target = if let E::ProximityIn { surface, .. } = &event {
            Some(state.target(surface))
        } else {
            None
        };
        let ts = state.tools.entry(id).or_default();
        match event {
            E::Type {
                tool_type: WEnum::Value(t),
            } => {
                ts.kind = Some(t);
                state.report.tools.push(format!("{t:?}"));
                if t == zwp_tablet_tool_v2::Type::Eraser {
                    state.report.eraser_seen = true;
                }
            }
            E::Capability {
                capability: WEnum::Value(c),
            } => {
                let s = format!("{c:?}");
                if !state.report.tool_capabilities.contains(&s) {
                    state.report.tool_capabilities.push(s);
                }
            }
            E::ProximityIn { .. } => ts.in_prox = prox_target,
            E::ProximityOut => {
                ts.in_prox = None;
                ts.down = false;
            }
            E::Down { .. } => {
                ts.down = true;
                ts.stroke_started = false;
            }
            E::Up => ts.down = false,
            E::Motion { x, y } => {
                ts.x = x;
                ts.y = y;
            }
            E::Pressure { pressure } => ts.pressure = f64::from(pressure) / 65535.0,
            E::Tilt { tilt_x, tilt_y } => {
                ts.tilt = (tilt_x, tilt_y);
                if tilt_x != 0.0 || tilt_y != 0.0 {
                    state.report.tilt_seen = true;
                }
            }
            E::Button { button, .. } => {
                if !state.report.buttons_seen.contains(&button) {
                    state.report.buttons_seen.push(button);
                }
            }
            E::Frame { time } => {
                state.report.frames += 1;
                let Some(target) = ts.in_prox else { return };
                let now = clock::monotonic_ns();
                let stroke_start = ts.down && !ts.stroke_started;
                if ts.down {
                    ts.stroke_started = true;
                    let p = ts.pressure;
                    state.report.pressure_min =
                        Some(state.report.pressure_min.map_or(p, |m: f64| m.min(p)));
                    state.report.pressure_max =
                        Some(state.report.pressure_max.map_or(p, |m: f64| m.max(p)));
                }
                state.samples.push(PenSample {
                    x: ts.x,
                    y: ts.y,
                    pressure: ts.pressure,
                    tilt_x: ts.tilt.0,
                    tilt_y: ts.tilt.1,
                    down: ts.down,
                    eraser: ts.kind == Some(zwp_tablet_tool_v2::Type::Eraser),
                    target,
                    event_ns: clock::reconstruct_ms_timestamp(time, now),
                    recv_ns: now,
                    stroke_start,
                });
            }
            _ => {}
        }
    }
}
