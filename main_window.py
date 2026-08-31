"""
Main window for the General Downloader application
"""
import sys
import os
import re
import logging
from typing import List, Optional
from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QTabWidget, QTableWidget, QTableWidgetItem, QPushButton, QLabel,
    QLineEdit, QTextEdit, QProgressBar, QMessageBox, QHeaderView,
    QMenuBar, QMenu, QStatusBar, QSystemTrayIcon, QDialog,
    QFormLayout, QSpinBox, QTimeEdit, QCheckBox, QGroupBox,
    QFileDialog, QSplitter, QTreeWidget, QTreeWidgetItem
)
from PyQt6.QtCore import Qt, QThread, pyqtSignal, QTimer, QTime, QMetaObject, Q_ARG
from PyQt6.QtGui import QAction, QIcon, QFont, QPixmap

# Import our modules
from nyaa_scraper import NyaaScraper
from torrent_manager import TorrentManager
from browser_monitor import BrowserMonitor, AnimeTabInfo
from scheduler import scheduler, start_scheduler, stop_scheduler, schedule_daily_10pm_icon_update

# Set up logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# SweetAlert2-style notification system with smooth animations
class SweetAlert:
    """SweetAlert2-style notifications for PyQt6 using custom animated widgets"""
    
    _toasts = []  # Keep references to prevent garbage collection
    
    @staticmethod
    def _create_alert_widget(icon_type: str, title: str, text: str, buttons: list = None, parent=None) -> 'AlertWidget':
        """Create a custom animated alert widget"""
        return AlertWidget(icon_type, title, text, buttons or ["OK"], parent)
    
    @staticmethod
    def success(title: str, text: str, parent=None) -> int:
        """Show success alert with animation"""
        alert = SweetAlert._create_alert_widget("success", title, text, ["OK"], parent)
        alert.show_animated()
        return alert.exec_()
    
    @staticmethod
    def error(title: str, text: str, parent=None) -> int:
        """Show error alert with animation"""
        alert = SweetAlert._create_alert_widget("error", title, text, ["OK"], parent)
        alert.show_animated()
        return alert.exec_()
    
    @staticmethod
    def warning(title: str, text: str, parent=None) -> int:
        """Show warning alert with animation"""
        alert = SweetAlert._create_alert_widget("warning", title, text, ["OK"], parent)
        alert.show_animated()
        return alert.exec_()
    
    @staticmethod
    def info(title: str, text: str, parent=None) -> int:
        """Show info alert with animation"""
        alert = SweetAlert._create_alert_widget("info", title, text, ["OK"], parent)
        alert.show_animated()
        return alert.exec_()
    
    @staticmethod
    def confirm(title: str, text: str, parent=None) -> bool:
        """Show confirmation dialog with animation"""
        alert = SweetAlert._create_alert_widget("question", title, text, ["Yes", "No"], parent)
        alert.show_animated()
        return alert.exec_() == 1  # Yes = 1
    
    @staticmethod
    def toast(title: str, text: str, parent=None, duration: int = 3000):
        """Show a temporary toast notification (non-blocking, animated)"""
        toast = ToastWidget(title, text, parent, duration)
        SweetAlert._toasts.append(toast)  # Keep reference
        toast.show_animated()
        return toast


class AlertWidget(QDialog):
    """Custom animated alert dialog with SweetAlert2 styling"""
    
    def __init__(self, icon_type: str, title: str, text: str, buttons: list, parent=None):
        super().__init__(parent)
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint | Qt.WindowType.Dialog)
        self.setModal(True)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        self.setFixedSize(420, 220)
        
        self.result_value = 0
        self._animation = None
        
        # Main container with rounded corners
        self.container = QWidget(self)
        self.container.setFixedSize(400, 200)
        self.container.setStyleSheet("""
            QWidget {
                background-color: #2d2d2d;
                border-radius: 16px;
                border: 1px solid #3d3d3d;
            }
        """)
        
        layout = QVBoxLayout(self.container)
        layout.setContentsMargins(24, 24, 24, 24)
        layout.setSpacing(16)
        
        # Icon
        icon_map = {
            "success": ("✓", "#4CAF50"),
            "error": ("✗", "#f44336"),
            "warning": ("⚠", "#ff9800"),
            "info": ("ℹ", "#2196f3"),
            "question": ("?", "#2196f3"),
        }
        icon_char, icon_color = icon_map.get(icon_type, ("ℹ", "#2196f3"))
        
        icon_label = QLabel(icon_char)
        icon_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        icon_label.setStyleSheet(f"""
            QLabel {{
                color: {icon_color};
                font-size: 56px;
                font-weight: bold;
            }}
        """)
        layout.addWidget(icon_label)
        
        # Title
        title_label = QLabel(title)
        title_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        title_label.setWordWrap(True)
        title_label.setStyleSheet("""
            QLabel {
                color: #ffffff;
                font-size: 18px;
                font-weight: 600;
            }
        """)
        layout.addWidget(title_label)
        
        # Text
        text_label = QLabel(text)
        text_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        text_label.setWordWrap(True)
        text_label.setStyleSheet("""
            QLabel {
                color: #cccccc;
                font-size: 14px;
                line-height: 1.5;
            }
        """)
        layout.addWidget(text_label)
        
        layout.addStretch()
        
        # Buttons
        btn_layout = QHBoxLayout()
        btn_layout.setSpacing(12)
        btn_layout.addStretch()
        
        for i, btn_text in enumerate(buttons):
            btn = QPushButton(btn_text)
            btn.setFixedHeight(40)
            btn.setCursor(Qt.CursorShape.PointingHandCursor)
            
            if i == 0:  # Primary button
                btn.setStyleSheet("""
                    QPushButton {
                        background-color: #3085d6;
                        color: white;
                        border: none;
                        border-radius: 8px;
                        padding: 10px 28px;
                        font-size: 14px;
                        font-weight: 600;
                        min-width: 90px;
                    }
                    QPushButton:hover { background-color: #2a75c0; }
                    QPushButton:pressed { background-color: #1e5fa0; }
                """)
            else:  # Secondary button
                btn.setStyleSheet("""
                    QPushButton {
                        background-color: transparent;
                        color: #cccccc;
                        border: 1px solid #444;
                        border-radius: 8px;
                        padding: 10px 28px;
                        font-size: 14px;
                        font-weight: 500;
                        min-width: 90px;
                    }
                    QPushButton:hover { background-color: #3a3a3a; border-color: #555; }
                    QPushButton:pressed { background-color: #2a2a2a; }
                """)
            
            btn.clicked.connect(lambda checked, r=i: self._on_button_clicked(r))
            btn_layout.addWidget(btn)
        
        layout.addLayout(btn_layout)
        
        # Main layout for dialog
        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(10, 10, 10, 10)
        main_layout.addWidget(self.container, alignment=Qt.AlignmentFlag.AlignCenter)
        
        # Center on parent
        if parent:
            self.move(parent.geometry().center() - self.rect().center())
    
    def _on_button_clicked(self, index: int):
        self.result_value = index
        self.close_animated()
    
    def show_animated(self):
        """Show with fade-in + scale animation"""
        self.setWindowOpacity(0)
        self.show()
        
        self._animation = QPropertyAnimation(self, b"windowOpacity")
        self._animation.setDuration(200)
        self._animation.setStartValue(0.0)
        self._animation.setEndValue(1.0)
        self._animation.setEasingCurve(QEasingCurve.Type.OutCubic)
        self._animation.start()
    
    def close_animated(self):
        """Close with fade-out animation"""
        if self._animation:
            self._animation.stop()
        
        self._animation = QPropertyAnimation(self, b"windowOpacity")
        self._animation.setDuration(150)
        self._animation.setStartValue(self.windowOpacity())
        self._animation.setEndValue(0.0)
        self._animation.setEasingCurve(QEasingCurve.Type.InCubic)
        self._animation.finished.connect(self.close)
        self._animation.start()
    
    def exec_(self):
        self.show_animated()
        super().exec_()
        return self.result_value


