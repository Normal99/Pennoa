//! `pn-bench`: evidence for model choices (see docs/PHASE0.md).
//!
//! * `recog`: run every configured vision model over a folder of note pages and
//!   score text (CER, WER, æ/ø/å recall) and math (LaTeX exact match) + latency.
//! * `asr`: transcribe a recording with a whisper.cpp server and score WER/CER.
//! * `score`: compare two text files directly.

mod metrics;
mod providers;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use providers::{Client, ModelConfig};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

const TEXT_PROMPT: &str = "Transcribe the handwritten text in this image exactly as written. \
Keep the original language (Norwegian Bokmål, Nynorsk or English), spelling, line breaks and letters such as æ, ø and å. \
Do not translate, correct or add anything. Write formulas inline as LaTeX between $ signs. Output only the transcription.";

const MATH_PROMPT: &str = "This image contains handwritten mathematics or physics. \
Output every expression or equation as LaTeX, one per line, in reading order, \
without $ signs, numbering, explanations or code fences.";

#[derive(Parser)]
#[command(version, about = "Pennoa benchmarks")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Handwriting recognition benchmark over a folder of page images.
    Recog {
        /// Folder with `<id>.jpg|png` plus `<id>.txt` and/or `<id>.math.txt` ground truth.
        #[arg(long)]
        data: PathBuf,
        /// TOML file with `[[model]]` entries (see bench/models.example.toml).
        #[arg(long)]
        models: PathBuf,
        /// Output folder for raw model outputs and results.json / results.md.
        #[arg(long, default_value = "bench-results")]
        out: PathBuf,
        /// Only run models whose name contains this text.
        #[arg(long)]
        only: Option<String>,
    },
    /// Speech-to-text benchmark against a whisper.cpp `whisper-server`.
    Asr {
        /// e.g. http://127.0.0.1:8081
        #[arg(long, default_value = "http://127.0.0.1:8081")]
        server: String,
        /// Audio file (wav/mp3/ogg/flac, whatever the server accepts).
        #[arg(long)]
        audio: PathBuf,
        /// Reference transcript (plain text).
        #[arg(long)]
        reference: Option<PathBuf>,
        /// Language code, e.g. `no`, `nn`, `en`.
        #[arg(long, default_value = "no")]
        language: String,
        #[arg(long, default_value = "bench-results")]
        out: PathBuf,
    },
    /// Score a hypothesis text file against a reference.
    Score {
        reference: PathBuf,
        hypothesis: PathBuf,
    },
}

#[derive(Deserialize)]
struct ModelsFile {
    model: Vec<ModelConfig>,
}

#[derive(Debug, Serialize)]
struct PageResult {
    model: String,
    page: String,
    task: String,
    latency_ms: f64,
    cer: Option<f64>,
    wer: Option<f64>,
    diacritic_recall: Option<f64>,
    math_exact: Option<(usize, usize)>,
    stop_reason: Option<String>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    error: Option<String>,
}

struct Page {
    id: String,
    image: PathBuf,
    text: Option<String>,
    math: Option<Vec<String>>,
}

fn load_pages(dir: &Path) -> Result<Vec<Page>> {
    let mut pages = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let text = std::fs::read_to_string(dir.join(format!("{id}.txt"))).ok();
        let math = std::fs::read_to_string(dir.join(format!("{id}.math.txt")))
            .ok()
            .map(|s| {
                s.lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect()
            });
        pages.push(Page {
            id,
            image: path,
            text,
            math,
        });
    }
    pages.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(pages)
}

fn media_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
}

