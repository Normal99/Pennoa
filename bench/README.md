# Recognition benchmark data

`pn-bench recog` measures how well each model reads **your** handwriting. Results decide which models Pennoa uses (ADR-0003).

## 1. Prepare 20–30 pages
Put them in a folder **outside this repository**, e.g. `~/pennoa-bench/pages/`. Your notes are private: never commit them.

For each page:

| File | Content |
|---|---|
| `001.jpg` (or `.png`) | A photo or scan of the page: flat, evenly lit, the whole page visible. The phone camera is fine |
| `001.txt` | What the page says, typed exactly as written: same line breaks, your spelling (mistakes included), æ ø å. Write formulas inline between `$…$` |
| `001.math.txt` (optional) | Only for pages with maths or physics: one LaTeX expression per line, e.g. `v = u + at` |

Aim for a realistic mix:
- Norwegian (Bokmål and Nynorsk if you use both) and English;
- tidy and rushed writing;
- several pages with maths or physics formulas;
- one or two pages with drawings or diagrams.

## 2. Configure models
```bash
cp bench/models.example.toml ~/pennoa-bench/models.toml
```
Edit the copy, then start the local models with `llama-server` (see the comments in the file).

## 3. Run
```bash
cargo run --release -p pn-bench -- recog --data ~/pennoa-bench/pages --models ~/pennoa-bench/models.toml --out ~/pennoa-bench/results
```

**Output:**
- `results.md`: one row per model with text CER, WER, æøå recall, maths exact-match and median latency;
- `results.json`: per page;
- `outputs/`: every raw model answer.

**Metrics:**
- **CER/WER** are error rates; lower is better.
- **æøå recall** is the share of your Norwegian letters the model got right.
- **Math exact** counts expressions that match exactly after normalising spacing and `\cdot`/`\times`.

## Speech-to-text
```bash
whisper-server -m ggml-model-q5_0.bin --port 8081   # e.g. NB-Whisper medium from NbAiLab
cargo run --release -p pn-bench -- asr --audio lesson.ogg --reference lesson.txt --language no
```
