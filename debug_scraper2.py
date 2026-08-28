"""
Debug script for nyaa.si scraper - version 2
"""
import requests
from bs4 import BeautifulSoup
import re
import urllib.parse

def debug_search():
    # Clean the search term
    anime_name = "One Piece"
    clean_name = re.sub(r'[\[\(].*?[\]\)]', '', anime_name).strip()
    clean_name = re.sub(r'\b(1080p|720p|480p|HEVC|x265|x264|BD|WEB|DUAL|AUDIO|SUB)\b', '', clean_name, flags=re.IGNORECASE).strip()

    print(f"Original: {anime_name}")
    print(f"Cleaned: {clean_name}")

    # URL encode the search term
    encoded_name = urllib.parse.quote_plus(clean_name)
    url = f"https://nyaa.si/?f=0&c=1_2&q={encoded_name}"

    print(f"URL: {url}")

    # Get the page
    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
    }

    response = requests.get(url, headers=headers, timeout=10)
    print(f"Response status: {response.status_code}")
    print(f"Response length: {len(response.text)}")

    # Parse with BeautifulSoup
    soup = BeautifulSoup(response.text, 'html.parser')

    # Find the torrents table
    table = soup.find('table', class_='torrent-list')
    if table:
        print("Found torrent-list table")
        # Get all rows except the header (first row)
        rows = table.find_all('tr')[1:]  # Skip header
        print(f"Found {len(rows)} data rows")

        # Examine first few rows to understand structure
        for i, row in enumerate(rows[:3]):
            print(f"\nRow {i}:")
            print(f"  Attributes: {row.attrs}")
            cells = row.find_all('td')
            print(f"  Number of cells: {len(cells)}")
            for j, cell in enumerate(cells):
                print(f"    Cell {j}: {cell.get_text(strip=True)[:50]}")
                # Check for links
                links = cell.find_all('a')
                if links:
                    for k, link in enumerate(links):
                        print(f"      Link {k}: {link.get('href', 'No href')[:50]}")
                        print(f"      Link text: {link.get_text(strip=True)[:30]}")
    else:
        print("Could not find torrent-list table")
        # Let's see what tables we do have
        tables = soup.find_all('table')
        print(f"Found {len(tables)} tables total")
        for i, table in enumerate(tables):
            print(f"Table {i} classes: {table.get('class', 'No class')}")

if __name__ == "__main__":
    debug_search()