fn strip_fences(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_recog(data: &Path, models: &Path, out: &Path, only: Option<&str>) -> Result<()> {
    let cfg: ModelsFile =
        toml::from_str(&std::fs::read_to_string(models)?).context("parsing models file")?;
    let pages = load_pages(data)?;
    anyhow::ensure!(
        !pages.is_empty(),
        "no page images found in {}",
        data.display()
    );
    std::fs::create_dir_all(out.join("outputs"))?;
    let client = Client::new()?;
    let mut results = Vec::new();

    for m in cfg
        .model
        .iter()
        .filter(|m| only.is_none_or(|o| m.name.contains(o)))
    {
        for page in &pages {
            let image = std::fs::read(&page.image)?;
            for task in &m.tasks {
                let (prompt, has_truth) = match task.as_str() {
                    "text" => (TEXT_PROMPT, page.text.is_some()),
                    "math" => (MATH_PROMPT, page.math.is_some()),
                    other => {
                        eprintln!("unknown task {other}, skipping");
                        continue;
                    }
                };
                if !has_truth {
                    continue;
                }
                eprint!("{} · {} · {task} … ", m.name, page.id);
                let t = Instant::now();
                let reply = client.vision(m, &image, media_type(&page.image), prompt);
                let latency_ms = t.elapsed().as_secs_f64() * 1000.0;
                let mut r = PageResult {
                    model: m.name.clone(),
                    page: page.id.clone(),
                    task: task.clone(),
                    latency_ms,
                    cer: None,
                    wer: None,
                    diacritic_recall: None,
                    math_exact: None,
                    stop_reason: None,
                    input_tokens: None,
                    output_tokens: None,
                    error: None,
                };
                match reply {
                    Ok(reply) => {
                        let text = strip_fences(&reply.text);
                        let safe = m.name.replace(['/', ' '], "_");
                        std::fs::write(
                            out.join("outputs")
                                .join(format!("{safe}__{}__{task}.txt", page.id)),
                            &text,
                        )?;
                        r.stop_reason = reply.stop_reason;
                        r.input_tokens = reply.input_tokens;
                        r.output_tokens = reply.output_tokens;
                        if task == "text" {
                            let truth = page.text.as_deref().unwrap_or_default();
                            r.cer = Some(metrics::cer(truth, &text));
                            r.wer = Some(metrics::wer(truth, &text));
                            r.diacritic_recall = metrics::diacritic_recall(truth, &text);
                            eprintln!(
                                "CER {:.1}% · {:.1} s",
                                r.cer.unwrap_or(1.0) * 100.0,
                                latency_ms / 1000.0
                            );
                        } else {
                            let truth = page.math.as_deref().unwrap_or_default();
                            let lines: Vec<&str> = text
                                .lines()
                                .map(str::trim)
                                .filter(|l| !l.is_empty())
                                .collect();
                            let hits = truth
                                .iter()
                                .filter(|t| lines.iter().any(|l| metrics::latex_exact(t, l)))
                                .count();
                            r.math_exact = Some((hits, truth.len()));
                            eprintln!(
                                "math {hits}/{} exact · {:.1} s",
                                truth.len(),
                                latency_ms / 1000.0
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("error: {e:#}");
                        r.error = Some(format!("{e:#}"));
                    }
                }
                results.push(r);
            }
        }
    }

    std::fs::write(
        out.join("results.json"),
        serde_json::to_string_pretty(&results)?,
    )?;
    let md = summary_markdown(&results);
    std::fs::write(out.join("results.md"), &md)?;
    println!("{md}");
    Ok(())
}

fn summary_markdown(results: &[PageResult]) -> String {
    let mut names: Vec<&str> = results.iter().map(|r| r.model.as_str()).collect();
    names.dedup();
    let mut md = String::from(
        "| Model | Text CER | Text WER | æøå recall | Math exact | Median latency | Errors |\n|---|---|---|---|---|---|---|\n",
    );
    let mean = |v: Vec<f64>| (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64);
    let pct = |x: Option<f64>| x.map_or("—".into(), |x| format!("{:.1}%", x * 100.0));
    for name in names {
        let rs: Vec<&PageResult> = results.iter().filter(|r| r.model == name).collect();
        let cer = mean(rs.iter().filter_map(|r| r.cer).collect());
        let wer = mean(rs.iter().filter_map(|r| r.wer).collect());
        let dia = mean(rs.iter().filter_map(|r| r.diacritic_recall).collect());
        let (hit, tot) = rs
            .iter()
            .filter_map(|r| r.math_exact)
            .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
        let mut lat: Vec<f64> = rs
            .iter()
            .filter(|r| r.error.is_none())
            .map(|r| r.latency_ms)
            .collect();
        lat.sort_by(f64::total_cmp);
        let med = lat
            .get(lat.len() / 2)
            .map_or("—".into(), |m| format!("{:.1} s", m / 1000.0));
        let errors = rs.iter().filter(|r| r.error.is_some()).count();
        let math = if tot > 0 {
            format!("{hit}/{tot} ({:.0}%)", hit as f64 * 100.0 / tot as f64)
        } else {
            "—".into()
        };
        md.push_str(&format!(
            "| {name} | {} | {} | {} | {math} | {med} | {errors} |\n",
            pct(cer),
            pct(wer),
            pct(dia)
        ));
    }
    md
}

fn run_asr(
    server: &str,
    audio: &Path,
    reference: Option<&Path>,
    language: &str,
    out: &Path,
) -> Result<()> {
    std::fs::create_dir_all(out)?;
    let client = Client::new()?;
    let bytes = std::fs::read(audio)?;
    let file_name = audio
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.wav")
        .to_string();
    let form = reqwest::blocking::multipart::Form::new()
        .part(
            "file",
            reqwest::blocking::multipart::Part::bytes(bytes).file_name(file_name),
        )
        .text("response_format", "json")
        .text("language", language.to_string())
        .text("temperature", "0");
    let t = Instant::now();
    let resp = client
        .http()
        .post(format!("{}/inference", server.trim_end_matches('/')))
        .multipart(form)
        .send()?;
    let secs = t.elapsed().as_secs_f64();
    let status = resp.status();
    let v: serde_json::Value = resp.json().context("whisper-server reply was not JSON")?;
    anyhow::ensure!(status.is_success(), "{status}: {v}");
    let text = v["text"].as_str().unwrap_or_default().trim().to_string();
    std::fs::write(out.join("asr-output.txt"), &text)?;
    println!(
        "transcribed in {secs:.1} s → {}",
        out.join("asr-output.txt").display()
    );
    if let Some(r) = reference {
        let truth = std::fs::read_to_string(r)?;
        println!(
            "WER {:.1}% · CER {:.1}% · æøå recall {}",
            metrics::wer(&truth, &text) * 100.0,
            metrics::cer(&truth, &text) * 100.0,
            metrics::diacritic_recall(&truth, &text)
                .map_or("—".into(), |d| format!("{:.1}%", d * 100.0))
        );
    }
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Recog {
            data,
            models,
            out,
            only,
        } => run_recog(&data, &models, &out, only.as_deref()),
        Cmd::Asr {
            server,
            audio,
            reference,
            language,
            out,
        } => run_asr(&server, &audio, reference.as_deref(), &language, &out),
        Cmd::Score {
            reference,
            hypothesis,
        } => {
            let r = std::fs::read_to_string(reference)?;
            let h = std::fs::read_to_string(hypothesis)?;
            println!(
                "CER {:.2}% · WER {:.2}% · æøå recall {}",
                metrics::cer(&r, &h) * 100.0,
                metrics::wer(&r, &h) * 100.0,
                metrics::diacritic_recall(&r, &h)
                    .map_or("—".into(), |d| format!("{:.1}%", d * 100.0))
            );
            Ok(())
        }
    }
}
