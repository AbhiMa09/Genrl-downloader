#!/usr/bin/env python3
import os
import sys
import time
import subprocess
import json
from PIL import Image, ImageChops, ImageDraw

TARGETS = [
    {
        "id": "genrl_downloader_dashboard",
        "folder": "assets/genrl_downloader_dashboard",
        "args": ["--page", "0"],
        "width": 1593,
        "height": 1600,
    },
    {
        "id": "genrl_downloader_inbox",
        "folder": "assets/genrl_downloader_inbox",
        "args": ["--page", "1"],
        "width": 893,
        "height": 1600,
    },
    {
        "id": "genrl_downloader_queue",
        "folder": "assets/genrl_downloader_queue",
        "args": ["--page", "2"],
        "width": 1600,
        "height": 1432,
    },
    {
        "id": "genrl_downloader_library",
        "folder": "assets/genrl_downloader_library",
        "args": ["--page", "3"],
        "width": 1600,
        "height": 1578,
    },
    {
        "id": "genrl_downloader_search",
        "folder": "assets/genrl_downloader_search",
        "args": ["--page", "4"],
        "width": 1600,
        "height": 1352,
    },
    {
        "id": "genrl_downloader_activity",
        "folder": "assets/genrl_downloader_activity",
        "args": ["--page", "5"],
        "width": 1454,
        "height": 1600,
    },
    {
        "id": "genrl_downloader_rules",
        "folder": "assets/genrl_downloader_rules",
        "args": ["--page", "6"],
        "width": 1600,
        "height": 1536,
    },
    {
        "id": "genrl_downloader_settings",
        "folder": "assets/genrl_downloader_settings",
        "args": ["--page", "7"],
        "width": 1214,
        "height": 1600,
    },
    {
        "id": "genrl_downloader_show_detail",
        "folder": "assets/genrl_downloader_show_detail",
        "args": ["--page", "3", "--show", "10029"],
        "width": 1600,
        "height": 1506,
    },
    {
        "id": "genrl_downloader_setup_wizard",
        "folder": "assets/genrl_downloader_setup_wizard",
        "args": ["--wizard"],
        "width": 1600,
        "height": 1280,
    },
    {
        "id": "genrl_downloader_loading_ingestion_state",
        "folder": "assets/genrl_downloader_loading_ingestion_state",
        "args": ["--ingestion"],
        "width": 1600,
        "height": 1554,
    },
    {
        "id": "genrl_downloader_tray_menu_notifications",
        "folder": "assets/genrl_downloader_tray_menu_notifications",
        "args": ["--tray"],
        "width": 1280,
        "height": 898,
    },
    {
        "id": "genrl_downloader_app_logo",
        "folder": "assets/genrl_downloader_app_logo",
        "args": ["--logo"],
        "width": 512,
        "height": 520,
    },
]

BIN_PATH = "app/build/linux/x64/debug/bundle/genrl_downloader"

