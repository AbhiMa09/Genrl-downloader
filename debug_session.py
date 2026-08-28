#!/usr/bin/env python
"""Debug script to test Zen Browser session reading"""
import os
import json
import lz4.block
from pathlib import Path

def find_zen_profile():
    """Find Zen Browser profile directory"""
    possible_paths = [
        Path(os.environ.get('APPDATA', '')) / 'Zen' / 'Profiles',
        Path(os.environ.get('LOCALAPPDATA', '')) / 'Zen' / 'Profiles',
        Path.home() / 'AppData' / 'Roaming' / 'Zen' / 'Profiles',
        Path.home() / 'AppData' / 'Local' / 'Zen' / 'Profiles',
    ]
    
    for base_path in possible_paths:
        if base_path.exists():
            print(f"Found base path: {base_path}")
            for profile_dir in base_path.iterdir():
                if profile_dir.is_dir():
                    session_dir = profile_dir / 'sessionstore-backups'
                    if session_dir.exists():
                        print(f"Found profile: {profile_dir}")
                        print(f"Session dir: {session_dir}")
                        for f in session_dir.iterdir():
                            print(f"  Session file: {f.name} ({f.stat().st_size} bytes)")
                        return profile_dir
    return None

def parse_session_file(session_file: Path):
    """Parse Firefox/Zen session file (JSON or JSONLZ4)"""
    tabs_data = []
    
    if session_file.suffix == '.jsonlz4':
        try:
            with open(session_file, 'rb') as f:
                f.read(8)  # Skip header
                compressed = f.read()
                decompressed = lz4.block.decompress(compressed)
                data = json.loads(decompressed)
        except Exception as e:
            print(f"Failed to decompress {session_file}: {e}")
            return tabs_data
    else:
        try:
            with open(session_file, 'r', encoding='utf-8') as f:
                data = json.load(f)
        except Exception as e:
            print(f"Failed to parse {session_file}: {e}")
            return tabs_data
    
    if 'windows' in data:
        for window in data['windows']:
            if 'tabs' in window:
                for tab in window['tabs']:
                    if 'entries' in tab and tab['entries']:
                        current_entry = tab['entries'][-1]
                        url = current_entry.get('url', '')
                        title = current_entry.get('title', '')
                        if url and title:
                            tabs_data.append({'url': url, 'title': title})
    
    return tabs_data

if __name__ == "__main__":
    profile = find_zen_profile()
    if profile:
        session_dir = profile / 'sessionstore-backups'
        session_files = [
            session_dir / 'recovery.jsonlz4',
            session_dir / 'recovery.js',
            session_dir / 'previous.jsonlz4',
            session_dir / 'previous.js',
        ]
        
        for session_file in session_files:
            if session_file.exists():
                print(f"\n=== Reading {session_file.name} ===")
                tabs = parse_session_file(session_file)
                print(f"Found {len(tabs)} tabs:")
                for i, tab in enumerate(tabs[:10]):
                    print(f"  {i+1}. {tab['title'][:80]}")
                    print(f"     {tab['url'][:100]}")
                if len(tabs) > 10:
                    print(f"  ... and {len(tabs) - 10} more")
    else:
        print("Zen Browser profile not found!")
        print("Check if Zen Browser is installed and has been run at least once.")