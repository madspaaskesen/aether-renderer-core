#!/usr/bin/env python3
import os
import shutil
import subprocess
import time
from pathlib import Path

FRAME_SRC = Path('tests/testdata/frame_0016.png')
FRAME_COUNTS = [1, 10, 100, 1000, 10000]

if not FRAME_SRC.exists():
    raise FileNotFoundError(f"Source frame not found: {FRAME_SRC}")

results = []

for count in FRAME_COUNTS:
    frame_dir = Path(f'frames_{count}')
    output_file = Path(f'output_{count}.webm')

    if frame_dir.exists():
        shutil.rmtree(frame_dir)
    frame_dir.mkdir(parents=True)

    for i in range(count):
        target = frame_dir / f"frame_{i:04d}.png"
        shutil.copy2(FRAME_SRC, target)

    cmd = ['aether-renderer-core', '--input', str(frame_dir), '--output', str(output_file)]

    start = time.time()
    subprocess.run(cmd, check=True)
    duration = time.time() - start

    size_mb = output_file.stat().st_size / (1024 * 1024)
    avg = duration / count
    print(f"{count} | {duration:.2f} | {size_mb:.2f} | {avg:.4f}")
    results.append((count, duration, size_mb, avg))

print("\nBenchmark Results:")
print("Frames | Total Time | Size (MB) | Avg Time/Frame")
for r in results:
    print(f"{r[0]} | {r[1]:.2f} | {r[2]:.2f} | {r[3]:.4f}")