def capture_target(target):
    folder = target["folder"]
    out_path = os.path.join(folder, "app_screen.png")
    proto_path = os.path.join(folder, "screen.png")
    diff_path = os.path.join(folder, "diff.png")

    w = target["width"]
    h = target["height"]

    cmd = [
        BIN_PATH,
        "--width", str(w),
        "--height", str(h),
        "--no-decoration",
    ] + target["args"]

    env = os.environ.copy()
    env["DISPLAY"] = ":0"
    env["WAYLAND_DISPLAY"] = "wayland-0"

    print(f"Launching {target['id']} at {w}x{h}...")
    proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(2.5)

    if os.path.exists(out_path):
        os.remove(out_path)

    spec_res = subprocess.run(
        ["spectacle", "-b", "-n", "-a", "-e", "-S", "-o", out_path],
        env=env,
        capture_output=True,
        text=True
    )

    proc.terminate()
    try:
        proc.wait(timeout=2)
    except Exception:
        proc.kill()

    if not os.path.exists(out_path):
        print(f"ERROR: Failed to capture {out_path}: {spec_res.stderr}")
        return None

    # Load captured image and prototype
    app_img = Image.open(out_path).convert("RGB")
    proto_img = Image.open(proto_path).convert("RGB")

    print(f"Captured: {app_img.size}, Target: {proto_img.size}")

    # Ensure sizes match by padding/cropping if needed
    target_w, target_h = proto_img.size
    if app_img.size != proto_img.size:
        print(f"Resizing captured image from {app_img.size} to {proto_img.size}...")
        canvas = Image.new("RGB", (target_w, target_h), (16, 20, 26)) # surface bg
        canvas.paste(app_img, (0, 0))
        app_img = canvas
        app_img.save(out_path)

    # Compute pixel diff
    diff_img = ImageChops.difference(proto_img, app_img)
    diff_pixels = diff_img.load()
    proto_pixels = proto_img.load()
    app_pixels = app_img.load()

    total_pixels = target_w * target_h
    mismatched_pixels = 0
    severe_mismatches = 0

    # Region breakdown:
    # 1. Sidebar: x in [0, 220]
    # 2. Header: y in [0, 70]
    # 3. Main Content: remaining area
    regions = {
        "sidebar": {"total": 0, "mismatched": 0},
        "header": {"total": 0, "mismatched": 0},
        "content": {"total": 0, "mismatched": 0},
    }

    diff_visual = Image.new("RGBA", (target_w, target_h), (0, 0, 0, 0))
    diff_visual_pixels = diff_visual.load()

    for y in range(target_h):
        for x in range(target_w):
            r_diff, g_diff, b_diff = diff_pixels[x, y]
            delta = (r_diff + g_diff + b_diff) / 3.0

            region_key = "content"
            if x <= 220:
                region_key = "sidebar"
            elif y <= 70:
                region_key = "header"

            regions[region_key]["total"] += 1

            if delta > 15: # threshold for perceptual difference
                mismatched_pixels += 1
                regions[region_key]["mismatched"] += 1
                # Mark diff in visual
                if delta > 60:
                    severe_mismatches += 1
                    diff_visual_pixels[x, y] = (255, 0, 0, 200) # bright red
                else:
                    diff_visual_pixels[x, y] = (255, 165, 0, 150) # orange

    diff_visual.save(diff_path)

    mismatch_pct = (mismatched_pixels / total_pixels) * 100.0
    match_pct = 100.0 - mismatch_pct

    region_stats = {}
    for r_k, r_v in regions.items():
        if r_v["total"] > 0:
            region_stats[r_k] = {
                "mismatch_pct": round((r_v["mismatched"] / r_v["total"]) * 100.0, 2),
                "mismatched_count": r_v["mismatched"],
                "total_count": r_v["total"],
            }

    result = {
        "id": target["id"],
        "folder": folder,
        "width": target_w,
        "height": target_h,
        "total_pixels": total_pixels,
        "mismatched_pixels": mismatched_pixels,
        "match_percentage": round(match_pct, 2),
        "mismatch_percentage": round(mismatch_pct, 2),
        "severe_mismatches": severe_mismatches,
        "regions": region_stats,
    }

    print(f"Result for {target['id']}: {result['match_percentage']}% match ({result['mismatch_percentage']}% diff)")
    return result

def main():
    results = []
    for t in TARGETS:
        res = capture_target(t)
        if res:
            results.append(res)
        time.sleep(0.5)

    with open("assets/audit_report.json", "w") as f:
        json.dump(results, f, indent=2)

    print("\n=== SUMMARY REPORT ===")
    for r in results:
        print(f"{r['id']:<45} | Size: {r['width']}x{r['height']:<6} | Match: {r['match_percentage']}% | Diff: {r['mismatch_percentage']}%")

if __name__ == "__main__":
    main()
