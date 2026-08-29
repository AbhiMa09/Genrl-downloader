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
import time
from typing import Optional

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
    synonyms
    coverImage {
      large
      medium
    }
    description
    episodes
    status
    genres
    studios {
      nodes {
        name
      }
    }
    startDate {
      year
      month
      day
    }
    endDate {
      year
      month
      day
    }
    averageScore
    popularity
    source
    format
    season
    seasonYear
  }
}
"""

DEVIANTART_API_URL = "https://www.deviantart.com/api/v1/oauth2/browse/popular"
DEVIANTART_SEARCH_URL = "https://www.deviantart.com/api/v1/oauth2/browse/search"

# Try to import deviantart_auth
try:
    from deviantart_auth import get_access_token
    DEVIANTART_AVAILABLE = True
except ImportError:
    DEVIANTART_AVAILABLE = False
    print("deviantart_auth module not found. DeviantArt search will be skipped.")

def sanitize_filename(name):
    return re.sub(r'[<>:"/\\|?*]', '_', name)


def search_deviantart_folder_icon(anime_name: str) -> Optional[str]:
    """
    Search DeviantArt for 'anime name folder icon' and return best image URL.
    Returns None if DeviantArt auth fails or no suitable results.
    """
    if not DEVIANTART_AVAILABLE:
        return None
    
    try:
        token = get_access_token()
    except Exception as e:
        print(f"  DeviantArt auth failed: {e}")
        return None
    
    # Try search with "folder icon" first
    queries = [
        f"{anime_name} folder icon",
        f"{anime_name} anime folder",
        f"{anime_name} folder",
    ]
    
    for query in queries:
        try:
            print(f"  Searching DeviantArt: {query}")
            params = {
                'q': query,
                'access_token': token,
                'limit': 24,
                'mature_content': 'false',
            }
            
            resp = requests.get(DEVIANTART_SEARCH_URL, params=params, timeout=15)
            
            if resp.status_code == 429:
                print(f"  DeviantArt rate limit hit (429). Backing off...")
                time.sleep(2)
                continue
            
            resp.raise_for_status()
            data = resp.json()
            
            results = data.get('results', [])
            if not results:
                continue
            
            # Filter and rank results
            candidates = []
            for item in results:
                # Check if it's an image
                if item.get('content', {}).get('type') != 'image':
                    continue
                
                content = item.get('content', {})
                width = content.get('width', 0)
                height = content.get('height', 0)
                
                # Skip non-square-ish images (banners, wallpapers)
                if width > 0 and height > 0:
                    ratio = width / height
                    if ratio < 0.7 or ratio > 1.3:
                        continue
                
                # Prefer higher resolution
                resolution_score = min(width * height, 4000000) / 40000  # Normalize
                
                # Bonus for folder/icon related tags
                tags = [t.get('label', '').lower() for t in item.get('tags', [])]
                tag_bonus = 0
                if any(kw in ' '.join(tags) for kw in ['folder', 'icon', 'cover', 'poster', 'keyart', 'key art', 'anime']):
                    tag_bonus = 10
                
                # Bonus for favorites/views
                stats = item.get('stats', {})
                fav_bonus = min(stats.get('favourites', 0) / 100, 20)
                view_bonus = min(stats.get('comments', 0) / 50, 10)
                
                total_score = resolution_score + tag_bonus + fav_bonus + view_bonus
                
                src = content.get('src')
                if src:
                    candidates.append((total_score, src, item.get('title', '')))
            
            if candidates:
                candidates.sort(key=lambda x: x[0], reverse=True)
                best = candidates[0]
                print(f"  Found DeviantArt result: {best[2]} (score: {best[0]:.1f})")
                return best[1]
                
        except requests.HTTPError as e:
            if e.response is not None and e.response.status_code == 429:
                print(f"  DeviantArt rate limit hit. Backing off...")
                time.sleep(3)
                continue
            print(f"  DeviantArt search error: {e}")
        except Exception as e:
            print(f"  DeviantArt search error: {e}")
    
    return None
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
    
    # Build content dynamically, omitting empty/null fields
    lines = []
    
    title = anime_data.get('title', {})
    if title.get('romaji'):
        lines.append(f"Title (Romaji): {title['romaji']}")
    if title.get('english'):
        lines.append(f"Title (English): {title['english']}")
    if title.get('native'):
        lines.append(f"Title (Native): {title['native']}")
    if anime_data.get('synonyms'):
        lines.append(f"Synonyms: {', '.join(anime_data['synonyms'])}")
    
    # Format dates
    start = anime_data.get('startDate', {})
    if start.get('year'):
        date_parts = [str(start['year'])]
        if start.get('month'):
            date_parts.append(f"{start['month']:02d}")
        if start.get('day'):
            date_parts.append(f"{start['day']:02d}")
        lines.append(f"Start Date: {'-'.join(date_parts)}")
    
    end = anime_data.get('endDate', {})
    if end.get('year'):
        date_parts = [str(end['year'])]
        if end.get('month'):
            date_parts.append(f"{end['month']:02d}")
        if end.get('day'):
            date_parts.append(f"{end['day']:02d}")
        lines.append(f"End Date: {'-'.join(date_parts)}")
    
    if anime_data.get('episodes'):
        lines.append(f"Episodes: {anime_data['episodes']}")
    if anime_data.get('status'):
        lines.append(f"Status: {anime_data['status']}")
    if anime_data.get('format'):
        lines.append(f"Format: {anime_data['format']}")
    if anime_data.get('season'):
        lines.append(f"Season: {anime_data['season']} {anime_data.get('seasonYear', '')}".strip())
    if anime_data.get('source'):
        lines.append(f"Source: {anime_data['source']}")
    if anime_data.get('averageScore'):
        lines.append(f"Average Score: {anime_data['averageScore']}/100")
    if anime_data.get('popularity'):
        lines.append(f"Popularity: #{anime_data['popularity']}")
    
    if anime_data.get('genres'):
        lines.append(f"Genres: {', '.join(anime_data['genres'])}")
    
    if anime_data.get('studios', {}).get('nodes'):
        studio_names = [s['name'] for s in anime_data['studios']['nodes'] if s.get('name')]
        if studio_names:
            lines.append(f"Studios: {', '.join(studio_names)}")
    
    # Add synonyms if available
    if anime_data.get('synonyms'):
        lines.append(f"Also Known As: {', '.join(anime_data['synonyms'])}")
    
    lines.append("")  # Empty line before synopsis
    lines.append("Synopsis:")
    lines.append(description.strip())
    
    content = "\n".join(lines)
    
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
        
        # Try DeviantArt search first, then fall back to AniList cover
        print(f"  Searching for folder icon: {clean_name} folder icon...")
        icon_url = search_deviantart_folder_icon(clean_name)
        
        if not icon_url:
            print(f"  DeviantArt search didn't find results, falling back to AniList...")
            anime = search_anime(folder.name)
            if anime:
                anilist_cover = anime.get('coverImage', {}).get('large')
                if anilist_cover:
                    icon_url = anilist_cover
                    print(f"  Using AniList cover: {anilist_cover[:80]}...")
        
        anime = search_anime(folder.name)
        if anime:
            print(f"  Found on AniList: {anime['title']['romaji']}")
            safe_name = sanitize_filename(anime['title']['romaji'])
        else:
            print(f"  No AniList match, using folder name")
            safe_name = sanitize_filename(clean_name)
        
        if not icon_url:
            print(f"  No icon found")
            continue
        
        print(f"  Using: {icon_url[:80]}...")
        
        # Save icon directly in the anime folder for better organization
        png_path = folder / f"{safe_name}.png"
        ico_path = folder / f"{safe_name}.ico"

        if not png_path.exists():
            print(f"  Downloading image...")
            if not download_image(icon_url, png_path):
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