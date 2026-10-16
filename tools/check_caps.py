#!/usr/bin/env python3
"""
CI / Lint check: ensures every source file in app/lib and core/ respects the hard cap of 500 lines.
Soft cap is 300 lines. Tests and generated files are excepted.
"""
import os
import sys

HARD_CAP = 500
SOFT_CAP = 300

def count_lines(filepath):
    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        return sum(1 for _ in f)

def check_caps(root_dir='.'):
    violations = []
    warnings = []

    # Check app/lib
    app_lib = os.path.join(root_dir, 'app', 'lib')
    if os.path.exists(app_lib):
        for root, _, files in os.walk(app_lib):
            for file in files:
                if file.endswith('.dart'):
                    path = os.path.join(root, file)
                    lines = count_lines(path)
                    rel = os.path.relpath(path, root_dir)
                    if lines > HARD_CAP:
                        violations.append((lines, rel))
                    elif lines > SOFT_CAP:
                        warnings.append((lines, rel))

    # Check core (crates only, excluding tests, target, benches)
    core_dir = os.path.join(root_dir, 'core')
    if os.path.exists(core_dir):
        for root, dirs, files in os.walk(core_dir):
            parts = root.split(os.sep)
            if 'target' in parts or 'tests' in parts or 'benches' in parts:
                continue
            for file in files:
                if file.endswith('.rs'):
                    if file == 'tests.rs' or file.endswith('_test.rs'):
                        continue
                    path = os.path.join(root, file)
                    lines = count_lines(path)
                    rel = os.path.relpath(path, root_dir)
                    if lines > HARD_CAP:
                        violations.append((lines, rel))
                    elif lines > SOFT_CAP:
                        warnings.append((lines, rel))

    return violations, warnings

if __name__ == '__main__':
    root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
    violations, warnings = check_caps(root)

    if warnings:
        print(f"[INFO] {len(warnings)} files exceed soft cap ({SOFT_CAP} lines):")
        for lines, path in sorted(warnings, reverse=True)[:10]:
            print(f"  {lines:4d} lines: {path}")

    if violations:
        print(f"\n[ERROR] {len(violations)} files EXCEED HARD CAP ({HARD_CAP} lines):", file=sys.stderr)
        for lines, path in sorted(violations, reverse=True):
            print(f"  {lines:4d} lines: {path}", file=sys.stderr)
        sys.exit(1)
    else:
        print(f"\n[OK] All source files are within hard cap of {HARD_CAP} lines!")
        sys.exit(0)
