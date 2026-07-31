# Render benchmarks

Fixture: `fixtures/heavy-async.md` (48 Mermaid diagrams + 24 Liquid fences + 8 Rust blocks).

Host: 4× Intel Xeon @ 2400 MHz. Each label is 2 warmups + 7 timed `/usr/bin/time -v` runs.

Hardware PMU cycle counters were unavailable (`perf_event_open` → EACCES).
`estimated_cpu_cycles` = `(user_sec + sys_sec) * cpu_mhz * 1e6`.

## Results (release binaries)

Initial async implementation (Mermaid + Liquid deferred) vs sync `main`:

| Metric | Sync (`main`) | Async (Tokio `spawn_blocking`) | Delta |
| --- | ---: | ---: | ---: |
| Wall time avg | 0.874 s | 0.309 s | **-64.7%** |
| Wall time min / max | 0.870 / 0.880 s | 0.300 / 0.320 s | |
| User CPU avg | 0.861 s | 0.896 s | +4.0% |
| Sys CPU avg | 0.019 s | 0.026 s | +38.5% |
| CPU time avg | 0.880 s | 0.921 s | +4.7% |
| CPU utilization avg | 101% | 296% | +193% |
| Max RSS peak | 65,752 KB | 71,300 KB | +8.4% |
| Max RSS avg | 65,286 KB | 70,563 KB | +8.1% |
| Estimated CPU cycles avg | 2.112e9 | 2.211e9 | +4.7% |
| Estimated CPU cycles peak | 2.136e9 | 2.256e9 | +5.6% |
| Voluntary ctx switches avg | 78 | 575 | +634% |
| Involuntary ctx switches avg | 605 | 822 | +36% |
| PDF size | 511,169 B | 511,169 B | same |

### Verdict

On Mermaid-heavy input, async overlap helps a lot: wall clock drops by about two thirds because independent Mermaid jobs use multiple cores (~3× CPU%). Total CPU work and peak RAM rise only modestly. For documents with few diagrams, expect little or no wall-clock win (Typst layout still serial). Follow-up hardening keeps Liquid synchronous and merges Mermaid output via structured segments.

## Multi-file `--jobs` batch

Host: 4× Intel Xeon @ 2400 MHz. Harness: `scripts/bench_batch.py` (Python
`perf_counter` + `RUSAGE_CHILDREN`; GNU `time` unavailable in this environment).

### Light docs — 8× `example.md`, `--output-mode merge`

| Metric | `-j 1` | `-j 4` | Delta |
| --- | ---: | ---: | ---: |
| Wall time avg | 0.101 s | 0.041 s | **-59.5%** (2.47×) |
| CPU time avg | 0.105 s | 0.124 s | +18.7% |
| CPU utilization avg | 104% | 304% | +192% |
| Max RSS avg | 54,880 KB | 65,916 KB | +20.1% |

Zip and files modes on the same inputs are similar (~2.4–2.6× wall speedup).

### Heavy Mermaid — 4× `heavy-async.md`, `--output-mode merge`

| Metric | `-j 1` | `-j 4` | Delta |
| --- | ---: | ---: | ---: |
| Wall time avg | 1.251 s | 0.936 s | **-25.2%** (1.34×) |
| CPU time avg | 3.562 s | 3.494 s | −1.9% |
| CPU utilization avg | 285% | 373% | +31% |
| Max RSS avg | 136,704 KB | 138,865 KB | +1.6% |

Even with `-j 1`, CPU% is already ~3× because each file still parallelizes Mermaid
internally. Extra file-level jobs help less here (cores already busy) but still
cut wall time by about a quarter.

### Verdict

File-level `--jobs` pays off most on many moderate documents. Mermaid-heavy
single documents already saturate cores via the Tokio pool, so batch parallelism
adds a smaller wall-clock win and a bit more RAM.

Raw JSON under `results/` is gitignored (machine-specific). Write fresh files locally when you re-run the harness.

Reproduce:

```bash
cargo build --release
mkdir -p benches/results
python3 scripts/bench_render.py \
  --binary target/release/md2pdf \
  --source benches/fixtures/heavy-async.md \
  --label async --runs 7 --warmup 2 \
  --out benches/results/async.json

python3 scripts/bench_batch.py \
  --binary target/release/md2pdf \
  --fixture example.md \
  --copies 8 --mode merge --jobs 1,4 \
  --runs 7 --warmup 2 \
  --out benches/results/batch-example-merge.json
```
