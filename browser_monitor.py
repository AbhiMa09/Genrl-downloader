"""
Browser tab monitor for detecting MyAnimeList tabs in Zen Browser
"""
import psutil
import time
import re
import threading
import json
import os
import requests
from pathlib import Path
from typing import List, Dict, Optional, Callable
from dataclasses import dataclass
from datetime import datetime, timedelta
import logging

try:
    import pygetwindow as gw
    GETWINDOW_AVAILABLE = True
except ImportError:
    GETWINDOW_AVAILABLE = False
    print("pygetwindow not installed. Window monitoring will be limited.")

try:
    from pynput import keyboard
    PYNPUT_AVAILABLE = True
except ImportError:
    PYNPUT_AVAILABLE = False
    print("pynput not installed. Hotkey functionality will be limited.")

@dataclass
class AnimeTabInfo:
    """Information about an anime browser tab"""
    title: str
    url: str
    process_name: str
    timestamp: datetime
    anime_name: Optional[str] = None
    confidence: float = 0.0
    mal_id: Optional[int] = None
    total_episodes: Optional[int] = None
    total_seasons: Optional[int] = None
    title_variants: Optional[List[str]] = None  # All title variants from AniList for comprehensive searching

class BrowserMonitor:
    def __init__(self, check_interval: int = 3):
        self.check_interval = check_interval
        self.is_monitoring = False
        self.monitor_thread = None
        self.callbacks: List[Callable[[AnimeTabInfo], None]] = []
        self.logger = logging.getLogger(__name__)
        self.last_known_tabs: Dict[str, AnimeTabInfo] = {}
        self.anime_cache: Dict[str, Dict] = {}  # Cache anime info from AniList

        # Rate limiting for callbacks
        self.max_callbacks_per_cycle = 3  # Max 3 tabs per check cycle
        self.min_time_between_callbacks = 2.0  # Minimum seconds between callback batches
        self.last_callback_time = 0

        # MyAnimeList-specific patterns - ONLY track these
        self.mal_patterns = [
            r'(?i)myanimelist\.net',
            r'(?i)myanimelist\.com',
            r'(?i)myanimelist',
        ]

        # Patterns for extracting anime names from MyAnimeList titles
        self.title_patterns = [
            r'(?i)^([^-\|]+?)\s*[-|]\s*MyAnimeList\.net',
            r'(?i)^([^-\|]+?)\s*[-|]\s*MyAnimeList',
            r'(?i)^([^-\|]+?)\s*[-|]\s*Episode\s*\d+',
            r'(?i)^([^-\|]+?)\s*[-|]\s*\d+',
            r'(?i)^([^-\|]+?)\s*[-|]\s*[Ss]eason',
        ]

        # MAL URL pattern to extract anime ID
        self.mal_url_pattern = r'myanimelist\.net/anime/(\d+)/'

        # Zen Browser profile path (Firefox-based)
        self.zen_profile_path = self._find_zen_profile()

        # AniList GraphQL endpoint
        self.anilist_url = "https://graphql.anilist.co"
        self.anilist_query = """
        query ($search: String, $id: Int, $idMal: Int) {
          Media(search: $search, id: $id, idMal: $idMal, type: ANIME) {
            id
            idMal
            title {
              romaji
              english
              native
            }
            synonyms
            episodes
            format
            status
            seasonYear
            season
            coverImage {
              large
            }
          }
        }
        """

    def _extract_mal_id(self, url: str) -> Optional[int]:
        """Extract MyAnimeList anime ID from URL"""
        match = re.search(self.mal_url_pattern, url)
        if match:
            return int(match.group(1))
        return None

    def _calculate_title_similarity(self, window_title: str, session_title: str) -> float:
        """Calculate similarity between window title and session title"""
        # Normalize both titles
        wt = window_title.lower().strip()
        st = session_title.lower().strip()
        
        # Remove common browser suffixes
        for suffix in [' - zen browser', ' - google chrome', ' - mozilla firefox', ' - microsoft edge', ' - brave']:
            if wt.endswith(suffix):
                wt = wt[:-len(suffix)].strip()
            if st.endswith(suffix):
                st = st[:-len(suffix)].strip()
        
        # Exact match
        if wt == st:
            return 1.0
        
        # One contains the other
        if wt in st or st in wt:
            return 0.8
        
        # Word overlap
        wt_words = set(wt.split())
        st_words = set(st.split())
        if wt_words and st_words:
            intersection = wt_words & st_words
            union = wt_words | st_words
            jaccard = len(intersection) / len(union) if union else 0
            return jaccard
        
        return 0.0

    def _get_anime_info_from_anilist(self, anime_name: str = None, mal_id: int = None) -> Optional[Dict]:
        """Get anime info from AniList API"""
        cache_key = f"mal_{mal_id}" if mal_id else f"name_{anime_name}"
        if cache_key in self.anime_cache:
            return self.anime_cache[cache_key]

        variables = {}
        if mal_id:
            variables['idMal'] = mal_id
        elif anime_name:
            variables['search'] = anime_name
        else:
            return None

        try:
            response = requests.post(
                self.anilist_url,
                json={"query": self.anilist_query, "variables": variables},
                timeout=10
            )
            response.raise_for_status()
            data = response.json()

            if "data" in data and data["data"] and data["data"]["Media"]:
                anime_data = data["data"]["Media"]
                # Estimate seasons based on episodes and format
                episodes = anime_data.get('episodes')
                format_type = anime_data.get('format', '')
                
                # Estimate seasons: TV series typically 12-24 eps per season
                total_seasons = None
                if episodes and format_type == 'TV':
                    if episodes <= 13:
                        total_seasons = 1
                    elif episodes <= 26:
                        total_seasons = 2
                    elif episodes <= 39:
                        total_seasons = 3
                    elif episodes <= 52:
                        total_seasons = 4
                    else:
                        total_seasons = (episodes + 23) // 24  # Rough estimate
                
                # Collect all title variants for comprehensive searching
                title_variants = []
                if anime_data['title'].get('romaji'):
                    title_variants.append(anime_data['title']['romaji'])
                if anime_data['title'].get('english'):
                    title_variants.append(anime_data['title']['english'])
                if anime_data['title'].get('native'):
                    title_variants.append(anime_data['title']['native'])
                # Add synonyms if available
                if anime_data.get('synonyms'):
                    title_variants.extend(anime_data['synonyms'])
                # Deduplicate while preserving order
                seen = set()
                unique_variants = []
                for v in title_variants:
                    if v and v not in seen:
                        seen.add(v)
                        unique_variants.append(v)

                result = {
                    'id': anime_data['id'],
                    'mal_id': anime_data.get('idMal'),
                    'title_romaji': anime_data['title'].get('romaji'),
                    'title_english': anime_data['title'].get('english'),
                    'title_native': anime_data['title'].get('native'),
                    'title_variants': unique_variants,  # All title variants for comprehensive searching
                    'episodes': episodes,
                    'format': format_type,
                    'status': anime_data.get('status'),
                    'season_year': anime_data.get('seasonYear'),
                    'season': anime_data.get('season'),
                    'cover_image': anime_data['coverImage'].get('large'),
                    'estimated_seasons': total_seasons,
                }
                self.anime_cache[cache_key] = result
                self.logger.debug(f"Cached anime {unique_variants[0]} with {len(unique_variants)} title variants: {unique_variants}")
                return result
        except Exception as e:
            self.logger.error(f"Failed to get anime info from AniList: {e}")
        return None

    def _find_zen_profile(self) -> Optional[Path]:
        """Find Zen Browser profile directory"""
        # Common locations for Zen Browser
        possible_paths = [
            Path(os.environ.get('APPDATA', '')) / 'Zen' / 'Profiles',
            Path(os.environ.get('LOCALAPPDATA', '')) / 'Zen' / 'Profiles',
            Path.home() / 'AppData' / 'Roaming' / 'Zen' / 'Profiles',
            Path.home() / 'AppData' / 'Local' / 'Zen' / 'Profiles',
        ]
        
        for base_path in possible_paths:
            if base_path.exists():
                # Find the default profile
                for profile_dir in base_path.iterdir():
                    if profile_dir.is_dir() and (profile_dir / 'sessionstore-backups').exists():
                        self.logger.info(f"Found Zen Browser profile: {profile_dir}")
                        return profile_dir
        
        self.logger.warning("Zen Browser profile not found")
        return None

    def _read_zen_session(self) -> List[Dict]:
        """Read Zen Browser session file to get tab URLs"""
        tabs_data = []
        
        if not self.zen_profile_path:
            self.logger.warning("Zen profile path not found")
            return tabs_data
        
        session_dir = self.zen_profile_path / 'sessionstore-backups'
        if not session_dir.exists():
            self.logger.warning(f"Session directory not found: {session_dir}")
            return tabs_data
        
        # List all session files for debugging
        all_files = list(session_dir.iterdir())
        self.logger.debug(f"Session files found: {[f.name for f in all_files]}")
        
        # Try recovery.js first (most recent), then previous.js, then recovery.bak
        session_files = [
            session_dir / 'recovery.jsonlz4',
            session_dir / 'recovery.baklz4',  # Backup recovery file
            session_dir / 'recovery.js',
            session_dir / 'previous.jsonlz4',
            session_dir / 'previous.js',
        ]
        
        for session_file in session_files:
            if session_file.exists():
                try:
                    tabs_data = self._parse_session_file(session_file)
                    if tabs_data:
                        self.logger.info(f"Successfully read {len(tabs_data)} tabs from {session_file.name}")
                        # Log first few tabs for debugging
                        for tab in tabs_data[:5]:
                            self.logger.debug(f"  Tab: {tab['title'][:60]} -> {tab['url'][:80]}")
                        break
                    else:
                        self.logger.warning(f"Parsed {session_file.name} but got no tabs")
                except Exception as e:
                    self.logger.error(f"Failed to read {session_file}: {e}")
                    import traceback
                    traceback.print_exc()
                    continue
            else:
                self.logger.debug(f"Session file not found: {session_file}")
        
        if not tabs_data:
            self.logger.warning("No tabs found in any session file")
        
        return tabs_data

    def _parse_session_file(self, session_file: Path) -> List[Dict]:
        """Parse Firefox/Zen session file (JSON or JSONLZ4)"""
        tabs_data = []
        
        if session_file.suffix == '.jsonlz4':
            # Decompress LZ4
            try:
                import lz4.block
                with open(session_file, 'rb') as f:
                    # Skip the 8-byte header
                    f.read(8)
                    compressed = f.read()
                    decompressed = lz4.block.decompress(compressed)
                    data = json.loads(decompressed)
            except ImportError:
                self.logger.warning("lz4 not installed, cannot read .jsonlz4 files")
                return tabs_data
            except Exception as e:
                self.logger.debug(f"Failed to decompress {session_file}: {e}")
                return tabs_data
        else:
            # Regular JSON
            try:
                with open(session_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
            except Exception as e:
                self.logger.debug(f"Failed to parse {session_file}: {e}")
                return tabs_data
        
        # Extract tabs from session data
        if 'windows' in data:
            for window in data['windows']:
                if 'tabs' in window:
                    for tab in window['tabs']:
                        if 'entries' in tab and tab['entries']:
                            # Get the current entry (last one)
                            current_entry = tab['entries'][-1]
                            url = current_entry.get('url', '')
                            title = current_entry.get('title', '')
                            if url and title:
                                tabs_data.append({
                                    'url': url,
                                    'title': title,
                                })
        
        return tabs_data

    def add_callback(self, callback: Callable[[AnimeTabInfo], None]):
        """Add a callback to be called when anime tab is detected"""
        self.callbacks.append(callback)

    def remove_callback(self, callback: Callable[[AnimeTabInfo], None]):
        """Remove a callback"""
        if callback in self.callbacks:
            self.callbacks.remove(callback)

    def _extract_anime_name_from_title(self, title: str) -> Optional[str]:
        """Extract anime name from MyAnimeList browser tab title"""
        # Clean the title first
        cleaned = re.sub(r'\s*[\(\[].*?[\)\]]', '', title)  # Remove parentheses/brackets content
        cleaned = re.sub(r'\s+', ' ', cleaned).strip()  # Normalize whitespace

        # Try each pattern
        for pattern in self.title_patterns:
            match = re.search(pattern, cleaned)
            if match:
                anime_name = match.group(1).strip()
                # Clean up common suffixes
                anime_name = re.sub(r'\s*(dub|sub|hd|4k|1080p|720p|480p)\s*$', '', anime_name, flags=re.IGNORECASE)
                return anime_name.strip() if anime_name.strip() else None

        # If no pattern matches but it's a MAL page, try to get name before " - MyAnimeList"
        if 'myanimelist' in cleaned.lower():
            separators = [' - MyAnimeList', ' - myanimelist', ' | MyAnimeList', ' | myanimelist']
            for sep in separators:
                if sep in cleaned:
                    parts = cleaned.split(sep)
                    if len(parts) > 1 and len(parts[0].strip()) > 2:
                        return parts[0].strip()

        return None

    def _is_mal_tab(self, title: str, url: str = "") -> tuple[bool, float]:
        """Check if a tab is a MyAnimeList tab and return confidence score"""
        confidence = 0.0
        text_to_check = f"{title} {url}".lower()

        # Check for MyAnimeList patterns
        matches = 0
        for pattern in self.mal_patterns:
            if re.search(pattern, text_to_check):
                matches += 1
                confidence += 0.5  # Each match adds confidence

        # Boost confidence for exact myanimelist.net
        if 'myanimelist.net' in text_to_check:
            confidence += 0.3

        # Cap confidence at 1.0
        confidence = min(confidence, 1.0)

        # Only consider it a MAL tab if confidence is high enough
        return confidence >= 0.5, confidence

    def _get_browser_tabs(self) -> List[AnimeTabInfo]:
        """Get information about browser tabs - only MyAnimeList tabs from session file"""
        tabs = []

        if not GETWINDOW_AVAILABLE:
            self.logger.warning("pygetwindow not available - cannot detect browser tabs")
            return tabs

        # Get tabs from Zen session file (has actual URLs for ALL tabs, active and inactive)
        session_tabs = self._read_zen_session()
        
        if not session_tabs:
            self.logger.debug("No tabs found in session file")
            return tabs

        self.logger.debug(f"Processing {len(session_tabs)} tabs from session file")

        # Get Zen Browser process name for all tabs
        process_name = "zen.exe"
        
        # Check if Zen Browser is running
        zen_running = False
        for proc in psutil.process_iter(['name']):
            try:
                if 'zen.exe' in proc.info['name'].lower():
                    zen_running = True
                    process_name = proc.info['name']
                    break
            except:
                continue

        if not zen_running:
            self.logger.debug("Zen Browser not running")
            return tabs

        # Process ALL tabs from session file - filter for MyAnimeList
        mal_tabs_found = 0
        for tab_data in session_tabs:
            url = tab_data.get('url', '')
            title = tab_data.get('title', '')
            
            if not url or not title:
                continue
            
            # Check if it's a MyAnimeList URL
            is_mal, confidence = self._is_mal_tab(title, url)
            if is_mal:
                # Skip generic pages (seasonal list, home page, search, etc.)
                generic_patterns = [
                    r'/anime/season/',
                    r'/anime/genre/',
                    r'myanimelist\.net/?$',
                    r'myanimelist\.net/panel',
                    r'/search\?',
                    r'/user/',
                    r'/forum/',
                    r'/news/',
                    r'/reviews/',
                    r'/recommendations/',
                ]
                is_generic = any(re.search(p, url, re.IGNORECASE) for p in generic_patterns)
                if is_generic:
                    self.logger.debug(f"Skipping generic MAL page: {title[:60]} -> {url[:80]}")
                    continue
                
                mal_tabs_found += 1
                anime_name = self._extract_anime_name_from_title(title)
                
                # Extract MAL ID from URL and get anime info from AniList
                mal_id = None
                total_episodes = None
                total_seasons = None
                
                mal_id = self._extract_mal_id(url)
                if mal_id:
                    anime_info = self._get_anime_info_from_anilist(mal_id=mal_id)
                    if anime_info:
                        anime_name = anime_info.get('title_romaji') or anime_name
                        total_episodes = anime_info.get('episodes')
                        total_seasons = anime_info.get('estimated_seasons')
                        self.logger.info(f"AniList info: {anime_name} - {total_episodes} episodes, ~{total_seasons} seasons")
                
                tab_info = AnimeTabInfo(
                    title=title,
                    url=url,
                    process_name=process_name,
                    timestamp=datetime.now(),
                    anime_name=anime_name,
                    confidence=confidence,
                    mal_id=mal_id,
                    total_episodes=total_episodes,
                    total_seasons=total_seasons,
                    title_variants=anime_info.get('title_variants') if anime_info else None
                )
                tabs.append(tab_info)
                self.logger.info(f"MAL tab from session: {title[:80]} -> {anime_name or 'Unknown'} (confidence: {confidence:.2f})")
                self.logger.info(f"  URL: {url}")
                if total_seasons:
                    self.logger.info(f"  Estimated seasons: {total_seasons}, Episodes: {total_episodes}")

        self.logger.info(f"Found {mal_tabs_found} MyAnimeList tabs in session")
        return tabs

    def _get_browser_processes(self) -> List[AnimeTabInfo]:
        """Fallback method: check browser processes (limited info)"""
        tabs = []
        browser_processes = ['zen.exe', 'chrome.exe', 'firefox.exe', 'msedge.exe', 'oper.exe', 'brave.exe']

        for proc in psutil.process_iter(['pid', 'name']):
            try:
                if proc.info['name'].lower() in browser_processes:
                    tab_info = AnimeTabInfo(
                        title=f"{proc.info['name']} process (no window info)",
                        url="",
                        process_name=proc.info['name'],
                        timestamp=datetime.now(),
                        anime_name=None,
                        confidence=0.1
                    )
                    tabs.append(tab_info)
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                continue

        return tabs

    def _monitor_loop(self):
        """Main monitoring loop with rate limiting"""
        self.logger.info("Starting browser monitor (MyAnimeList only)...")
        while self.is_monitoring:
            try:
                current_tabs = self._get_browser_tabs()

                # Check for new MAL tabs
                new_tabs = []
                for tab in current_tabs:
                    tab_key = f"{tab.title}_{tab.process_name}"
                    is_new = tab_key not in self.last_known_tabs
                    has_changed_confidence = (
                        tab_key in self.last_known_tabs and
                        abs(tab.confidence - self.last_known_tabs[tab_key].confidence) > 0.2
                    )

                    if is_new or has_changed_confidence:
                        new_tabs.append(tab)

                # Rate limit: only process max_callbacks_per_cycle tabs per cycle
                if new_tabs:
                    # Sort by confidence (highest first) to prioritize best matches
                    new_tabs.sort(key=lambda t: t.confidence, reverse=True)
                    
                    # Limit to max_callbacks_per_cycle
                    limited_tabs = new_tabs[:self.max_callbacks_per_cycle]
                    
                    # Check if enough time has passed since last callback batch
                    current_time = time.time()
                    if current_time - self.last_callback_time >= self.min_time_between_callbacks:
                        self.last_callback_time = current_time
                        
                        for tab in limited_tabs:
                            self.logger.info(f"New/changed MAL tab: {tab.title} (confidence: {tab.confidence:.2f})")
                            # Notify callbacks
                            for callback in self.callbacks:
                                try:
                                    callback(tab)
                                except Exception as e:
                                    self.logger.error(f"Error in browser monitor callback: {e}")
                    else:
                        self.logger.debug(f"Rate limited: {len(limited_tabs)} tabs queued, waiting for cooldown")

                # Update last known tabs
                self.last_known_tabs = {f"{tab.title}_{tab.process_name}": tab for tab in current_tabs}

                time.sleep(self.check_interval)
            except Exception as e:
                self.logger.error(f"Error in browser monitor loop: {e}")
                time.sleep(self.check_interval)

    def start_monitoring(self):
        """Start monitoring browser tabs"""
        if self.is_monitoring:
            self.logger.warning("Browser monitor is already running")
            return

        self.is_monitoring = True
        self.monitor_thread = threading.Thread(target=self._monitor_loop, daemon=False)
        self.monitor_thread.start()
        self.logger.info("Browser monitor started (MyAnimeList only)")

    def stop_monitoring(self):
        """Stop monitoring browser tabs"""
        if not self.is_monitoring:
            self.logger.warning("Browser monitor is not running")
            return

        self.is_monitoring = False
        if self.monitor_thread and self.monitor_thread.is_alive():
            print("[BROWSER_MONITOR] Waiting for thread to finish...")
            self.monitor_thread.join(timeout=5)
            print("[BROWSER_MONITOR] Thread joined")
        self.logger.info("Browser monitor stopped")

    def get_current_anime_tabs(self) -> List[AnimeTabInfo]:
        """Get current MyAnimeList tabs"""
        return self._get_browser_tabs()

if __name__ == "__main__":
    # Test the browser monitor
    logging.basicConfig(level=logging.DEBUG)
    monitor = BrowserMonitor(check_interval=3)

    def on_anime_tab_detected(tab: AnimeTabInfo):
        print(f"[DETECTED] {tab.title} -> {tab.anime_name or 'Unknown'} (confidence: {tab.confidence:.2f})")

    monitor.add_callback(on_anime_tab_detected)

    print("Starting browser monitor for 30 seconds (MyAnimeList only)...")
    monitor.start_monitoring()

    try:
        time.sleep(30)
    except KeyboardInterrupt:
        print("\nStopping...")
    finally:
        monitor.stop_monitoring()
        print("Browser monitor stopped.")