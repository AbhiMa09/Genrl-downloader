"""
Torrent manager for qBittorrent client
"""
import logging
from typing import List, Dict, Optional
try:
    from qbittorrentapi import Client, LoginFailed, APIConnectionError
    QBITTORRENT_AVAILABLE = True
except ImportError:
    QBITTORRENT_AVAILABLE = False
    print("qbittorrent-api not installed. Torrent functionality will be limited.")

import time

class TorrentManager:
    def __init__(self, host: str = "localhost", port: int = 8080,
                 username: str = "admin", password: str = "adminadmin"):
        self.host = host
        self.port = port
        self.username = username
        self.password = password
        self.client = None
        self.logger = logging.getLogger(__name__)

        if QBITTORRENT_AVAILABLE:
            self._connect()

    def _connect(self):
        """Connect to qBittorrent client"""
        try:
            self.client = Client(
                host=f"http://{self.host}:{self.port}",
                username=self.username,
                password=self.password
            )
            self.client.auth_log_in()
            self.logger.info("Connected to qBittorrent successfully")
        except LoginFailed:
            self.logger.error("Failed to login to qBittorrent. Check credentials.")
            self.client = None
        except APIConnectionError:
            self.logger.error(f"Failed to connect to qBittorrent at {self.host}:{self.port}")
            self.client = None
        except Exception as e:
            self.logger.error(f"Unexpected error connecting to qBittorrent: {e}")
            self.client = None

    def is_connected(self) -> bool:
        """Check if connected to qBittorrent"""
        if not QBITTORRENT_AVAILABLE or not self.client:
            return False
        try:
            self.client.app_version()  # This will raise an exception if not connected
            return True
        except:
            return False

    def add_torrent(self, magnet_link: str, save_path: str = None,
                    tags: List[str] = None) -> bool:
        """
        Add a torrent magnet link to qBittorrent
        """
        if not self.is_connected():
            self.logger.error("Not connected to qBittorrent")
            return False

        try:
            # Add the torrent
            result = self.client.torrents_add(
                urls=magnet_link,
                save_path=save_path,
                tags=tags or []
            )

            # qBittorrent should return "Ok." on success, but let's be more flexible
            # Handle possible variations: "Ok", "Ok.", "OK.", etc.
            if isinstance(result, str) and result.strip() in ["Ok.", "Ok", "OK.", "OK"]:
                self.logger.info(f"Successfully added torrent: {magnet_link[:50]}...")
                return True
            else:
                # Log the actual result for debugging
                self.logger.error(f"Failed to add torrent. Expected 'Ok.' variant, got: {repr(result)}")
                return False
        except Exception as e:
            self.logger.error(f"Error adding torrent: {e}")
            return False

    def get_torrents(self, filter_status: str = "all") -> List[Dict]:
        """
        Get list of torrents from qBittorrent
        filter_status: all, downloading, seeding, completed, paused, active, inactive, resumed, stalled, stalled_uploading, stalled_downloading, checking, error, missingFiles
        """
        if not self.is_connected():
            self.logger.error("Not connected to qBittorrent")
            return []

        try:
            torrents = self.client.torrents_info(status_filter=filter_status)
            result = []
            for torrent in torrents:
                result.append({
                    'name': torrent.name,
                    'hash': torrent.hash,
                    'state': torrent.state,
                    'progress': torrent.progress,
                    'download_speed': torrent.dlspeed,
                    'upload_speed': torrent.upspeed,
                    'eta': torrent.eta,
                    'size': torrent.size,
                    'downloaded': torrent.downloaded,
                    'uploaded': torrent.uploaded,
                    'ratio': torrent.ratio,
                    'seeds': torrent.num_seeds,
                    'peers': torrent.num_leechs
                })
            return result
        except Exception as e:
            self.logger.error(f"Error getting torrents: {e}")
            return []

    def remove_torrent(self, torrent_hash: str, delete_files: bool = False) -> bool:
        """
        Remove a torrent from qBittorrent
        """
        if not self.is_connected():
            self.logger.error("Not connected to qBittorrent")
            return False

        try:
            self.client.torrents_delete(
                torrent_hashes=torrent_hash,
                delete_files=delete_files
            )
            self.logger.info(f"Removed torrent: {torrent_hash}")
            return True
        except Exception as e:
            self.logger.error(f"Error removing torrent: {e}")
            return False

    def pause_torrent(self, torrent_hash: str) -> bool:
        """Pause a torrent"""
        if not self.is_connected():
            return False
        try:
            self.client.torrents_pause(torrent_hashes=torrent_hash)
            return True
        except Exception as e:
            self.logger.error(f"Error pausing torrent: {e}")
            return False

    def resume_torrent(self, torrent_hash: str) -> bool:
        """Resume a torrent"""
        if not self.is_connected():
            return False
        try:
            self.client.torrents_resume(torrent_hashes=torrent_hash)
            return True
        except Exception as e:
            self.logger.error(f"Error resuming torrent: {e}")
            return False

    def get_search_results_via_qbittorrent(self, search_term: str, limit: int = 10) -> List[Dict]:
        """
        Use qBittorrent's built-in search (if enabled) to search for torrents
        Note: This requires the search plugin to be enabled in qBittorrent
        """
        if not self.is_connected():
            self.logger.error("Not connected to qBittorrent")
            return []

        try:
            # This searches using qBittorrent's search tab
            results = self.client.search(
                pattern=search_term,
                category="All",  # You can specify categories like "anime"
                limit=limit
            )

            formatted_results = []
            for result in results:
                formatted_results.append({
                    'name': result.get('name', ''),
                    'size': result.get('size', 0),
                    'seeders': result.get('num_seeds', 0),
                    'leechers': result.get('num_leechs', 0),
                    'magnet_uri': result.get('magnet_uri', ''),
                    'source': result.get('source', ''),
                    'timestamp': result.get('timestamp', 0)
                })

            return formatted_results
        except Exception as e:
            self.logger.error(f"Error searching via qBittorrent: {e}")
            return []

if __name__ == "__main__":
    # Test the torrent manager
    logging.basicConfig(level=logging.INFO)
    tm = TorrentManager()

    if tm.is_connected():
        print("Connected to qBittorrent!")
        # Example: Get torrents
        torrents = tm.get_torrents()
        print(f"Found {len(torrents)} torrents")
        for torrent in torrents[:3]:  # Show first 3
            print(f"- {torrent['name']} ({torrent['state']})")
    else:
        print("Not connected to qBittorrent. Make sure it's running on localhost:8080")