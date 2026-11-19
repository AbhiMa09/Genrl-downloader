#!/usr/bin/env python3
"""
Universal Auto-Download Engine for Genrl Downloader.
Enforces:
  Priority 1: Batch / Complete Season Packs (fits in free disk space).
  Priority 2: Single Episodes fallback.
  Quality: 1080p preferred, falling back to 720p / x265 HEVC.
  Auto-Accept: Title match with >= 4 seeders without user prompts.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET

DEFAULT_QBIT_URL = "http://127.0.0.1:8080"
PRIMARY_DL_DIR = "/mnt/hhd/GenrlTest/Downloads"
SECONDARY_DL_DIR = "/mnt/new-volume/Downloads"
LOG_FILE = os.path.expanduser("~/.local/share/genrl-downloader/autodownload.log")


def log(msg: str):
    timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
    line = f"[{timestamp}] {msg}"
    print(line, flush=True)
    try:
        os.makedirs(os.path.dirname(LOG_FILE), exist_ok=True)
        with open(LOG_FILE, "a") as f:
            f.write(line + "\n")
    except Exception:
        pass


def get_free_space_gb(path: str) -> float:
    try:
        stat = shutil.disk_usage(path)
        return stat.free / (1024 ** 3)
    except Exception:
        return 0.0


def choose_download_dir(needed_gb: float) -> str:
    primary_free = get_free_space_gb(PRIMARY_DL_DIR)
    if primary_free >= needed_gb + 15.0:  # 15GB safety floor
        return PRIMARY_DL_DIR
    sec_free = get_free_space_gb(SECONDARY_DL_DIR)
    if sec_free >= needed_gb + 15.0:
        os.makedirs(SECONDARY_DL_DIR, exist_ok=True)
        log(f"Primary drive low ({primary_free:.1f}GB). Using secondary drive: {SECONDARY_DL_DIR} ({sec_free:.1f}GB free)")
        return SECONDARY_DL_DIR
    return PRIMARY_DL_DIR


def search_nyaa(query: str, min_seeders: int = 4) -> list:
    url = f"https://nyaa.si/?page=rss&q={urllib.parse.quote(query)}&c=1_2&f=0"
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    try:
        with urllib.request.urlopen(req, timeout=12) as resp:
            tree = ET.fromstring(resp.read())
            results = []
            for item in tree.findall(".//item"):
                title = item.find("title").text if item.find("title") is not None else ""
                link = item.find("link").text if item.find("link") is not None else ""
                seeders_elem = item.find("{https://nyaa.si/xmlns/nyaa}seeders")
                size_elem = item.find("{https://nyaa.si/xmlns/nyaa}size")
                seeders = int(seeders_elem.text) if seeders_elem is not None and seeders_elem.text.isdigit() else 0
                size_str = size_elem.text if size_elem is not None else "0 MiB"
                
                # parse size in GB
                size_gb = 0.0
                parts = size_str.split()
                if len(parts) == 2:
                    val = float(parts[0])
                    unit = parts[1].upper()
                    if "GIB" in unit or "GB" in unit:
                        size_gb = val
                    elif "MIB" in unit or "MB" in unit:
                        size_gb = val / 1024.0
                    elif "TIB" in unit or "TB" in unit:
                        size_gb = val * 1024.0

                if seeders >= min_seeders:
                    results.append({
                        "title": title,
                        "link": link,
                        "seeders": seeders,
                        "size_gb": size_gb,
                        "size_str": size_str,
                    })
            return results
    except Exception as e:
        log(f"Nyaa search error for '{query}': {e}")
        return []


def get_active_qbit_torrents() -> list:
    try:
        req = urllib.request.Request(f"{DEFAULT_QBIT_URL}/api/v2/torrents/info?category=genrl")
        with urllib.request.urlopen(req, timeout=5) as resp:
            return json.loads(resp.read())
    except Exception as e:
        log(f"qBit check error: {e}")
        return []


def grab_torrent(link: str, save_path: str = None) -> bool:
    cmd = ["genrl", "grab", "--no-watch", link]
    if save_path:
        cmd.extend(["--to", save_path])
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode == 0:
        log(f"Successfully grabbed: {link} into {save_path or 'default'}")
        return True
    log(f"Failed to grab: {link}. Output: {res.stderr.strip() or res.stdout.strip()}")
    return False


def is_already_grabbed(title_substr: str, active_torrents: list) -> bool:
    title_norm = title_substr.lower().replace(" ", "").replace("-", "")
    for t in active_torrents:
        t_name = t.get("name", "").lower().replace(" ", "").replace("-", "")
        if title_norm in t_name or t_name in title_norm:
            return True
    return False


def process_show_batch_first(show_name: str, alt_names: list = None, max_episodes: int = 15):
    """
    Priority 1: Batch search
    Priority 2: Single episodes fallback
    """
    log(f"=== Processing Show: {show_name} ===")
    active = get_active_qbit_torrents()
    names = [show_name] + (alt_names or [])
    
    # 1. PRIORITY 1: BATCH SEARCH
    log(f"Checking Priority 1 (Batches) for {show_name}...")
    batch_queries = [
        f"{show_name} Batch",
        f"{show_name} Complete",
        f"{show_name} Archive",
    ]
    for alt in (alt_names or []):
        batch_queries.append(f"{alt} Batch")
        batch_queries.append(f"{alt} Complete")

    found_batch = False
    for bq in batch_queries:
        candidates = search_nyaa(bq, min_seeders=4)
        if not candidates:
            continue
        
        # Sort by quality: 1080p > 720p > x265
        def score_batch(c):
            t = c["title"].lower()
            sc = 0
            if "1080p" in t: sc += 30
            elif "720p" in t: sc += 20
            if "hevc" in t or "x265" in t: sc += 15
            sc += min(c["seeders"], 50)
            return sc
        
        candidates.sort(key=score_batch, reverse=True)
        for cand in candidates:
            if is_already_grabbed(cand["title"], active):
                log(f"Batch already queued or downloaded: {cand['title']}")
                found_batch = True
                break
            
            # Check disk space
            dest = choose_download_dir(cand["size_gb"])
            free_gb = get_free_space_gb(dest)
            if cand["size_gb"] > (free_gb - 10.0):
                log(f"Batch {cand['title']} ({cand['size_gb']:.1f}GB) exceeds free space ({free_gb:.1f}GB). Skipping.")
                continue

            log(f"Selected Batch: {cand['title']} ({cand['size_gb']:.1f}GB, {cand['seeders']} seeds)")
            if grab_torrent(cand["link"], dest):
                found_batch = True
                break
        if found_batch:
            break

    if found_batch:
        log(f"Priority 1 Batch acquired for {show_name}!")
        return

    # 2. PRIORITY 2: SINGLE EPISODES FALLBACK
    log(f"No suitable batch found for {show_name}. Falling back to Priority 2 (Single Episodes)...")
    grabbed_count = 0
    # Search single episodes up to max_episodes
    for ep in range(1, max_episodes + 1):
        ep_query = f"{show_name} {ep}"
        candidates = search_nyaa(ep_query, min_seeders=4)
        if not candidates:
            # Try leading zero format
            candidates = search_nyaa(f"{show_name} {ep:02d}", min_seeders=4)
        if not candidates:
            continue
        
        # Pick best: 1080p preferred, then 720p/x265
        def score_ep(c):
            t = c["title"].lower()
            sc = 0
            if "1080p" in t: sc += 40
            elif "720p" in t: sc += 25
            if "hevc" in t or "x265" in t: sc += 15
            sc += min(c["seeders"], 30)
            return sc
        
        candidates.sort(key=score_ep, reverse=True)
        for cand in candidates:
            if is_already_grabbed(cand["title"], active):
                break
            dest = choose_download_dir(cand["size_gb"])
            log(f"Grabbing Episode {ep}: {cand['title']} ({cand['seeders']} seeds)")
            if grab_torrent(cand["link"], dest):
                grabbed_count += 1
                active = get_active_qbit_torrents()
                time.sleep(2)  # courteous throttle
                break

    log(f"Priority 2 finished for {show_name}: Grabbed {grabbed_count} episodes.")


def monitor_downloads_loop(poll_interval: int = 30):
    log("Starting autonomous download monitor...")
    while True:
        try:
            active = get_active_qbit_torrents()
            downloading = [t for t in active if t.get("state") in ("downloading", "stalledDL", "metaDL")]
            completed = [t for t in active if t.get("progress", 0) >= 1.0]
            
            log(f"Monitor Status: {len(downloading)} downloading, {len(completed)} completed.")
            for t in downloading:
                prog = t.get("progress", 0) * 100
                speed = t.get("dlspeed", 0) / (1024 * 1024)
                log(f"  -> {t.get('name')}: {prog:.1f}% @ {speed:.2f} MB/s")
            
            # If all downloads are complete or idle, sleep longer
            time.sleep(poll_interval)
        except KeyboardInterrupt:
            log("Monitor stopped by user.")
            break
        except Exception as e:
            log(f"Monitor error: {e}")
            time.sleep(poll_interval)


def main():
    parser = argparse.ArgumentParser(description="Universal Batch-First Auto-Downloader")
    parser.add_argument("--show", type=str, help="Target show title to download")
    parser.add_argument("--alt", type=str, nargs="*", help="Alternative names or synonyms")
    parser.add_argument("--max-episodes", type=int, default=15, help="Max single episodes to check if no batch")
    parser.add_argument("--monitor", action="store_true", help="Run background monitor loop after queuing")
    parser.add_argument("--poll-interval", type=int, default=30, help="Seconds between progress polls")
    args = parser.parse_args()

    if args.show:
        process_show_batch_first(args.show, args.alt, args.max_episodes)
    
    if args.monitor:
        monitor_downloads_loop(args.poll_interval)


if __name__ == "__main__":
    main()
