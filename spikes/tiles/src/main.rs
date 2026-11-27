//! Phase 0 spike: can vello draw and cache pages of handwriting fast enough?
//!
//! Generates a synthetic A4 page of handwriting-like strokes (pn-ink samples),
//! turns each stroke into a variable-width outline, renders the page into
//! 512 px tiles at several zoom levels and reports timings as JSON.
//!
//! ```text
//! cargo run -p spike-tiles --release -- --out spikes/tiles/out
//! ```

mod outline;
mod page;

use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;
use std::time::Instant;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color, Fill};
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene, wgpu};

const TILE: u32 = 512;

#[derive(Parser)]
struct Args {
    /// Lines of handwriting on the page (an A4 ruled page has ~27).
    #[arg(long, default_value_t = 25)]
    lines: usize,
    /// Zoom levels in pixels per millimetre (4.6 ≈ page fits a 14" 1920×1200 screen).
    #[arg(long, value_delimiter = ',', default_values_t = [4.6, 9.2, 18.4])]
    zooms: Vec<f64>,
    /// Directory for sample tile PNGs and report.json.
    #[arg(long, default_value = "spikes/tiles/out")]
    out: std::path::PathBuf,
    /// Repeat each tile render this many times (first run includes warm-up).
    #[arg(long, default_value_t = 3)]
    repeats: usize,
}

#[derive(Serialize)]
struct ZoomReport {
    px_per_mm: f64,
    tiles: usize,
    tiles_with_ink: usize,
    tile_ms_median: f64,
    tile_ms_p95: f64,
    tile_ms_max: f64,
    full_page_ms: f64,
}

#[derive(Serialize)]
struct Report {
    adapter: String,
    backend: String,
    strokes: usize,
    samples: usize,
    encoded_bytes: usize,
    bytes_per_sample: f64,
    outline_build_ms: f64,
    zooms: Vec<ZoomReport>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    std::fs::create_dir_all(&args.out)?;

    // --- content ---------------------------------------------------------
    let strokes = page::synthetic_page(args.lines);
    let samples: usize = strokes.iter().map(|s| s.samples.len()).sum();
    let encoded_bytes: usize = strokes
        .iter()
        .map(|s| pn_ink::codec::encode(&s.samples).len())
        .sum();

    let t = Instant::now();
    let outlines: Vec<_> = strokes
        .iter()
        .map(|s| (outline::stroke_outline(s), page::color(s.color)))
        .collect();
    let outline_build_ms = ms(t);

    // --- GPU -------------------------------------------------------------
    let mut ctx = vello::util::RenderContext::new();
    let dev_id = pollster::block_on(ctx.device(None)).context("no compatible GPU adapter")?;
    let handle = &ctx.devices[dev_id];
    let (device, queue) = (&handle.device, &handle.queue);
    let info = handle.adapter().get_info();
    eprintln!("adapter: {} ({:?})", info.name, info.backend);

    let mut renderer = Renderer::new(
        device,
        RendererOptions {
            antialiasing_support: vello::AaSupport::area_only(),
            ..Default::default()
        },
    )
    .map_err(|e| anyhow::anyhow!("renderer: {e}"))?;

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("tile"),
        size: wgpu::Extent3d {
            width: TILE,
            height: TILE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    let mut zooms = Vec::new();
    for &zoom in &args.zooms {
        let page_px_w = (page::PAGE_W_MM * zoom).ceil() as u32;
        let page_px_h = (page::PAGE_H_MM * zoom).ceil() as u32;
        let (cols, rows) = (page_px_w.div_ceil(TILE), page_px_h.div_ceil(TILE));

        let mut times = Vec::new();
        let mut with_ink = 0;
        let page_start = Instant::now();
        for row in 0..rows {
            for col in 0..cols {
                // Tile rect in page millimetres; only strokes touching it are encoded.
                let tile_mm = Rect::new(
                    (col * TILE) as f64 / zoom,
                    (row * TILE) as f64 / zoom,
                    ((col + 1) * TILE) as f64 / zoom,
                    ((row + 1) * TILE) as f64 / zoom,
                );
                let xf = Affine::translate((-((col * TILE) as f64), -((row * TILE) as f64)))
                    * Affine::scale(zoom);
                let mut scene = Scene::new();
                let mut inked = false;
                for (path, bounds, color) in outlines.iter().map(|((p, b), c)| (p, b, c)) {
                    if bounds.intersect(tile_mm).area() > 0.0 {
                        scene.fill(Fill::NonZero, xf, *color, None, path);
                        inked = true;
                    }
                }
                with_ink += inked as usize;
                for rep in 0..args.repeats {
                    let t = Instant::now();
                    renderer
                        .render_to_texture(
                            device,
                            queue,
                            &scene,
                            &view,
                            &RenderParams {
                                base_color: Color::WHITE,
                                width: TILE,
                                height: TILE,
                                antialiasing_method: AaConfig::Area,
                            },
                        )
                        .map_err(|e| anyhow::anyhow!("render: {e}"))?;
                    device.poll(wgpu::PollType::wait_indefinitely())?;
                    if rep > 0 || args.repeats == 1 {
                        times.push(ms(t));
                    }
                }
                if row == 1 && col == 0 {
                    save_png(
                        device,
                        queue,
                        &target,
                        &args.out.join(format!("tile_{zoom}.png")),
                    )?;
                }
            }
        }
        let full_page_ms = ms(page_start) / args.repeats as f64;
        times.sort_by(f64::total_cmp);
        zooms.push(ZoomReport {
            px_per_mm: zoom,
            tiles: (cols * rows) as usize,
            tiles_with_ink: with_ink,
            tile_ms_median: times[times.len() / 2],
            tile_ms_p95: times[(times.len() * 95 / 100).min(times.len() - 1)],
            tile_ms_max: *times.last().unwrap(),
            full_page_ms,
        });
    }

    let report = Report {
        adapter: info.name.clone(),
        backend: format!("{:?}", info.backend),
        strokes: strokes.len(),
        samples,
        encoded_bytes,
        bytes_per_sample: encoded_bytes as f64 / samples as f64,
        outline_build_ms,
        zooms,
    };
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(args.out.join("report.json"), &json)?;
    println!("{json}");
    Ok(())
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn save_png(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    tex: &wgpu::Texture,
    path: &std::path::Path,
) -> Result<()> {
    let bytes_per_row = TILE * 4; // 2048, already a multiple of 256
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (bytes_per_row * TILE) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: TILE,
            height: TILE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([enc.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let data = slice.get_mapped_range()?.to_vec();
    image::save_buffer(path, &data, TILE, TILE, image::ColorType::Rgba8)?;
    Ok(())
}
