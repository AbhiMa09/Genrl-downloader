import os
import sys
import requests
import json
import shutil
from pathlib import Path
from PIL import Image
import io
import re
import urllib.parse

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

def search_folder_icon(anime_name):
    """Search for 'anime name folder icon' and return best image URL"""
    try:
        query = f"{anime_name} folder icon"
        # Use DuckDuckGo HTML (no API key needed)
        url = f"https://html.duckduckgo.com/html/?q={urllib.parse.quote(query)}"
        headers = {'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'}
        resp = requests.get(url, headers=headers, timeout=10)
        
        # Parse HTML for image URLs
        img_urls = re.findall(r'class="result__snippet".*?src="([^"]+)"', resp.text, re.DOTALL)
        img_urls += re.findall(r'<img[^>]+src="([^"]+\.(?:jpg|jpeg|png|webp))"', resp.text)
        
        # Filter for likely folder icons
        for url in img_urls:
            if any(kw in url.lower() for kw in ['folder', 'icon', 'cover', 'poster', 'art']):
                if url.startswith('http'):
                    return url
        
        # Fallback: return first valid image
        for url in img_urls:
            if url.startswith('http') and any(ext in url for ext in ['.jpg', '.jpeg', '.png', '.webp']):
                return url
    except Exception as e:
        print(f"  Folder icon search failed: {e}")
    return None

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
    
    # Skip system/hidden folders
    SYSTEM_FOLDERS = {
        '$RECYCLE.BIN', 'System Volume Information', 'Program Files', 
        'Program Files (x86)', 'Windows', 'PerfLogs', 'Recovery',
        'SteamLibrary', 'icons', '$RECYCLE.BIN'
    }
    
    folders = [f for f in ANIME_FOLDERS_ROOT.iterdir() 
               if f.is_dir() and not f.name.startswith('.') and f.name not in SYSTEM_FOLDERS]
    
    if not folders:
        print("No anime folders found on D:\\")
        return
    
    print(f"Found {len(folders)} folders to process...")
    
    for folder in folders:
        print(f"\nProcessing: {folder.name}")
        
        # Try folder icon search first
        clean_name = re.sub(r'[\[\(].*?[\]\)]', '', folder.name).strip()
        clean_name = re.sub(r'\b(1080p|720p|480p|HEVC|x265|x264|BD|WEB|DUAL|AUDIO|SUB)\b', '', clean_name, flags=re.IGNORECASE).strip()
        
        if not clean_name or len(clean_name) < 2:
            print(f"  Skipping: invalid name")
            continue
        
        print(f"  Searching for folder icon: {clean_name} folder icon...")
        icon_url = search_folder_icon(clean_name)
        
        anime = search_anime(folder.name)
        if anime:
            print(f"  Found on AniList: {anime['title']['romaji']}")
            safe_name = sanitize_filename(anime['title']['romaji'])
            anilist_cover = anime['coverImage']['large']
        else:
            print(f"  No AniList match, using folder name")
            safe_name = sanitize_filename(clean_name)
            anilist_cover = None
        
        # Use folder icon search result if found, else AniList cover
        final_url = icon_url or anilist_cover
        if not final_url:
            print(f"  No icon found")
            continue
        
        print(f"  Using: {final_url[:80]}...")
        
        # Save icon directly in the anime folder for better organization
        png_path = folder / f"{safe_name}.png"
        ico_path = folder / f"{safe_name}.ico"

        if not png_path.exists():
            print(f"  Downloading image...")
            if not download_image(final_url, png_path):
                continue

        if not ico_path.exists():
            print(f"  Converting to ICO with enhanced quality...")
            if not convert_to_ico(png_path, ico_path, sizes=[(512, 512), (256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)]):
                continue

        print(f"  Setting folder icon...")
        try:
            if not set_folder_icon(folder, ico_path):
                continue
        except PermissionError:
            print(f"  Permission denied - run as Administrator")
            continue
        except Exception as e:
            print(f"  Error setting icon: {e}")
            continue
        
        if anime:
            print(f"  Saving synopsis...")
            save_synopsis(folder, anime)
        
        print(f"  Done!")

if __name__ == "__main__":
    print("Anime Folder Icon & Synopsis Manager")
    print("=" * 40)
    process_anime_folders()
    print("\nComplete! Restart Explorer or press F5 to see icon changes.")