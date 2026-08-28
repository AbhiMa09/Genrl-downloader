# General Downloader

An automated anime downloading and organization system that integrates with nyaa.si, qBittorrent, and provides automatic folder icon and synopsis generation.

## Features

- **Nyaa.si Integration**: Search for anime torrents sorted by seeders
- **qBittorrent Integration**: Add and manage torrents directly from the application
- **Browser Tab Monitoring**: Detect anime tabs in Zen Browser (and other browsers)
- **Automatic Processing**: Generate folder icons and synopsis files after downloads
- **Scheduled Tasks**: Automated daily icon updates and periodic tab checking
- **Tabbed Interface**: Easy navigation between different functionalities
- **System Tray**: Run in background with easy access

## Requirements

- Windows 10/11
- Python 3.8+
- qBittorrent (running on localhost:8080 with default credentials)
- Dependencies listed in `requirements.txt`

## Installation

1. Install Python 3.8+ from https://python.org
2. Install qBittorrent from https://qbittorrent.org
3. Make sure qBittorrent is running and accessible at http://localhost:8080
   - Default username: admin
   - Default password: adminadmin
   - (Change these in Settings for security)
4. Install dependencies:
   ```
   pip install -r requirements.txt
   ```
5. Run the application:
   - Double-click `run.bat` OR
   - Run `python launcher.py`

## Usage

### Search & Download Tab
1. Enter an anime name in the search box
2. Click "Search" or press Enter
3. Browse results sorted by seeders (highest first)
4. Click "Download" on a result or select a row and click "Download Selected Torrent"
5. Monitor progress in the Torrents tab

### Torrents Tab
- View active torrents from qBittorrent
- See download/upload speeds, progress, ETA
- Refresh to update status

### Browser Monitor Tab
- Start/stop monitoring for anime tabs in your browser
- See detected anime tabs with confidence scores
- Configure automatic actions (search/download)

### Settings Tab
- Configure qBittorrent connection settings
- Enable/disable scheduled tasks
- Set folder paths for anime and icons
- Apply settings

## How It Works

1. **Search**: When you search for an anime, the application queries nyaa.si and returns torrents sorted by seeders
2. **Download**: Selected torrents are added to qBittorrent via its API
3. **Processing**: After downloads complete, run the icon manager to:
   - Fetch anime information from AniList
   - Download cover art and convert to ICO format
   - Set folder icons using desktop.ini
   - Generate synopsis.txt files with story details
4. **Automation**: 
   - Daily at 10 PM: Automatic icon/synopsis update
   - Every 15 minutes: Browser tab checking for new anime
   - Browser detection: When you visit nyaa.si or anime sites, get notified

## Customization

- Change qBittorrent host/port/credentials in Settings
- Modify scheduled task times in the scheduler.py file
- Adjust browser monitoring sensitivity in browser_monitor.py
- Customize icon sizes or synopsis format in anime_icon_manager.py

## Safety Features

- Automatic downloads are DISABLED by default (enable in Browser Monitor tab)
- Manual confirmation required for most actions
- No automatic file deletion or modification without explicit user action
- Credentials stored only in memory, not saved to disk

## Troubleshooting

### qBittorrent Connection Issues
- Make sure qBittorrent is running
- Check that Web API is enabled (Tools -> Options -> Web UI)
- Verify host/port/credentials in Settings
- Try accessing http://localhost:8080 in your browser

### Browser Monitoring Not Working
- Ensure pygetwindow and psutil are installed
- Try running as administrator for better window access
- Some browsers may require additional permissions

### Icon Updates Failing
- Icon updates require Administrator privileges
- Run the application as Administrator or enable "Run as administrator" in properties
- Make sure D:\icons folder exists and is writable

## Files Overview

- `launcher.py` - Main entry point
- `main_window.py` - Primary GUI application
- `nyaa_scraper.py` - Nyaa.si search functionality
- `torrent_manager.py` - qBittorrent API integration
- `browser_monitor.py` - Browser tab detection
- `scheduler.py` - Automated task scheduling
- `anime_icon_manager.py` - Existing icon/synopsis generator (provided separately)
- `requirements.txt` - Python dependencies

## License

This project is for personal use only. Please respect copyright laws and only download content you have the right to access.

## Disclaimer

This tool is provided as-is without any warranties. The developers are not responsible for any misuse or legal issues arising from the use of this software.