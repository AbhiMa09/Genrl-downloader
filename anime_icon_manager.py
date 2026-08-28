import os
import sys
import requests
import json
import shutil
from pathlib import Path
from PIL import Image
import io
import re

ANILIST_URL = "https://graphql.anilist.co"
ANIME_FOLDERS_ROOT = Path(r"D:\\")
# ICONS_CACHE_DIR is no longer needed as we'll save icons directly in anime folders

SEARCH_QUERY = """
query ($search: String) {
  Media(search: $search, type: ANIME) {
    id
    title {
      romaji
      english
      native
    }
    coverImage {
      large
      medium
    }
    description
    episodes
    status
    genres
    startDate {
      year
    }
  }
}
"""

def sanitize_filename(name):
    return re.sub(r'[<>:"/\\|?*]', '_', name)

def search_anime(folder_name):
    clean_name = re.sub(r'[\[\(].*?[\]\)]', '', folder_name).strip()
    clean_name = re.sub(r'\b(1080p|720p|480p|HEVC|x265|x264|BD|WEB|DUAL|AUDIO|SUB)\b', '', clean_name, flags=re.IGNORECASE).strip()
    
    variables = {"search": clean_name}
    response = requests.post(ANILIST_URL, json={"query": SEARCH_QUERY, "variables": variables})
    data = response.json()
    
    if "data" in data and data["data"] and data["data"]["Media"]:
        return data["data"]["Media"]
    return None

def download_image(url, save_path):
    try:
        response = requests.get(url, timeout=10)
        response.raise_for_status()
        with open(save_path, 'wb') as f:
            f.write(response.content)
        return True
    except Exception as e:
        print(f"Failed to download {url}: {e}")
        return False

def convert_to_ico(png_path, ico_path, sizes=[(512, 512), (256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)]):
    try:
        img = Image.open(png_path)
        # Ensure image is in RGBA mode for best quality
        if img.mode != 'RGBA':
            img = img.convert('RGBA')
        img.save(ico_path, format='ICO', sizes=sizes)
        return True
    except Exception as e:
        print(f"Failed to convert to ICO: {e}")
        return False

def set_folder_icon(folder_path, icon_path):
    desktop_ini = folder_path / "desktop.ini"
    icon_relative = os.path.relpath(icon_path, folder_path)
    
    content = f"""[.ShellClassInfo]
IconResource={icon_relative},0
[ViewState]
Mode=
Vid=
FolderType=Generic
"""
    try:
        with open(desktop_ini, 'w', encoding='utf-8') as f:
            f.write(content)
        os.system(f'attrib +h +s "{desktop_ini}"')
        os.system(f'attrib +s "{folder_path}"')
        return True
    except Exception as e:
        print(f"Failed to set folder icon: {e}")
        return False

def save_synopsis(folder_path, anime_data):
    synopsis_path = folder_path / "synopsis.txt"
    description = anime_data.get("description", "").replace("<br>", "\n").replace("<i>", "").replace("</i>", "")
    description = re.sub(r'<[^>]+>', '', description)
    
    content = f"""Title: {anime_data['title'].get('romaji', 'N/A')}
English: {anime_data['title'].get('english', 'N/A')}
Native: {anime_data['title'].get('native', 'N/A')}
Year: {anime_data.get('startDate', {}).get('year', 'N/A')}
Episodes: {anime_data.get('episodes', 'N/A')}
Status: {anime_data.get('status', 'N/A')}
Genres: {', '.join(anime_data.get('genres', []))}

Synopsis:
{description.strip()}
"""
    try:
        with open(synopsis_path, 'w', encoding='utf-8') as f:
            f.write(content)
        return True
    except Exception as e:
        print(f"Failed to save synopsis: {e}")
        return False

def process_anime_folders():
    if not ANIME_FOLDERS_ROOT.exists():
        print(f"Anime root folder not found: {ANIME_FOLDERS_ROOT}")
        return
    
    folders = [f for f in ANIME_FOLDERS_ROOT.iterdir() if f.is_dir() and not f.name.startswith('.')]
    
    if not folders:
        print("No anime folders found on D:\\")
        return
    
    print(f"Found {len(folders)} folders to process...")
    
    for folder in folders:
        print(f"\nProcessing: {folder.name}")
        
        anime = search_anime(folder.name)
        if not anime:
            print(f"  No match found on AniList")
            continue
        
        print(f"  Found: {anime['title']['romaji']}")
        
        cover_url = anime['coverImage']['large']
        safe_name = sanitize_filename(anime['title']['romaji'])

        # Save icon directly in the anime folder for better organization
        png_path = folder / f"{safe_name}.png"
        ico_path = folder / f"{safe_name}.ico"

        if not png_path.exists():
            print(f"  Downloading cover image...")
            if not download_image(cover_url, png_path):
                continue

        if not ico_path.exists():
            print(f"  Converting to ICO with enhanced quality...")
            # Increased sizes for better quality icons, including larger sizes
            if not convert_to_ico(png_path, ico_path, sizes=[(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)]):
                continue

        print(f"  Setting folder icon...")
        if not set_folder_icon(folder, ico_path):
            continue
        
        print(f"  Saving synopsis...")
        save_synopsis(folder, anime)
        
        print(f"  Done!")

if __name__ == "__main__":
    print("Anime Folder Icon & Synopsis Manager")
    print("=" * 40)
    process_anime_folders()
    print("\nComplete! Restart Explorer or press F5 to see icon changes.")