class ToastWidget(QWidget):
    """Animated toast notification (non-blocking)"""
    
    def __init__(self, title: str, text: str, parent=None, duration: int = 3000):
        super().__init__(parent)
        self.duration = duration
        self.setWindowFlags(
            Qt.WindowType.FramelessWindowHint | 
            Qt.WindowType.ToolTip | 
            Qt.WindowType.WindowStaysOnTopHint |
            Qt.WindowType.NoDropShadowWindowHint
        )
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        self.setAttribute(Qt.WidgetAttribute.WA_ShowWithoutActivating)
        
        self.setFixedWidth(340)
        
        # Container with border-radius and shadow
        self.container = QWidget(self)
        self.container.setStyleSheet("""
            QWidget {
                background-color: #2d2d2d;
                border-radius: 12px;
                border: 1px solid #3d3d3d;
                border-left: 4px solid #3085d6;
            }
        """)
        
        layout = QVBoxLayout(self.container)
        layout.setContentsMargins(16, 14, 16, 14)
        layout.setSpacing(6)
        
        # Title
        title_label = QLabel(f"<b>{title}</b>")
        title_label.setStyleSheet("color: #ffffff; font-size: 14px; font-weight: 600;")
        title_label.setWordWrap(True)
        layout.addWidget(title_label)
        
        # Text
        text_label = QLabel(text)
        text_label.setWordWrap(True)
        text_label.setStyleSheet("color: #cccccc; font-size: 13px; line-height: 1.4;")
        layout.addWidget(text_label)
        
        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(0, 0, 0, 0)
        main_layout.addWidget(self.container)
        
        self.duration = duration
        self._animation = None
        
        # Position at bottom-right of parent
        if parent:
            geo = parent.geometry()
            self.move(geo.right() - 380, geo.bottom() - 80)
        else:
            screen = QApplication.primaryScreen().geometry()
            self.move(screen.width() - 380, screen.height() - 100)
    
    def show_animated(self):
        """Show with slide-up + fade-in animation"""
        self.setWindowOpacity(0)
        self.show()
        
        # Start slightly below target position
        start_pos = self.pos() + QPoint(0, 30)
        self.move(start_pos)
        
        # Opacity animation
        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(250)
        self.opacity_anim.setStartValue(0.0)
        self.opacity_anim.setEndValue(1.0)
        self.opacity_anim.setEasingCurve(QEasingCurve.Type.OutCubic)
        
        # Position animation
        self.pos_anim = QPropertyAnimation(self, b"pos")
        self.pos_anim.setDuration(300)
        self.pos_anim.setStartValue(start_pos)
        self.pos_anim.setEndValue(start_pos - QPoint(0, 30))
        self.pos_anim.setEasingCurve(QEasingCurve.Type.OutCubic)
        
        self.opacity_anim.start()
        self.pos_anim.start()
        
        # Auto-close
        QTimer.singleShot(self.duration, self.close_animated)
    
    def close_animated(self):
        """Close with slide-down + fade-out animation"""
        if hasattr(self, 'opacity_anim') and self.opacity_anim.state() == QPropertyAnimation.State.Running:
            self.opacity_anim.stop()
        if hasattr(self, 'pos_anim') and self.pos_anim.state() == QPropertyAnimation.State.Running:
            self.pos_anim.stop()
        
        # Fade out
        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(200)
        self.opacity_anim.setStartValue(self.windowOpacity())
        self.opacity_anim.setEndValue(0.0)
        self.opacity_anim.setEasingCurve(QEasingCurve.Type.InCubic)
        
        # Slide down
        self.pos_anim = QPropertyAnimation(self, b"pos")
        self.pos_anim.setDuration(200)
        self.pos_anim.setStartValue(self.pos())
        self.pos_anim.setEndValue(self.pos() + QPoint(0, 20))
        self.pos_anim.setEasingCurve(QEasingCurve.Type.InCubic)
        
        self.opacity_anim.finished.connect(self.close)
        self.opacity_anim.start()
        self.pos_anim.start()


