"""
Main window for the General Downloader application
"""
import sys
import os
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
from PyQt6.QtCore import Qt, QThread, pyqtSignal, QTimer, QTime
from PyQt6.QtGui import QAction, QIcon, QFont, QPixmap

# Import our modules
from nyaa_scraper import NyaaScraper
from torrent_manager import TorrentManager
from browser_monitor import BrowserMonitor, AnimeTabInfo
from scheduler import scheduler, start_scheduler, stop_scheduler, schedule_daily_10pm_icon_update

# Set up logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class TorrentDownloadThread(QThread):
    """Thread for handling torrent downloads without blocking GUI"""
    progress_update = pyqtSignal(str)
    download_complete = pyqtSignal(bool, str)

    def __init__(self, magnet_link: str, save_path: str = None, tags: List[str] = None):
        super().__init__()
        self.magnet_link = magnet_link
        self.save_path = save_path
        self.tags = tags or []
        self.torrent_manager = TorrentManager()

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

    def __init__(self, anime_name: str):
        super().__init__()
        self.anime_name = anime_name
        self.scraper = NyaaScraper()

    def run(self):
        self.search_status.emit(f"Searching for '{self.anime_name}' on nyaa.si...")
        try:
            results = self.scraper.get_torrents_for_anime(self.anime_name, limit=10)
            self.search_results.emit(results)
        except Exception as e:
            self.search_error.emit(str(e))

class IconUpdateThread(QThread):
    """Thread for running the icon manager script"""
    update_progress = pyqtSignal(str)
    update_complete = pyqtSignal(bool, str)

    def __init__(self):
        super().__init__()

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
                logger.debug(f"Icon update output: {result.stdout}")
            else:
                self.update_complete.emit(False, f"Icon update failed: {result.stderr}")
                logger.error(f"Icon update error: {result.stderr}")

        except subprocess.TimeoutExpired:
            self.update_complete.emit(False, "Icon update timed out after 5 minutes")
        except Exception as e:
            self.update_complete.emit(False, f"Error running icon update: {str(e)}")
            logger.error(f"Icon update exception: {e}")

class GeneralDownloaderMainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("General Downloader")
        self.setGeometry(100, 100, 1200, 800)

        # Initialize components
        self.nyaa_scraper = NyaaScraper()
        self.torrent_manager = TorrentManager()
        self.browser_monitor = BrowserMonitor(check_interval=10)
        self.current_search_results = []

        # Set up UI
        self.setup_ui()
        self.setup_menu()
        self.setup_status_bar()
        self.setup_system_tray()

        # Start background services
        self.start_background_services()

        # Set up timers
        self.setup_timers()

        logger.info("General Downloader initialized")

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

        layout.addWidget(monitor_group)
        layout.addWidget(tabs_group)

        # Auto-actions
        actions_group = QGroupBox("Automatic Actions")
        actions_layout = QVBoxLayout(actions_group)

        self.auto_search_checkbox = QCheckBox("Automatically search for new anime tabs")
        self.auto_search_checkbox.setChecked(True)
        actions_layout.addWidget(self.auto_search_checkbox)

        self.auto_download_checkbox = QCheckBox("Automatically download best torrent (highest seeders)")
        self.auto_download_checkbox.setChecked(False)  # Disabled by default for safety
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

        # Start browser monitoring if enabled
        # (We'll start it manually for now to avoid surprises)

        logger.info("Background services started")

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
        self.search_thread = AnimeSearchThread(anime_name)
        self.search_thread.search_results.connect(self.on_search_results)
        self.search_thread.search_error.connect(self.on_search_error)
        self.search_thread.search_status.connect(self.on_search_status)
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

    def on_search_error(self, error: str):
        """Handle search error"""
        self.search_button.setEnabled(True)
        self.search_progress.setVisible(False)
        self.search_status_label.setText("Search failed")
        QMessageBox.critical(self, "Search Error", f"Failed to search: {error}")

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
        else:
            self.selected_info.clear()
            self.download_button.setEnabled(False)

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
        self.download_thread = TorrentDownloadThread(magnet_link, save_path=save_path)
        self.download_thread.progress_update.connect(self.on_download_progress)
        self.download_thread.download_complete.connect(self.on_download_complete)
        self.download_thread.start()

    def on_download_progress(self, message: str):
        """Handle download progress updates"""
        self.status_label.setText(message)

    def on_download_complete(self, success: bool, message: str):
        """Handle download completion"""
        self.download_button.setEnabled(True)
        self.download_button.setText("Download Selected Torrent")

        if success:
            QMessageBox.information(self, "Download Started", message)
            self.search_status_label.setText(message)
        else:
            QMessageBox.warning(self, "Download Failed", message)
            self.search_status_label.setText("Download failed")

        # Refresh torrents to show the new addition
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
        self.download_thread = TorrentDownloadThread(magnet_link, save_path=save_path)
        self.download_thread.download_complete.connect(self.on_manual_download_complete)
        self.download_thread.start()

    def on_manual_download_complete(self, success: bool, message: str):
        """Handle manual torrent download completion"""
        self.add_manual_torrent_button.setEnabled(True)
        self.add_manual_torrent_button.setText("Add Torrent")

        if success:
            QMessageBox.information(self, "Torrent Added", message)
            self.manual_magnet_input.clear()
        else:
            QMessageBox.warning(self, "Failed to Add Torrent", message)

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

    def stop_browser_monitoring(self):
        """Stop monitoring browser tabs"""
        self.browser_monitor.stop_monitoring()
        self.start_monitor_button.setEnabled(True)
        self.stop_monitor_button.setEnabled(False)
        self.browser_status_label.setText("Browser monitor: Stopped")
        self.status_label.setText("Browser monitoring stopped")

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
            QMessageBox.information(self, "Connection Test", "Successfully connected to qBittorrent!")
            self.qb_status_label.setText("qBittorrent: Connected")
            self.qb_status_label.setStyleSheet("color: green")
        else:
            QMessageBox.warning(self, "Connection Test", "Failed to connect to qBittorrent. Please check your settings and make sure qBittorrent is running.")
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

        self.icon_thread = IconUpdateThread()
        self.icon_thread.update_progress.connect(self.on_icon_update_progress)
        self.icon_thread.update_complete.connect(self.on_icon_update_complete)
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
            # Stop background services
            stop_scheduler()
            self.browser_monitor.stop_monitoring()

            # Accept the close event
            event.accept()
        else:
            event.ignore()

def main():
    app = QApplication(sys.argv)
    app.setApplicationName("General Downloader")
    app.setApplicationVersion("1.0")

    # Set application style
    app.setStyle('Fusion')

    window = GeneralDownloaderMainWindow()
    window.show()

    sys.exit(app.exec())

if __name__ == "__main__":
    main()