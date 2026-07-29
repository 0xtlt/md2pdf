#!/usr/bin/env python3
"""Benchmark md2pdf with /usr/bin/time metrics and estimated CPU cycles."""

from __future__ import annotations

import argparse
import json
import os
import re
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path


TIME_RE = {
    "user_sec": re.compile(r"User time \(seconds\):\s*([0-9.]+)"),
    "sys_sec": re.compile(r"System time \(seconds\):\s*([0-9.]+)"),
    "elapsed_sec": re.compile(
        r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\):\s*([0-9.:]+)"
    ),
    "max_rss_kb": re.compile(r"Maximum resident set size \(kbytes\):\s*([0-9]+)"),
    "minor_faults": re.compile(r"Minor \(reclaiming a frame\) page faults:\s*([0-9]+)"),
    "major_faults": re.compile(r"Major \(requiring I/O\) page faults:\s*([0-9]+)"),
    "vol_ctx": re.compile(r"Voluntary context switches:\s*([0-9]+)"),
    "invol_ctx": re.compile(r"Involuntary context switches:\s*([0-9]+)"),
    "cpu_percent": re.compile(r"Percent of CPU this job got:\s*([0-9]+)%"),
}


def parse_elapsed(value: str) -> float:
    parts = value.split(":")
    if len(parts) == 3:
        hours, minutes, seconds = parts
        return int(hours) * 3600 + int(minutes) * 60 + float(seconds)
    if len(parts) == 2:
        minutes, seconds = parts
        return int(minutes) * 60 + float(seconds)
    return float(value)


def cpu_mhz() -> float:
    text = Path("/proc/cpuinfo").read_text(encoding="utf-8")
    matches = re.findall(r"cpu MHz\s*:\s*([0-9.]+)", text)
    if not matches:
        return 2400.0
    return statistics.mean(float(value) for value in matches)


def run_once(binary: Path, source: Path, label: str, index: int) -> dict:
    with tempfile.TemporaryDirectory(prefix="md2pdf-bench-") as tmp:
        output = Path(tmp) / f"{label}-{index}.pdf"
        cmd = [
            "/usr/bin/time",
            "-v",
            str(binary),
            str(source),
            "--quiet",
            "--output",
            str(output),
        ]
        proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
        if proc.returncode != 0:
            raise RuntimeError(
                f"md2pdf failed ({proc.returncode}):\n{proc.stdout}\n{proc.stderr}"
            )
        metrics: dict = {"ok": True, "pdf_bytes": output.stat().st_size}
        for key, pattern in TIME_RE.items():
            match = pattern.search(proc.stderr)
            if not match:
                raise RuntimeError(f"missing metric {key} in:\n{proc.stderr}")
            raw = match.group(1)
            if key == "elapsed_sec":
                metrics[key] = parse_elapsed(raw)
            elif key.endswith("_sec"):
                metrics[key] = float(raw)
            elif key == "cpu_percent":
                metrics[key] = int(raw)
            else:
                metrics[key] = int(raw)
        cpu_seconds = metrics["user_sec"] + metrics["sys_sec"]
        mhz = cpu_mhz()
        metrics["cpu_mhz"] = mhz
        # Hardware PMU cycles unavailable in this environment (perf_event EACCES).
        # Estimate retired-ish work as CPU-seconds * clock.
        metrics["estimated_cpu_cycles"] = int(cpu_seconds * mhz * 1_000_000)
        metrics["cycles_source"] = "estimated_from_cpu_time_x_mhz"
        return metrics


def summarize(runs: list[dict]) -> dict:
    def avg(key: str) -> float:
        return statistics.mean(run[key] for run in runs)

    def peak(key: str) -> float:
        return max(run[key] for run in runs)

    return {
        "runs": len(runs),
        "wall_sec_avg": avg("elapsed_sec"),
        "wall_sec_min": min(run["elapsed_sec"] for run in runs),
        "wall_sec_max": max(run["elapsed_sec"] for run in runs),
        "user_sec_avg": avg("user_sec"),
        "sys_sec_avg": avg("sys_sec"),
        "cpu_sec_avg": avg("user_sec") + avg("sys_sec"),
        "cpu_percent_avg": avg("cpu_percent"),
        "max_rss_kb_peak": peak("max_rss_kb"),
        "max_rss_kb_avg": avg("max_rss_kb"),
        "minor_faults_avg": avg("minor_faults"),
        "major_faults_avg": avg("major_faults"),
        "vol_ctx_avg": avg("vol_ctx"),
        "invol_ctx_avg": avg("invol_ctx"),
        "estimated_cpu_cycles_avg": avg("estimated_cpu_cycles"),
        "estimated_cpu_cycles_peak": peak("estimated_cpu_cycles"),
        "pdf_bytes_avg": avg("pdf_bytes"),
        "cpu_mhz": runs[0]["cpu_mhz"],
        "cycles_source": runs[0]["cycles_source"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    binary = Path(args.binary).resolve()
    source = Path(args.source).resolve()
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)

    for index in range(args.warmup):
        run_once(binary, source, f"{args.label}-warmup", index)

    runs = [run_once(binary, source, args.label, index) for index in range(args.runs)]
    payload = {
        "label": args.label,
        "binary": str(binary),
        "source": str(source),
        "nproc": os.cpu_count(),
        "summary": summarize(runs),
        "runs": runs,
    }
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(payload["summary"], indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