class SweetProgressDialog(QDialog):
    """SweetAlert2-style progress dialog with spinner"""
    
    def __init__(self, title: str, text: str, parent=None):
        super().__init__(parent)
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint | Qt.WindowType.Dialog)
        self.setModal(True)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        self.setFixedSize(340, 180)
        
        self.container = QWidget(self)
        self.container.setFixedSize(320, 160)
        self.container.setStyleSheet("""
            QWidget {
                background-color: #2d2d2d;
                border-radius: 16px;
                border: 2px solid #3085d6;
            }
        """)
        
        layout = QVBoxLayout(self.container)
        layout.setContentsMargins(24, 24, 24, 24)
        layout.setSpacing(16)
        
        # Spinner (animated)
        self.spinner_label = QLabel("⏳")
        self.spinner_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.spinner_label.setStyleSheet("font-size: 48px;")
        layout.addWidget(self.spinner_label)
        
        # Title
        self.title_label = QLabel(title)
        self.title_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.title_label.setStyleSheet("color: #ffffff; font-size: 18px; font-weight: 600;")
        layout.addWidget(self.title_label)
        
        # Text
        self.text_label = QLabel(text)
        self.text_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.text_label.setWordWrap(True)
        self.text_label.setStyleSheet("color: #cccccc; font-size: 14px;")
        layout.addWidget(self.text_label)
        
        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(10, 10, 10, 10)
        main_layout.addWidget(self.container, alignment=Qt.AlignmentFlag.AlignCenter)
        
        self._rotation = 0
        self._spinner_timer = QTimer(self)
        self._spinner_timer.timeout.connect(self._rotate_spinner)
        self._spinner_timer.start(100)
        
        if parent:
            self.move(parent.geometry().center() - self.rect().center())
    
    def _rotate_spinner(self):
        self._rotation = (self._rotation + 30) % 360
        self.spinner_label.setText(chr(0x23F3 + (self._rotation // 30) % 8))  # Spinner chars
    
    def show(self):
        self.setWindowOpacity(0)
        super().show()
        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(200)
        self.opacity_anim.setStartValue(0.0)
        self.opacity_anim.setEndValue(1.0)
        self.opacity_anim.setEasingCurve(QEasingCurve.Type.OutCubic)
        self.opacity_anim.start()
    
    def close(self):
        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(150)
        self.opacity_anim.setStartValue(self.windowOpacity())
        self.opacity_anim.setEndValue(0.0)
        self.opacity_anim.setEasingCurve(QEasingCurve.Type.InCubic)
        self.opacity_anim.finished.connect(super().close)
        self.opacity_anim.start()
    
    def update_text(self, text: str):
        self.findChild(QLabel, "text_label").setText(text)

class TorrentDownloadThread(QThread):
    """Thread for handling torrent downloads without blocking GUI"""
    progress_update = pyqtSignal(str)
    download_complete = pyqtSignal(bool, str)

    def __init__(self, magnet_link: str, save_path: str = None, tags: List[str] = None, torrent_manager: TorrentManager = None, parent=None):
        super().__init__(parent)
        self.magnet_link = magnet_link
        self.save_path = save_path
        self.tags = tags or []
        self.torrent_manager = torrent_manager if torrent_manager is not None else TorrentManager()
        self.save_path = save_path
        self.tags = tags or []
        self.torrent_manager = torrent_manager if torrent_manager is not None else TorrentManager()

    def run(self):
        self.progress_update.emit("Connecting to qBittorrent...")
        if not self.torrent_manager.is_connected():
            self.download_complete.emit(False, "Failed to connect to qBittorrent. Make sure it's running.")
            return

        self.progress_update.emit("Adding torrent...")
        success = self.torrent_manager.add_torrent(
            self.magnet_link,
            save_path=self.save_path,
            tags=self.tags
        )

        if success:
            self.download_complete.emit(True, "Torrent added successfully!")
        else:
            self.download_complete.emit(False, "Failed to add torrent to qBittorrent.")

class AnimeSearchThread(QThread):
    """Thread for searching anime on nyaa.si without blocking GUI"""
    search_results = pyqtSignal(list)
    search_error = pyqtSignal(str)
    search_status = pyqtSignal(str)

    def __init__(self, anime_name: str, parent=None):
        super().__init__(parent)
        self.anime_name = anime_name
        self.scraper = NyaaScraper()
        self.logger = logging.getLogger(__name__)

    def run(self):
        self.search_status.emit(f"Searching for '{self.anime_name}' on nyaa.si...")
        try:
            results = self.scraper.get_torrents_for_anime(self.anime_name, limit=100)
            self.search_results.emit(results)
        except Exception as e:
            self.search_error.emit(str(e))

class IconUpdateThread(QThread):
    """Thread for running the icon manager script"""
    update_progress = pyqtSignal(str)
    update_complete = pyqtSignal(bool, str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.logger = logging.getLogger(__name__)

    def run(self):
        self.update_progress.emit("Starting icon update process...")
        try:
            import subprocess
            import os
            # Get the directory where this script is located
            script_dir = os.path.dirname(os.path.abspath(__file__))
            script_path = os.path.join(script_dir, "anime_icon_manager.py")
            result = subprocess.run(
                [sys.executable, script_path],
                capture_output=True,
                text=True,
                timeout=300  # 5 minute timeout
            )

            if result.returncode == 0:
                self.update_complete.emit(True, "Icon update completed successfully!")
                self.logger.debug(f"Icon update output: {result.stdout}")
            else:
                self.update_complete.emit(False, f"Icon update failed: {result.stderr}")
                self.logger.error(f"Icon update error: {result.stderr}")

        except subprocess.TimeoutExpired:
            self.update_complete.emit(False, "Icon update timed out after 5 minutes")
        except Exception as e:
            self.update_complete.emit(False, f"Error running icon update: {str(e)}")
            self.logger.error(f"Icon update exception: {e}")


class SmartDownloadThread(QThread):
    """Thread for smart downloading - detects seasons and downloads all episodes"""
    progress_update = pyqtSignal(str)
    download_complete = pyqtSignal(bool, str)
    season_found = pyqtSignal(str, int, list)  # anime_name, season_num, torrents

    def __init__(self, anime_name: str, save_path: str = None, max_seasons: int = 10, torrent_manager: TorrentManager = None, parent=None):
        super().__init__(parent)
        self.anime_name = anime_name
        self.save_path = save_path
        self.max_seasons = max_seasons
        self.scraper = NyaaScraper()
        self.torrent_manager = torrent_manager if torrent_manager is not None else TorrentManager()
        self.logger = logging.getLogger(__name__)

    def run(self):
        self.progress_update.emit(f"Starting smart download for: {self.anime_name}")
        
        # Check qBittorrent connection
        if not self.torrent_manager.is_connected():
            self.download_complete.emit(False, "Failed to connect to qBittorrent. Make sure it's running.")
            return
        
        # Detect seasons by searching for each season
        all_torrents_by_season = {}
        total_seasons_found = 0
        
        for season in range(1, self.max_seasons + 1):
            self.progress_update.emit(f"Searching for Season {season}...")
            season_torrents = self._search_season(self.anime_name, season)
            
            if season_torrents:
                all_torrents_by_season[season] = season_torrents
                total_seasons_found += 1
                self.season_found.emit(self.anime_name, season, season_torrents)
                self.progress_update.emit(f"Found {len(season_torrents)} torrents for Season {season}")
            else:
                self.progress_update.emit(f"No torrents found for Season {season}, stopping search")
                break  # No more seasons found
            
            # Small delay to be respectful to the server
            import time
            time.sleep(0.5)
        
        if total_seasons_found == 0:
            self.download_complete.emit(False, f"No torrents found for {self.anime_name}")
            return
        
        # Create folder for the anime
        import os
        safe_anime_name = self._sanitize_filename(self.anime_name)
        anime_folder = os.path.join(self.save_path or r"D:\Anime", safe_anime_name)
        os.makedirs(anime_folder, exist_ok=True)
        
        # Download each season sequentially (limit to top 3 torrents per season to avoid overload)
        total_downloaded = 0
        max_torrents_per_season = 3
        for season in range(1, total_seasons_found + 1):
            torrents = all_torrents_by_season[season]
            self.progress_update.emit(f"Downloading Season {season} ({len(torrents)} torrents found)...")

            # Sort by seeders and download best ones
            torrents.sort(key=lambda x: x['seeders'], reverse=True)

            # Limit to top torrents per season to avoid overwhelming qBittorrent
            torrents_to_download = torrents[:max_torrents_per_season]
            if len(torrents) > max_torrents_per_season:
                self.progress_update.emit(f"Limiting to top {max_torrents_per_season} torrents for Season {season} (sorted by seeders)")

            # Download selected torrents for this season
            successful_in_season = 0
            for i, torrent in enumerate(torrents_to_download):
                self.progress_update.emit(f"Season {season} - Downloading {i+1}/{len(torrents_to_download)}: {torrent['title'][:50]}...")
                try:
                    success = self.torrent_manager.add_torrent(
                        torrent['magnet'],
                        save_path=anime_folder,
                        tags=[f"Season {season}", self.anime_name]
                    )
                    if success:
                        total_downloaded += 1
                        successful_in_season += 1
                        self.progress_update.emit(f"✓ Successfully added torrent for Season {season}")
                        # Wait a bit between downloads
                        time.sleep(2)
                    else:
                        self.progress_update.emit(f"✗ Failed to add torrent for Season {season} (qBittorrent returned False)")
                        self.logger.warning(f"Failed to add torrent: {torrent['title']}")
                except Exception as e:
                    self.progress_update.emit(f"✗ Error adding torrent for Season {season}: {str(e)}")
                    self.logger.error(f"Exception adding torrent {torrent['title']}: {e}")

            self.progress_update.emit(f"Season {season} completed! ({successful_in_season}/{len(torrents_to_download)} torrents added)")
        
        # After all downloads, trigger icon update for high-quality folder icons
        self.progress_update.emit("Downloading high-quality folder icon...")
        self._download_high_quality_icon(anime_folder, safe_anime_name)

        success = total_downloaded > 0
        if success:
            self.download_complete.emit(True, f"Smart download complete! Downloaded {total_downloaded} torrents across {total_seasons_found} seasons to {anime_folder}")
        else:
            self.download_complete.emit(False, f"Smart download completed but no torrents were successfully added for {self.anime_name}")

    def _extract_episode_number(self, title: str) -> Optional[int]:
        """Extract episode number from torrent title"""
        # Common patterns for episode numbers
        patterns = [
            r'[._\- ]\[?(\d{1,3})\]?[._\- ]',  # [01], .01., _01-, etc.
            r'[._\- ]E?(\d{1,3})[._\- ]',      # E01, E1, 01, 1
            r'[Ss]\s*\d{1,2}[._\- ]E?(\d{1,3})',  # S01E01, S1E1
            r'[._\- ](\d{1,3})[ sq]',          # 01 [, 01 (
            r'[\[\[](\d{1,3})[\]\)]',          # [01], (01)
            r'[\[\(](\d{1,3})[\]\)]',          # [01], (01) - alternative
            r'(\d{1,3})(?=\s*[ of])',          # 01 followed by space and "of" or " "
            r'[第](\d{1,3})[话集]',             # Japanese: 第01话, 第01集
        ]

        for pattern in patterns:
            match = re.search(pattern, title, re.IGNORECASE)
            if match:
                try:
                    episode_num = int(match.group(1))
                    if 1 <= episode_num <= 100:  # Reasonable episode range
                        return episode_num
                except ValueError:
                    continue
        return None

    def _search_season(self, anime_name: str, season: int) -> list:
        """Search for torrents for a specific season"""
        season_queries = [
            f"{anime_name} Season {season}",
            f"{anime_name} S{season:02d}",
            f"{anime_name} S{season}",
            f"{anime_name} {season}期",  # Japanese format
            f"{anime_name} 第{season}期",
        ]

        all_results = []
        for query in season_queries:
            results = self.scraper.get_torrents_for_anime(query, limit=50)
            all_results.extend(results)
            if len(all_results) >= 50:
                break

        # Remove exact duplicates by magnet link first
        seen_magnets = set()
        magnet_unique_results = []
        for result in all_results:
            if result['magnet'] not in seen_magnets:
                seen_magnets.add(result['magnet'])
                magnet_unique_results.append(result)

        # Group by episode number and keep best torrent for each episode
        episodes = {}
        for result in magnet_unique_results:
            try:
                episode_num = self._extract_episode_number(result['title'])
            except Exception:
                # If episode extraction fails, treat as unique
                episode_num = None

            # If we can't extract episode number, treat as unique (could be special episode, OVA, etc.)
            if episode_num is None:
                # Use a special key for non-episode results
                key = f"no_episode_{len(episodes)}"
                if key not in episodes or result['seeders'] > episodes[key]['seeders']:
                    episodes[key] = result
            else:
                # Group by episode number, keep the one with most seeders
                if episode_num not in episodes or result['seeders'] > episodes[episode_num]['seeders']:
                    episodes[episode_num] = result

        # Return the best torrent for each episode
        return list(episodes.values())

    def _sanitize_filename(self, name: str) -> str:
        import re
        return re.sub(r'[<>:"/\\|?*]', '_', name)

    def _download_high_quality_icon(self, folder_path: str, anime_name: str):
        """Download high-quality icon for the anime folder"""
        try:
            import requests
            from PIL import Image
            import io
            import re
            
            # Search AniList for the anime
            ANILIST_URL = "https://graphql.anilist.co"
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
              }
            }
            """
            
            clean_name = re.sub(r'[\[\(].*?[\]\)]', '', anime_name).strip()
            clean_name = re.sub(r'\b(1080p|720p|480p|HEVC|x265|x264|BD|WEB|DUAL|AUDIO|SUB)\b', '', clean_name, flags=re.IGNORECASE).strip()
            
            variables = {"search": clean_name}
            response = requests.post(ANILIST_URL, json={"query": SEARCH_QUERY, "variables": variables}, timeout=10)
            data = response.json()
            
            if "data" in data and data["data"] and data["data"]["Media"]:
                anime_data = data["data"]["Media"]
                cover_url = anime_data['coverImage']['large']
                
                # Download the high-quality cover image
                img_response = requests.get(cover_url, timeout=10)
                img_response.raise_for_status()
                
                # Save as PNG
                png_path = os.path.join(folder_path, f"{self._sanitize_filename(anime_data['title']['romaji'])}.png")
                with open(png_path, 'wb') as f:
                    f.write(img_response.content)
                
                # Convert to ICO with multiple sizes for high quality
                ico_path = os.path.join(folder_path, f"{self._sanitize_filename(anime_data['title']['romaji'])}.ico")
                img = Image.open(png_path)
                # Use many sizes for high quality including 256x256
                img.save(ico_path, format='ICO', sizes=[(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)])
                
                # Set folder icon
                self._set_folder_icon(folder_path, ico_path)
                
                # Save synopsis
                self._save_synopsis(folder_path, anime_data)
                
                self.progress_update.emit("High-quality folder icon set!")
        except Exception as e:
            self.progress_update.emit(f"Could not set folder icon: {e}")
            self.logger.error(f"Icon download error: {e}")

    def _set_folder_icon(self, folder_path: str, icon_path: str):
        import os
        desktop_ini = os.path.join(folder_path, "desktop.ini")
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
        except Exception as e:
            self.logger.error(f"Failed to set folder icon: {e}")

    def _save_synopsis(self, folder_path: str, anime_data: dict):
        import re
        synopsis_path = os.path.join(folder_path, "synopsis.txt")
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
        except Exception as e:
            self.logger.error(f"Failed to save synopsis: {e}")

class GeneralDownloaderMainWindow(QMainWindow):
    # Signal for thread-safe GUI updates from browser monitor
    anime_tab_detected = pyqtSignal(object)

    def __init__(self):
        super().__init__()
        self.setWindowTitle("General Downloader")
        self.setGeometry(100, 100, 1200, 800)

        # Initialize components
        self.nyaa_scraper = NyaaScraper()
        self.torrent_manager = TorrentManager()
        self.browser_monitor = BrowserMonitor(check_interval=3)
        # Connect anime tab detection callback (thread-safe via signal)
        self.anime_tab_detected.connect(self.on_anime_tab_detected)
        self.browser_monitor.add_callback(self._on_anime_tab_detected_threaded)
        self.current_search_results = []

        # Auto-download queue to prevent overload
        self.auto_download_queue = []
        self.auto_download_processing = False
        self.auto_download_delay = 10  # Seconds between auto-downloads

        # Track active threads to prevent QThread destroyed while running errors
        self.active_threads = []

        # Set parent for QThreads to ensure proper cleanup
        self.setAttribute(Qt.WidgetAttribute.WA_DeleteOnClose)

        # Set up UI
        self.setup_ui()
        self.setup_menu()
        self.setup_status_bar()
        self.setup_system_tray()

        # Start background services
        self.start_background_services()

        # Set up timers
        self.setup_timers()

        # Initialize logger for main window
        self.logger = logging.getLogger(__name__)
        self.logger.info("General Downloader initialized")

    def _remove_thread(self, thread):
        """Remove thread from active tracking list"""
        if thread in self.active_threads:
            self.active_threads.remove(thread)

    def setup_ui(self):
        """Set up the main user interface"""
        central_widget = QWidget()
        self.setCentralWidget(central_widget)

        # Main layout
        main_layout = QVBoxLayout(central_widget)

        # Create tab widget
        self.tab_widget = QTabWidget()
        main_layout.addWidget(self.tab_widget)

        # Create tabs
        self.create_search_tab()
        self.create_downloads_tab()
        self.create_browser_tab()
        self.create_settings_tab()

    def create_search_tab(self):
        """Create the anime search tab"""
        search_tab = QWidget()
        layout = QVBoxLayout(search_tab)

        # Search section
        search_group = QGroupBox("Search Anime on Nyaa.si")
        search_layout = QVBoxLayout(search_group)

        # Search input
        search_input_layout = QHBoxLayout()
        self.search_input = QLineEdit()
        self.search_input.setPlaceholderText("Enter anime name to search...")
        self.search_input.returnPressed.connect(self.search_anime)
        self.search_button = QPushButton("Search")
        self.search_button.clicked.connect(self.search_anime)
        search_input_layout.addWidget(QLabel("Anime Name:"))
        search_input_layout.addWidget(self.search_input)
        search_input_layout.addWidget(self.search_button)
        search_layout.addLayout(search_input_layout)

        # Search status
        self.search_status_label = QLabel("Ready to search")
        search_layout.addWidget(self.search_status_label)

        # Progress bar for search
        self.search_progress = QProgressBar()
        self.search_progress.setVisible(False)
        search_layout.addWidget(self.search_progress)

        layout.addWidget(search_group)

        # Results section
        results_group = QGroupBox("Search Results")
        results_layout = QVBoxLayout(results_group)

        # Results table
        self.results_table = QTableWidget()
        self.results_table.setColumnCount(6)
        self.results_table.setHorizontalHeaderLabels([
            "Title", "Size", "Seeders", "Leechers", "Date", "Actions"
        ])
        self.results_table.horizontalHeader().setStretchLastSection(True)
        self.results_table.horizontalHeader().setSectionResizeMode(0, QHeaderView.ResizeMode.Stretch)
        self.results_table.setSelectionBehavior(QTableWidget.SelectionBehavior.SelectRows)
        self.results_table.setAlternatingRowColors(True)

        results_layout.addWidget(self.results_table)

        # Selected torrent info
        self.selected_info = QTextEdit()
        self.selected_info.setMaximumHeight(100)
        self.selected_info.setReadOnly(True)
        results_layout.addWidget(QLabel("Selected Torrent Info:"))
        results_layout.addWidget(self.selected_info)

        # Download button
        self.download_button = QPushButton("Download Selected Torrent")
        self.download_button.setEnabled(False)
        self.download_button.clicked.connect(self.download_selected_torrent)
        results_layout.addWidget(self.download_button)

        # Smart download button (downloads all seasons)
        self.smart_download_button = QPushButton("Smart Download All Seasons")
        self.smart_download_button.setEnabled(False)
        self.smart_download_button.setStyleSheet("background-color: #4CAF50; color: white; font-weight: bold; padding: 8px;")
        self.smart_download_button.clicked.connect(self.smart_download_selected_anime)
        results_layout.addWidget(self.smart_download_button)

        layout.addWidget(results_group)

        # Connect table selection
        self.results_table.itemSelectionChanged.connect(self.on_result_selected)

        self.tab_widget.addTab(search_tab, "Search & Download")

    def create_downloads_tab(self):
        """Create the torrents download monitoring tab"""
        downloads_tab = QWidget()
        layout = QVBoxLayout(downloads_tab)

        # Torrent status section
        torrent_group = QGroupBox("Active Torrents")
        torrent_layout = QVBoxLayout(torrent_group)

        # Torrents table
        self.torrents_table = QTableWidget()
        self.torrents_table.setColumnCount(8)
        self.torrents_table.setHorizontalHeaderLabels([
            "Name", "Progress", "Download Speed", "Upload Speed",
            "ETA", "Ratio", "Seeds", "Peers"
        ])
        self.torrents_table.horizontalHeader().setStretchLastSection(True)
        self.torrents_table.setAlternatingRowColors(True)

        torrent_layout.addWidget(self.torrents_table)

        # Torrent controls
        torrent_controls = QHBoxLayout()
        self.refresh_torrents_button = QPushButton("Refresh Torrents")
        self.refresh_torrents_button.clicked.connect(self.refresh_torrents)
        self.pause_torrent_button = QPushButton("Pause Selected")
        self.pause_torrent_button.clicked.connect(self.pause_selected_torrent)
        self.resume_torrent_button = QPushButton("Resume Selected")
        self.resume_torrent_button.clicked.connect(self.resume_selected_torrent)
        self.remove_torrent_button = QPushButton("Remove Selected")
        self.remove_torrent_button.clicked.connect(self.remove_selected_torrent)

        torrent_controls.addWidget(self.refresh_torrents_button)
        torrent_controls.addWidget(self.pause_torrent_button)
        torrent_controls.addWidget(self.resume_torrent_button)
        torrent_controls.addWidget(self.remove_torrent_button)
        torrent_controls.addStretch()

        torrent_layout.addLayout(torrent_controls)
        layout.addWidget(torrent_group)

        # Manual magnet input
        manual_group = QGroupBox("Add Torrent Manually")
        manual_layout = QVBoxLayout(manual_group)

        manual_input_layout = QHBoxLayout()
        self.manual_magnet_input = QLineEdit()
        self.manual_magnet_input.setPlaceholderText("Paste magnet link here...")
        self.add_manual_torrent_button = QPushButton("Add Torrent")
        self.add_manual_torrent_button.clicked.connect(self.add_manual_torrent)
        manual_input_layout.addWidget(QLabel("Magnet Link:"))
        manual_input_layout.addWidget(self.manual_magnet_input)
        manual_input_layout.addWidget(self.add_manual_torrent_button)
        manual_layout.addLayout(manual_input_layout)

        layout.addWidget(manual_group)

        self.tab_widget.addTab(downloads_tab, "Torrents")

    def create_browser_tab(self):
        """Create the browser monitoring tab"""
        browser_tab = QWidget()
        layout = QVBoxLayout(browser_tab)

        # Browser monitor section
        monitor_group = QGroupBox("Browser Tab Monitor")
        monitor_layout = QVBoxLayout(monitor_group)

        # Status
        self.browser_status_label = QLabel("Browser monitor: Stopped")
        monitor_layout.addWidget(self.browser_status_label)

        # Controls
        monitor_controls = QHBoxLayout()
        self.start_monitor_button = QPushButton("Start Monitoring")
        self.start_monitor_button.clicked.connect(self.start_browser_monitoring)
        self.stop_monitor_button = QPushButton("Stop Monitoring")
        self.stop_monitor_button.clicked.connect(self.stop_browser_monitoring)
        self.stop_monitor_button.setEnabled(False)
        monitor_controls.addWidget(self.start_monitor_button)
        monitor_controls.addWidget(self.stop_monitor_button)
        monitor_controls.addStretch()
        monitor_layout.addLayout(monitor_controls)

        # Detected tabs list
        tabs_group = QGroupBox("Detected Anime Tabs")
        tabs_layout = QVBoxLayout(tabs_group)

        self.detected_tabs_list = QTextEdit()
        self.detected_tabs_list.setReadOnly(True)
        tabs_layout.addWidget(self.detected_tabs_list)

        # Smart download button for selected detected anime
        smart_dl_layout = QHBoxLayout()
        self.smart_download_detected_btn = QPushButton("Smart Download All Seasons")
        self.smart_download_detected_btn.setEnabled(False)
        self.smart_download_detected_btn.setStyleSheet("background-color: #4CAF50; color: white; font-weight: bold; padding: 8px;")
        self.smart_download_detected_btn.clicked.connect(self.smart_download_detected_anime)
        smart_dl_layout.addWidget(self.smart_download_detected_btn)
        smart_dl_layout.addStretch()
        tabs_layout.addLayout(smart_dl_layout)

        layout.addWidget(monitor_group)
        layout.addWidget(tabs_group)

        # Auto-actions
        actions_group = QGroupBox("Automatic Actions")
        actions_layout = QVBoxLayout(actions_group)

        self.auto_search_checkbox = QCheckBox("Automatically search for new anime tabs")
        self.auto_search_checkbox.setChecked(True)
        actions_layout.addWidget(self.auto_search_checkbox)

        self.auto_download_checkbox = QCheckBox("Automatically download best torrent (highest seeders)")
        self.auto_download_checkbox.setChecked(True)  # Enabled by default for automatic downloading
        actions_layout.addWidget(self.auto_download_checkbox)

        layout.addWidget(actions_group)

        self.tab_widget.addTab(browser_tab, "Browser Monitor")

    def create_settings_tab(self):
        """Create the settings tab"""
        settings_tab = QWidget()
        layout = QVBoxLayout(settings_tab)

        # qBittorrent settings
        qb_group = QGroupBox("qBittorrent Settings")
        qb_layout = QFormLayout(qb_group)

        self.qb_host_input = QLineEdit("localhost")
        self.qb_port_input = QSpinBox()
        self.qb_port_input.setRange(1, 65535)
        self.qb_port_input.setValue(8080)
        self.qb_username_input = QLineEdit("admin")
        self.qb_password_input = QLineEdit()
        self.qb_password_input.setEchoMode(QLineEdit.EchoMode.Password)
        self.qb_password_input.setText("adminadmin")  # Default, should be changed

        qb_layout.addRow("Host:", self.qb_host_input)
        qb_layout.addRow("Port:", self.qb_port_input)
        qb_layout.addRow("Username:", self.qb_username_input)
        qb_layout.addRow("Password:", self.qb_password_input)

        qb_test_button = QPushButton("Test Connection")
        qb_test_button.clicked.connect(self.test_qbittorrent_connection)
        qb_layout.addRow(qb_test_button)

        layout.addWidget(qb_group)

        # Scheduler settings
        sched_group = QGroupBox("Scheduler Settings")
        sched_layout = QVBoxLayout(sched_group)

        # Daily icon update
        icon_update_layout = QHBoxLayout()
        self.icon_update_enabled = QCheckBox("Enable daily 10 PM icon update")
        self.icon_update_enabled.setChecked(True)
        icon_update_layout.addWidget(self.icon_update_enabled)
        icon_update_layout.addStretch()
        sched_layout.addLayout(icon_update_layout)

        # Browser tab checking
        tab_check_layout = QHBoxLayout()
        self.tab_check_enabled = QCheckBox("Enable periodic browser tab checking")
        self.tab_check_enabled.setChecked(True)
        tab_check_layout.addWidget(self.tab_check_enabled)
        tab_check_layout.addStretch()
        sched_layout.addLayout(tab_check_layout)

        layout.addWidget(sched_group)

        # Folder settings
        folder_group = QGroupBox("Folder Settings")
        folder_layout = QFormLayout(folder_group)

        self.anime_folder_input = QLineEdit(r"D:\\")
        folder_browse_button = QPushButton("Browse...")
        folder_browse_button.clicked.connect(self.browse_anime_folder)
        folder_input_layout = QHBoxLayout()
        folder_input_layout.addWidget(self.anime_folder_input)
        folder_input_layout.addWidget(folder_browse_button)
        folder_layout.addRow("Anime Root Folder:", folder_input_layout)

        self.icons_folder_input = QLineEdit(r"D:\\icons")
        icons_browse_button = QPushButton("Browse...")
        icons_browse_button.clicked.connect(self.browse_icons_folder)
        icons_input_layout = QHBoxLayout()
        icons_input_layout.addWidget(self.icons_folder_input)
        icons_input_layout.addWidget(icons_browse_button)
        folder_layout.addRow("Icons Folder:", icons_input_layout)

        layout.addWidget(folder_group)

        # Apply settings button
        apply_button = QPushButton("Apply Settings")
        apply_button.clicked.connect(self.apply_settings)
        layout.addWidget(apply_button)

        layout.addStretch()

        self.tab_widget.addTab(settings_tab, "Settings")

    def on_anime_tab_detected(self, tab: AnimeTabInfo):
        """Callback for when an anime tab is detected - thread-safe version"""
        from PyQt6.QtCore import QThread, QMetaObject, Qt, Q_ARG

        # Check if we're on the GUI thread
        if QThread.currentThread() != self.thread():
            # We're not on the GUI thread, invoke on GUI thread using queued connection
            QMetaObject.invokeMethod(
                self,
                "_on_anime_tab_detected_gui",
                Qt.ConnectionType.QueuedConnection,
                Q_ARG(AnimeTabInfo, tab)
            )
        else:
            # We're already on the GUI thread
            self._on_anime_tab_detected_gui(tab)

    def _on_anime_tab_detected_gui(self, tab: AnimeTabInfo):
        """Actual GUI update method for anime tab detection - runs on GUI thread"""
        try:
            from datetime import datetime

            # Format the detection information with episode/season info
            detection_text = f"Title: {tab.title}\n"
            detection_text += f"Anime: {tab.anime_name or 'Unknown'}\n"
            detection_text += f"Process: {tab.process_name}\n"
            detection_text += f"Confidence: {tab.confidence:.2f}\n"
            if tab.total_episodes:
                detection_text += f"Episodes: {tab.total_episodes}\n"
            if tab.total_seasons:
                detection_text += f"Estimated Seasons: {tab.total_seasons}\n"
            detection_text += f"Time: {tab.timestamp.strftime('%H:%M:%S')}\n"
            detection_text += "-" * 50 + "\n"

            # Append to the detected tabs list
            self.detected_tabs_list.insertPlainText(detection_text)

            # Auto-scroll to the bottom
            cursor = self.detected_tabs_list.textCursor()
            cursor.movePosition(cursor.MoveOperation.End)
            self.detected_tabs_list.setTextCursor(cursor)

            # Log to console as well
            print(f"[DETECTED] {tab.title} -> {tab.anime_name or 'Unknown'} (confidence: {tab.confidence:.2f})")

            # Store the last detected anime for smart download button
            if tab.anime_name and tab.anime_name != 'Unknown':
                # Only enable smart download button for tabs that are anime content pages (have anime ID in URL)
                import re
                if tab.url and re.search(r'myanimelist\.net/anime/\d+/', tab.url):
                    self.last_detected_anime = tab.anime_name
                    if hasattr(self, 'smart_download_detected_btn'):
                        self.smart_download_detected_btn.setEnabled(True)
                    # Show toast for detected anime
                    SweetAlert.toast("Anime Tab Detected", 
                        f"{tab.anime_name}\nEpisodes: {tab.total_episodes or '?'}\nSeasons: {tab.total_seasons or '?'}", 
                        self, 6000)

            # Handle auto-search if enabled
            if hasattr(self, 'auto_search_checkbox') and self.auto_search_checkbox.isChecked():
                if tab.anime_name and tab.anime_name != 'Unknown':
                    # Only trigger auto-search for tabs that are anime content pages (have anime ID in URL)
                    import re
                    if tab.url and re.search(r'myanimelist\.net/anime/\d+/', tab.url):
                        self.auto_search_anime(tab.anime_name)

            # Handle auto-download if enabled
            if hasattr(self, 'auto_download_checkbox') and self.auto_download_checkbox.isChecked():
                if tab.anime_name and tab.anime_name != 'Unknown':
                    # Only trigger auto-download for tabs that are anime content pages (have anime ID in URL)
                    import re
                    if tab.url and re.search(r'myanimelist\.net/anime/\d+/', tab.url):
                        self.auto_download_anime(tab.anime_name)
        except Exception as e:
            import traceback
            print(f"[ERROR] _on_anime_tab_detected_gui: {e}")
            traceback.print_exc()

    def auto_search_anime(self, anime_name: str):
        """Automatically search for an anime when detected in browser"""
        self.search_input.setText(anime_name)
        self.search_anime()

    def _detect_season_from_title(self, title: str) -> Optional[int]:
        """Detect season number from torrent title"""
        import re
        # Look for patterns like "Season 1", "S01", "[01]", etc.
        patterns = [
            r'[Ss]eason\s*(\d+)',
            r'[Ss]\s*(\d{1,2})',
            r'[\[\(](\d{1,2})[\]\)]',
            r'[._\- ](\d{1,2})[._\- ]',
        ]

        for pattern in patterns:
            match = re.search(pattern, title)
            if match:
                try:
                    season_num = int(match.group(1))
                    if 1 <= season_num <= 10:  # Reasonable season range
                        return season_num
                except ValueError:
                    continue
        return None

    def auto_download_anime(self, anime_name: str):
        """Queue anime for auto-download to prevent overload"""
        if anime_name in self.auto_download_queue:
            self.logger.debug(f"Anime already in queue: {anime_name}")
            return

        self.auto_download_queue.append(anime_name)
        self.status_label.setText(f"Queued for download: {anime_name} (queue: {len(self.auto_download_queue)})")
        self.logger.info(f"Queued anime for download: {anime_name} (queue: {len(self.auto_download_queue)})")
        SweetAlert.toast("Queued for Download", f"{anime_name}\nQueue position: {len(self.auto_download_queue)}", self, 5000)

        if not self.auto_download_processing:
            self._process_auto_download_queue()

    def _process_auto_download_queue(self):
        """Process the auto-download queue with delays"""
        if not self.auto_download_queue:
            self.auto_download_processing = False
            return

        self.auto_download_processing = True
        anime_name = self.auto_download_queue.pop(0)

        self.status_label.setText(f"Starting download for: {anime_name} (remaining: {len(self.auto_download_queue)})")
        self.logger.info(f"Auto-downloading: {anime_name}")

        # Get save path from settings
        save_path = self.anime_folder_input.text().strip() if hasattr(self, 'anime_folder_input') else None

        try:
            # Start smart download thread
            self.smart_download_thread = SmartDownloadThread(anime_name, save_path=save_path, max_seasons=10, parent=self, torrent_manager=self.torrent_manager)
            self.smart_download_thread.progress_update.connect(self.on_download_progress)
            self.smart_download_thread.download_complete.connect(self._on_auto_download_complete)
            self.smart_download_thread.season_found.connect(self.on_season_found)
            self.smart_download_thread.finished.connect(lambda: self._remove_thread(self.smart_download_thread))
            self.active_threads.append(self.smart_download_thread)
            self.smart_download_thread.start()

            # Show popup notification
            SweetAlert.success("Auto Download Started", 
                f"Started downloading all seasons for:\n{anime_name}\n\n"
                f"Check the Torrents tab for progress.", self)

            self.search_status_label.setText(f"Smart downloading {anime_name} - all seasons...")
        except Exception as e:
            self.logger.error(f"Failed to start smart download thread for {anime_name}: {e}")
            # If we fail to start the thread, we should still try to process the next item
            self.auto_download_processing = False
            # Schedule a retry after a short delay to avoid busy loop
            from PyQt6.QtCore import QTimer
            QTimer.singleShot(2000, self._process_auto_download_queue)

    def _on_auto_download_complete(self, success: bool, message: str):
        """Handle auto-download completion and process next in queue"""
        try:
            if success:
                self.search_status_label.setText(f"✓ {message}")
                self.logger.info(f"Smart download successful: {message}")
                SweetAlert.toast("Download Complete", message, self, 5000)
            else:
                self.search_status_label.setText(f"✗ {message}")
                self.logger.warning(f"Smart download failed: {message}")
                SweetAlert.toast("Download Failed", message, self, 5000)

            # Refresh torrents to show the new additions
            self.refresh_torrents()

            # Process next in queue after delay (shorter delay for faster processing)
            if self.auto_download_queue:
                from PyQt6.QtCore import QTimer
                QTimer.singleShot(2000, self._process_auto_download_queue)  # Reduced to 2 seconds
            else:
                self.auto_download_processing = False
                self.status_label.setText("Auto-download queue completed")
                SweetAlert.toast("Queue Complete", "All auto-downloads finished", self, 4000)
        except Exception as e:
            self.logger.error(f"Error in _on_auto_download_complete: {e}")
            # Ensure we continue processing next item despite error
            if self.auto_download_queue:
                from PyQt6.QtCore import QTimer
                QTimer.singleShot(2000, self._process_auto_download_queue)
            else:
                self.auto_download_processing = False
                self.status_label.setText("Auto-download queue completed")

    def smart_download_detected_anime(self):
        """Smart download for the last detected anime from browser monitor - uses queue"""
        if hasattr(self, 'last_detected_anime') and self.last_detected_anime:
            self.auto_download_anime(self.last_detected_anime)
            SweetAlert.success("Smart Download Started", 
                f"Started downloading all seasons for:\n{self.last_detected_anime}\n\n"
                f"Check the Torrents tab for progress.", self)
        else:
            SweetAlert.warning("No Anime Selected", "No anime detected yet. Open a MyAnimeList page first.", self)

    def _on_anime_tab_detected_threaded(self, tab: AnimeTabInfo):
        """Thread-safe callback wrapper for browser monitor - emits signal for GUI thread"""
        self.anime_tab_detected.emit(tab)

    def setup_menu(self):
        """Set up the application menu bar"""
        menubar = self.menuBar()

        # File menu
        file_menu = menubar.addMenu("File")

        exit_action = QAction("Exit", self)
        exit_action.setShortcut("Ctrl+Q")
        exit_action.triggered.connect(self.close)
        file_menu.addAction(exit_action)

        # Tools menu
        tools_menu = menubar.addMenu("Tools")

        update_icons_action = QAction("Update Folder Icons", self)
        update_icons_action.triggered.connect(self.update_icons_manual)
        tools_menu.addAction(update_icons_action)

        check_torrents_action = QAction("Check Torrent Status", self)
        check_torrents_action.triggered.connect(self.refresh_torrents)
        tools_menu.addAction(check_torrents_action)

        # Help menu
        help_menu = menubar.addMenu("Help")

        about_action = QAction("About", self)
        about_action.triggered.connect(self.show_about)
        help_menu.addAction(about_action)

    def setup_status_bar(self):
        """Set up the status bar"""
        self.status_bar = QStatusBar()
        self.setStatusBar(self.status_bar)

        # Status labels
        self.status_label = QLabel("Ready")
        self.status_bar.addWidget(self.status_label)

        self.status_bar.addPermanentWidget(QLabel(" | "))

        self.qb_status_label = QLabel("qBittorrent: Disconnected")
        self.status_bar.addPermanentWidget(self.qb_status_label)

    def setup_system_tray(self):
        """Set up system tray icon"""
        self.tray_icon = QSystemTrayIcon(self)
        # Try to use an icon, fallback to default if not available
        try:
            self.tray_icon.setIcon(QIcon.fromTheme("application-x-executable"))
        except:
            pass  # Use default icon

        self.tray_icon.setToolTip("General Downloader")

        # Create tray menu
        tray_menu = []

        show_action = QAction("Show", self)
        show_action.triggered.connect(self.show)
        tray_menu.append(show_action)

        hide_action = QAction("Hide", self)
        hide_action.triggered.connect(self.hide)
        tray_menu.append(hide_action)

        tray_menu.append(None)  # Separator

        exit_action = QAction("Exit", self)
        exit_action.triggered.connect(QApplication.instance().quit)
        tray_menu.append(exit_action)

        # Set context menu
        from PyQt6.QtWidgets import QMenu
        tray_context_menu = QMenu()
        for action in tray_menu:
            if action is None:
                tray_context_menu.addSeparator()
            else:
                tray_context_menu.addAction(action)
        self.tray_icon.setContextMenu(tray_context_menu)

        self.tray_icon.show()

    def start_background_services(self):
        """Start background services"""
        # Start scheduler
        start_scheduler()

        # Schedule daily icon update if enabled
        if hasattr(self, 'icon_update_enabled') and self.icon_update_enabled.isChecked():
            schedule_daily_10pm_icon_update()

        # Start browser monitoring automatically (with delay to let UI show first)
        from PyQt6.QtCore import QTimer
        QTimer.singleShot(1000, self._start_browser_monitor_delayed)

        logger.info("Background services started")

    def _start_browser_monitor_delayed(self):
        """Start browser monitor after UI is shown"""
        try:
            self.browser_monitor.start_monitoring()
            self.start_monitor_button.setEnabled(False)
            self.stop_monitor_button.setEnabled(True)
            self.browser_status_label.setText("Browser monitor: Running")
            logger.info("Browser monitor started automatically")
        except Exception as e:
            logger.error(f"Failed to start browser monitor: {e}")
            import traceback
            traceback.print_exc()

    def setup_timers(self):
        """Set up periodic update timers"""
        # Timer to update torrent status
        self.torrent_timer = QTimer()
        self.torrent_timer.timeout.connect(self.refresh_torrents)
        self.torrent_timer.start(30000)  # Update every 30 seconds

        # Timer to update status bar
        self.status_timer = QTimer()
        self.status_timer.timeout.connect(self.update_status_bar)
        self.status_timer.start(5000)  # Update every 5 seconds

    def update_status_bar(self):
        """Update the status bar information"""
        # Update qBittorrent status
        if self.torrent_manager.is_connected():
            self.qb_status_label.setText("qBittorrent: Connected")
            self.qb_status_label.setStyleSheet("color: green")
        else:
            self.qb_status_label.setText("qBittorrent: Disconnected")
            self.qb_status_label.setStyleSheet("color: red")

        # Update general status
        current_time = QTime.currentTime().toString("hh:mm:ss")
        self.status_label.setText(f"Last updated: {current_time}")

    def search_anime(self):
        """Search for anime on nyaa.si"""
        anime_name = self.search_input.text().strip()
        if not anime_name:
            QMessageBox.warning(self, "Input Error", "Please enter an anime name to search.")
            return

        # Disable search button and show progress
        self.search_button.setEnabled(False)
        self.search_progress.setVisible(True)
        self.search_progress.setRange(0, 0)  # Indeterminate progress
        self.search_status_label.setText(f"Searching for '{anime_name}'...")

        # Clear previous results
        self.results_table.setRowCount(0)
        self.selected_info.clear()
        self.download_button.setEnabled(False)

        # Start search thread
        self.search_thread = AnimeSearchThread(anime_name, parent=self)
        self.search_thread.search_results.connect(self.on_search_results)
        self.search_thread.search_error.connect(self.on_search_error)
        self.search_thread.search_status.connect(self.on_search_status)
        self.search_thread.finished.connect(lambda: self._remove_thread(self.search_thread))
        self.active_threads.append(self.search_thread)
        self.search_thread.start()

    def on_search_results(self, results: List[Dict]):
        """Handle search results"""
        self.search_button.setEnabled(True)
        self.search_progress.setVisible(False)
        self.current_search_results = results

        self.results_table.setRowCount(len(results))

        for row, torrent in enumerate(results):
            # Title
            title_item = QTableWidgetItem(torrent['title'])
            self.results_table.setItem(row, 0, title_item)

            # Size
            size_item = QTableWidgetItem(torrent['size'])
            self.results_table.setItem(row, 1, size_item)

            # Seeders
            seeders_item = QTableWidgetItem(str(torrent['seeders']))
            seeders_item.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
            self.results_table.setItem(row, 2, seeders_item)

            # Leecher
            # Leecher
            leechers_item = QTableWidgetItem(str(torrent['leechers']))
            leechers_item.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
            self.results_table.setItem(row, 3, leechers_item)

            # Date
            date_item = QTableWidgetItem(torrent['date'])
            self.results_table.setItem(row, 4, date_item)

            # Actions (Download button)
            download_btn = QPushButton("Download")
            download_btn.clicked.connect(lambda checked, r=row: self.download_torrent_by_row(r))
            self.results_table.setCellWidget(row, 5, download_btn)

        self.search_status_label.setText(f"Found {len(results)} results")
        SweetAlert.toast("Search Complete", f"Found {len(results)} results for your search", self, 4000)

        # Handle auto-download if enabled and we have results
        if (hasattr(self, 'auto_download_checkbox') and self.auto_download_checkbox.isChecked()
            and hasattr(self, 'auto_downloading_anime') and results):
            # Auto-select and download the first result (highest seeder due to sorting)
            first_torrent = results[0]
            self.download_torrent(first_torrent['magnet'])
            # Clear the auto-downloading flag after use
            if hasattr(self, 'auto_downloading_anime'):
                delattr(self, 'auto_downloading_anime')

    def on_search_error(self, error: str):
        """Handle search error"""
        self.search_button.setEnabled(True)
        self.search_progress.setVisible(False)
        self.search_status_label.setText("Search failed")
        SweetAlert.error("Search Failed", f"Failed to search: {error}", self)

    def on_search_status(self, status: str):
        """Handle search status updates"""
        self.search_status_label.setText(status)

    def on_result_selected(self):
        """Handle result selection in the table"""
        selected_rows = self.results_table.selectionModel().selectedRows()
        if selected_rows:
            row = selected_rows[0].row()
            if 0 <= row < len(self.current_search_results):
                torrent = self.current_search_results[row]
                info_text = f"""Title: {torrent['title']}
Size: {torrent['size']}
Seeders: {torrent['seeders']}
Leechers: {torrent['leechers']}
Date: {torrent['date']}
Category: {torrent.get('category', 'N/A')}
Magnet: {torrent['magnet'][:100]}..."""
                self.selected_info.setPlainText(info_text)
                self.download_button.setEnabled(True)
                self.smart_download_button.setEnabled(True)
        else:
            self.selected_info.clear()
            self.download_button.setEnabled(False)
            self.smart_download_button.setEnabled(False)

    def smart_download_selected_anime(self):
        """Smart download all seasons for the selected anime"""
        selected_rows = self.results_table.selectionModel().selectedRows()
        if selected_rows:
            row = selected_rows[0].row()
            if 0 <= row < len(self.current_search_results):
                torrent = self.current_search_results[row]
                # Extract anime name from the title (first part before season/episode info)
                anime_name = self._extract_anime_name_from_torrent_title(torrent['title'])
                self.auto_download_anime(anime_name)

    def _extract_anime_name_from_torrent_title(self, title: str) -> str:
        """Extract anime name from torrent title"""
        import re
        # Remove common patterns
        clean = re.sub(r'\s*[\[\(].*?[\]\)]', '', title)  # Remove brackets content
        clean = re.sub(r'\b(1080p|720p|480p|HEVC|x265|x264|BD|WEB|DUAL|AUDIO|SUB|S\d+|Season\s*\d+)\b', '', clean, flags=re.IGNORECASE).strip()
        # Take first part before common separators
        for sep in [' - ', ' | ', ' : ', ' :: ']:
            if sep in clean:
                clean = clean.split(sep)[0].strip()
                break
        return clean.strip()

    def download_torrent_by_row(self, row: int):
        """Download torrent by row index"""
        if 0 <= row < len(self.current_search_results):
            torrent = self.current_search_results[row]
            self.download_torrent(torrent['magnet'])

    def download_selected_torrent(self):
        """Download the currently selected torrent"""
        selected_rows = self.results_table.selectionModel().selectedRows()
        if selected_rows:
            row = selected_rows[0].row()
            if 0 <= row < len(self.current_search_results):
                torrent = self.current_search_results[row]
                self.download_torrent(torrent['magnet'])

    def download_torrent(self, magnet_link: str):
        """Download a torrent using the magnet link"""
        self.download_button.setEnabled(False)
        self.download_button.setText("Downloading...")

        # Get save path from settings
        save_path = self.anime_folder_input.text().strip() if hasattr(self, 'anime_folder_input') else None

        # Start download thread
        self.download_thread = TorrentDownloadThread(magnet_link, save_path=save_path, parent=self, torrent_manager=self.torrent_manager)
        self.download_thread.progress_update.connect(self.on_download_progress)
        self.download_thread.download_complete.connect(self.on_download_complete)
        self.download_thread.finished.connect(lambda: self._remove_thread(self.download_thread))
        self.active_threads.append(self.download_thread)
        self.download_thread.start()

    def on_download_progress(self, message: str):
        """Handle download progress updates"""
        self.status_label.setText(message)

    def on_download_complete(self, success: bool, message: str):
        """Handle download completion"""
        self.download_button.setEnabled(True)
        self.download_button.setText("Download Selected Torrent")

        if success:
            SweetAlert.success("Download Started", message, self)
            self.search_status_label.setText(message)
        else:
            SweetAlert.error("Download Failed", message, self)
            self.search_status_label.setText("Download failed")

        # Refresh torrents to show the new addition
        self.refresh_torrents()

    def on_season_found(self, anime_name: str, season_num: int, torrents: list):
        """Handle when a season is found during smart download"""
        self.status_label.setText(f"Found Season {season_num} for {anime_name} ({len(torrents)} torrents)")

    def on_smart_download_complete(self, success: bool, message: str):
        """Handle smart download completion"""
        if success:
            SweetAlert.success("Smart Download Complete", message, self)
            self.search_status_label.setText(message)
        else:
            SweetAlert.error("Smart Download Failed", message, self)
            self.search_status_label.setText("Smart download failed")

        # Refresh torrents to show the new additions
        self.refresh_torrents()

    def add_manual_torrent(self):
        """Add a torrent manually from magnet link"""
        magnet_link = self.manual_magnet_input.text().strip()
        if not magnet_link:
            QMessageBox.warning(self, "Input Error", "Please enter a magnet link.")
            return

        if not magnet_link.startswith("magnet:"):
            reply = QMessageBox.question(
                self, "Confirm Magnet Link",
                "The link doesn't appear to be a magnet link. Continue anyway?",
                QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No
            )
            if reply == QMessageBox.StandardButton.No:
                return

        self.add_manual_torrent_button.setEnabled(False)
        self.add_manual_torrent_button.setText("Adding...")

        # Get save path from settings
        save_path = self.anime_folder_input.text().strip() if hasattr(self, 'anime_folder_input') else None

        # Start download thread
        self.download_thread = TorrentDownloadThread(magnet_link, save_path=save_path, parent=self, torrent_manager=self.torrent_manager)
        self.download_thread.download_complete.connect(self.on_manual_download_complete)
        self.download_thread.finished.connect(lambda: self._remove_thread(self.download_thread))
        self.active_threads.append(self.download_thread)
        self.download_thread.start()

    def on_manual_download_complete(self, success: bool, message: str):
        """Handle manual torrent download completion"""
        self.add_manual_torrent_button.setEnabled(True)
        self.add_manual_torrent_button.setText("Add Torrent")

        if success:
            SweetAlert.success("Torrent Added", message, self)
            self.manual_magnet_input.clear()
        else:
            SweetAlert.error("Failed to Add Torrent", message, self)

        self.refresh_torrents()

    def refresh_torrents(self):
        """Refresh the torrents table"""
        try:
            torrents = self.torrent_manager.get_torrents()
            self.torrents_table.setRowCount(len(torrents))

            for row, torrent in enumerate(torrents):
                # Name
                name_item = QTableWidgetItem(torrent['name'])
                self.torrents_table.setItem(row, 0, name_item)

                # Progress
                progress = torrent['progress'] * 100  # Convert to percentage
                progress_item = QTableWidgetItem(f"{progress:.1f}%")
                progress_item.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
                self.torrents_table.setItem(row, 1, progress_item)

                # Download speed
                dl_speed = self._format_speed(torrent['download_speed'])
                dl_item = QTableWidgetItem(dl_speed)
                dl_item.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
                self.torrents_table.setItem(row, 2, dl_item)

                # Upload speed
                ul_speed = self._format_speed(torrent['upload_speed'])
                ul_item = QTableWidgetItem(ul_speed)
                ul_item.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
                self.torrents_table.setItem(row, 3, ul_item)

                # ETA
                eta = torrent['eta']
                if eta < 0:
                    eta_str = "N/A"
                elif eta < 60:
                    eta_str = f"{eta}s"
                elif eta < 3600:
                    eta_str = f"{eta//60}m {eta%60}s"
                else:
                    eta_str = f"{eta//3600}h {(eta%3600)//60}m"
                eta_item = QTableWidgetItem(eta_str)
                eta_item.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
                self.torrents_table.setItem(row, 4, eta_item)

                # Ratio
                ratio_item = QTableWidgetItem(f"{torrent['ratio']:.2f}")
                ratio_item.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
                self.torrents_table.setItem(row, 5, ratio_item)

                # Seeds
                seeds_item = QTableWidgetItem(str(torrent['seeds']))
                seeds_item.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
                self.torrents_table.setItem(row, 6, seeds_item)

                # Peers
                peers_item = QTableWidgetItem(str(torrent['peers']))
                peers_item.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
                self.torrents_table.setItem(row, 7, peers_item)

        except Exception as e:
            logger.error(f"Error refreshing torrents: {e}")

    def _format_speed(self, speed_bytes: int) -> str:
        """Format speed in bytes per second to human readable format"""
        if speed_bytes < 1024:
            return f"{speed_bytes} B/s"
        elif speed_bytes < 1024 * 1024:
            return f"{speed_bytes/1024:.1f} KB/s"
        else:
            return f"{speed_bytes/(1024*1024):.1f} MB/s"

    def pause_selected_torrent(self):
        """Pause the selected torrent"""
        selected_rows = self.torrents_table.selectionModel().selectedRows()
        if selected_rows:
            row = selected_rows[0].row()
            # We would need to get the hash from the torrent data
            # For simplicity, we'll show a message that this needs implementation
            QMessageBox.information(self, "Not Implemented", "Torrent pause/resume functionality needs to be implemented with torrent hash tracking.")

    def resume_selected_torrent(self):
        """Resume the selected torrent"""
        selected_rows = self.torrents_table.selectionModel().selectedRows()
        if selected_rows:
            row = selected_rows[0].row()
            QMessageBox.information(self, "Not Implemented", "Torrent pause/resume functionality needs to be implemented with torrent hash tracking.")

    def remove_selected_torrent(self):
        """Remove the selected torrent"""
        selected_rows = self.torrents_table.selectionModel().selectedRows()
        if selected_rows:
            row = selected_rows[0].row()
            # We would need to get the hash from the torrent data
            QMessageBox.information(self, "Not Implemented", "Torrent removal functionality needs to be implemented with torrent hash tracking.")

    def start_browser_monitoring(self):
        """Start monitoring browser tabs"""
        self.browser_monitor.start_monitoring()
        self.start_monitor_button.setEnabled(False)
        self.stop_monitor_button.setEnabled(True)
        self.browser_status_label.setText("Browser monitor: Running")
        self.status_label.setText("Browser monitoring started")
        SweetAlert.toast("Browser Monitor", "Started monitoring for MyAnimeList tabs", self, 4000)

    def stop_browser_monitoring(self):
        """Stop monitoring browser tabs"""
        self.browser_monitor.stop_monitoring()
        self.start_monitor_button.setEnabled(True)
        self.stop_monitor_button.setEnabled(False)
        self.browser_status_label.setText("Browser monitor: Stopped")
        self.status_label.setText("Browser monitoring stopped")
        SweetAlert.toast("Browser Monitor", "Stopped monitoring", self, 3000)

    def test_qbittorrent_connection(self):
        """Test connection to qBittorrent"""
        # Update torrent manager with current settings
        self.torrent_manager = TorrentManager(
            host=self.qb_host_input.text(),
            port=self.qb_port_input.value(),
            username=self.qb_username_input.text(),
            password=self.qb_password_input.text()
        )

        if self.torrent_manager.is_connected():
            SweetAlert.success("Connection Test", "Successfully connected to qBittorrent!", self)
            self.qb_status_label.setText("qBittorrent: Connected")
            self.qb_status_label.setStyleSheet("color: green")
        else:
            SweetAlert.error("Connection Test", "Failed to connect to qBittorrent. Please check your settings and make sure qBittorrent is running.", self)
            self.qb_status_label.setText("qBittorrent: Disconnected")
            self.qb_status_label.setStyleSheet("color: red")

    def browse_anime_folder(self):
        """Browse for anime folder"""
        folder = QFileDialog.getExistingDirectory(self, "Select Anime Folder", self.anime_folder_input.text())
        if folder:
            self.anime_folder_input.setText(folder)

    def browse_icons_folder(self):
        """Browse for icons folder"""
        folder = QFileDialog.getExistingDirectory(self, "Select Icons Folder", self.icons_folder_input.text())
        if folder:
            self.icons_folder_input.setText(folder)

    def apply_settings(self):
        """Apply settings from the settings tab"""
        # Update torrent manager
        self.torrent_manager = TorrentManager(
            host=self.qb_host_input.text(),
            port=self.qb_port_input.value(),
            username=self.qb_username_input.text(),
            password=self.qb_password_input.text()
        )

        # Update scheduler settings
        # In a full implementation, we would reschedule jobs here

        QMessageBox.information(self, "Settings Applied", "Settings have been applied successfully.")
        self.status_label.setText("Settings updated")

    def update_icons_manual(self):
        """Manually trigger icon update"""
        self.status_label.setText("Starting icon update...")

        self.icon_thread = IconUpdateThread(parent=self)
        self.icon_thread.update_progress.connect(self.on_icon_update_progress)
        self.icon_thread.update_complete.connect(self.on_icon_update_complete)
        self.icon_thread.finished.connect(lambda: self._remove_thread(self.icon_thread))
        self.active_threads.append(self.icon_thread)
        self.icon_thread.start()

    def on_icon_update_progress(self, message: str):
        """Handle icon update progress"""
        self.status_label.setText(message)

    def on_icon_update_complete(self, success: bool, message: str):
        """Handle icon update completion"""
        if success:
            QMessageBox.information(self, "Icon Update", message)
            self.status_label.setText("Icon update completed")
        else:
            QMessageBox.warning(self, "Icon Update Failed", message)
            self.status_label.setText("Icon update failed")

    def show_about(self):
        """Show about dialog"""
        QMessageBox.about(
            self, "About General Downloader",
            """
            <h2>General Downloader</h2>
            <p>Version 1.0</p>
            <p>An automated anime downloading and organization system</p>
            <p>Features:</p>
            <ul>
                <li>Search and download anime from nyaa.si</li>
                <li>Integration with qBittorrent</li>
                <li>Browser tab monitoring for Zen Browser</li>
                <li>Automatic folder icon and synopsis generation</li>
                <li>Scheduled automated tasks</li>
            </ul>
            <p>&copy; 2026 General Downloader</p>
            """
        )

    def closeEvent(self, event):
        """Handle application close event"""
        reply = QMessageBox.question(
            self, "Confirm Exit",
            "Are you sure you want to exit General Downloader?\n"
            "Background services will be stopped.",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No
        )

        if reply == QMessageBox.StandardButton.Yes:
            print("[CLOSE] Stopping background services...")
            # Stop background services
            stop_scheduler()
            print("[CLOSE] Scheduler stopped")
            self.browser_monitor.stop_monitoring()
            print("[CLOSE] Browser monitor stopped")

            # Wait for any running QThreads to finish
            threads_to_wait = [thread for thread in self.active_threads if thread.isRunning()]
            print(f"[CLOSE] Waiting for {len(threads_to_wait)} QThreads to finish...")

            for i, thread in enumerate(threads_to_wait):
                print(f"[CLOSE] Waiting for QThread {i+1}/{len(threads_to_wait)}: {type(thread).__name__}")
                thread.wait(5000)  # wait up to 5 seconds
                if thread.isRunning():
                    print(f"[CLOSE] QThread {type(thread).__name__} did not finish in time")
                else:
                    print(f"[CLOSE] QThread {type(thread).__name__} finished")

            # Wait for scheduler thread to finish
            from General_Downloader.scheduler import scheduler
            if scheduler.is_running and scheduler.scheduler_thread and scheduler.scheduler_thread.is_alive():
                print("[CLOSE] Waiting for scheduler thread to finish...")
                scheduler.scheduler_thread.join(timeout=5)
                if scheduler.scheduler_thread.is_alive():
                    print("[CLOSE] Scheduler thread did not finish in time")
                else:
                    print("[CLOSE] Scheduler thread finished")

            # Wait for browser monitor thread to finish
            if self.browser_monitor.is_monitoring and self.browser_monitor.monitor_thread and self.browser_monitor.monitor_thread.is_alive():
                print("[CLOSE] Waiting for browser monitor thread to finish...")
                self.browser_monitor.monitor_thread.join(timeout=5)
                if self.browser_monitor.monitor_thread.is_alive():
                    print("[CLOSE] Browser monitor thread did not finish in time")
                else:
                    print("[CLOSE] Browser monitor thread finished")

            # Accept the close event
            event.accept()
            print("[CLOSE] Event accepted")
        else:
            event.ignore()

def main():
    import traceback
    try:
        app = QApplication(sys.argv)
        app.setApplicationName("General Downloader")
        app.setApplicationVersion("1.0")

        # Set application style
        app.setStyle('Fusion')

        window = GeneralDownloaderMainWindow()
        window.show()

        print("[MAIN] Starting event loop...")
        exit_code = app.exec()
        print(f"[MAIN] Event loop exited with code: {exit_code}")
        sys.exit(exit_code)
    except Exception as e:
        print(f"[MAIN] FATAL ERROR: {e}")
        traceback.print_exc()
        sys.exit(1)

if __name__ == "__main__":
    main()