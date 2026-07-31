#!/usr/bin/env python3
"""Benchmark multi-file md2pdf conversion across --jobs and --output-mode."""

from __future__ import annotations

import argparse
import json
import os
import resource
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def cpu_mhz() -> float:
    text = Path("/proc/cpuinfo").read_text(encoding="utf-8")
    matches = []
    for line in text.splitlines():
        if line.startswith("cpu MHz"):
            matches.append(float(line.split(":")[1].strip()))
    if not matches:
        return 2400.0
    return statistics.mean(matches)


def prepare_sources(fixture: Path, count: int, work: Path) -> Path:
    root = work / "inputs"
    root.mkdir(parents=True, exist_ok=True)
    for index in range(count):
        shutil.copy2(fixture, root / f"doc-{index:02d}.md")
    return root


def run_once(
    binary: Path,
    inputs: Path,
    mode: str,
    jobs: int,
    label: str,
    index: int,
) -> dict:
    with tempfile.TemporaryDirectory(prefix="md2pdf-batch-") as tmp:
        tmp_path = Path(tmp)
        if mode == "merge":
            output = tmp_path / f"{label}-{index}.pdf"
        elif mode == "zip":
            output = tmp_path / f"{label}-{index}.zip"
        else:
            output = tmp_path / "out"
            output.mkdir()

        pattern = str(inputs / "*.md")
        cmd = [
            str(binary),
            pattern,
            "--output-mode",
            mode,
            "--jobs",
            str(jobs),
            "--quiet",
            "--output",
            str(output),
        ]

        ru_before = resource.getrusage(resource.RUSAGE_CHILDREN)
        started = time.perf_counter()
        proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
        elapsed = time.perf_counter() - started
        ru_after = resource.getrusage(resource.RUSAGE_CHILDREN)

        if proc.returncode != 0:
            raise RuntimeError(
                f"md2pdf failed ({proc.returncode}):\n{proc.stdout}\n{proc.stderr}"
            )

        if mode == "files":
            artifacts = sorted(output.glob("*.pdf"))
            artifact_bytes = sum(path.stat().st_size for path in artifacts)
            artifact_count = len(artifacts)
        else:
            artifact_bytes = output.stat().st_size
            artifact_count = 1

        user_sec = ru_after.ru_utime - ru_before.ru_utime
        sys_sec = ru_after.ru_stime - ru_before.ru_stime
        cpu_sec = user_sec + sys_sec
        cpu_percent = int(round((cpu_sec / elapsed) * 100)) if elapsed > 0 else 0
        # Linux reports ru_maxrss in kilobytes.
        max_rss_kb = int(ru_after.ru_maxrss)
        mhz = cpu_mhz()

        return {
            "ok": True,
            "artifact_bytes": artifact_bytes,
            "artifact_count": artifact_count,
            "jobs": jobs,
            "mode": mode,
            "elapsed_sec": elapsed,
            "user_sec": user_sec,
            "sys_sec": sys_sec,
            "cpu_percent": cpu_percent,
            "max_rss_kb": max_rss_kb,
            "cpu_mhz": mhz,
            "estimated_cpu_cycles": int(cpu_sec * mhz * 1_000_000),
        }


def summarize(runs: list[dict]) -> dict:
    def avg(key: str) -> float:
        return statistics.mean(run[key] for run in runs)

    return {
        "runs": len(runs),
        "wall_sec_avg": avg("elapsed_sec"),
        "wall_sec_min": min(run["elapsed_sec"] for run in runs),
        "wall_sec_max": max(run["elapsed_sec"] for run in runs),
        "user_sec_avg": avg("user_sec"),
        "sys_sec_avg": avg("sys_sec"),
        "cpu_sec_avg": avg("user_sec") + avg("sys_sec"),
        "cpu_percent_avg": avg("cpu_percent"),
        "max_rss_kb_peak": max(run["max_rss_kb"] for run in runs),
        "max_rss_kb_avg": avg("max_rss_kb"),
        "estimated_cpu_cycles_avg": avg("estimated_cpu_cycles"),
        "artifact_bytes_avg": avg("artifact_bytes"),
        "cpu_mhz": runs[0]["cpu_mhz"],
    }


def delta(baseline: dict, candidate: dict) -> dict:
    def pct(key: str) -> float:
        base = baseline[key]
        if base == 0:
            return 0.0
        return (candidate[key] - base) / base * 100.0

    return {
        "wall_sec_avg_pct": pct("wall_sec_avg"),
        "cpu_sec_avg_pct": pct("cpu_sec_avg"),
        "cpu_percent_avg_pct": pct("cpu_percent_avg"),
        "max_rss_kb_avg_pct": pct("max_rss_kb_avg"),
        "speedup_wall": baseline["wall_sec_avg"] / candidate["wall_sec_avg"]
        if candidate["wall_sec_avg"]
        else 0.0,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--fixture", required=True)
    parser.add_argument("--copies", type=int, default=4)
    parser.add_argument("--mode", default="merge", choices=["merge", "zip", "files"])
    parser.add_argument("--jobs", default="1,4")
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    binary = Path(args.binary).resolve()
    fixture = Path(args.fixture).resolve()
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    job_counts = [int(value) for value in args.jobs.split(",") if value.strip()]

    with tempfile.TemporaryDirectory(prefix="md2pdf-batch-inputs-") as work:
        inputs = prepare_sources(fixture, args.copies, Path(work))
        payloads = []
        summaries = {}
        for jobs in job_counts:
            label = f"{args.mode}-j{jobs}"
            for index in range(args.warmup):
                run_once(binary, inputs, args.mode, jobs, f"{label}-warmup", index)
            runs = [
                run_once(binary, inputs, args.mode, jobs, label, index)
                for index in range(args.runs)
            ]
            summary = summarize(runs)
            summaries[str(jobs)] = summary
            payloads.append(
                {
                    "label": label,
                    "jobs": jobs,
                    "mode": args.mode,
                    "copies": args.copies,
                    "summary": summary,
                    "runs": runs,
                }
            )

    comparison = None
    if "1" in summaries and any(key != "1" for key in summaries):
        parallel_key = next(key for key in summaries if key != "1")
        comparison = {
            "baseline_jobs": 1,
            "candidate_jobs": int(parallel_key),
            "delta": delta(summaries["1"], summaries[parallel_key]),
        }

    payload = {
        "binary": str(binary),
        "fixture": str(fixture),
        "copies": args.copies,
        "mode": args.mode,
        "nproc": os.cpu_count(),
        "comparison": comparison,
        "configs": payloads,
    }
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"comparison": comparison, "summaries": summaries}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
