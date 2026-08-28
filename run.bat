@echo off
title General Downloader
cd /d "%~dp0"
echo Starting General Downloader...
echo.
echo Make sure qBittorrent is running on localhost:8080
echo.
python launcher.py
pause