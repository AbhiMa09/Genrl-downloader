"""
Debug script for nyaa.si scraper
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

    # Save the response for inspection
    with open('debug_response.html', 'w', encoding='utf-8') as f:
        f.write(response.text)
    print("Saved response to debug_response.html")

    # Parse with BeautifulSoup
    soup = BeautifulSoup(response.text, 'html.parser')

    # Find all torrent rows (skip header)
    rows = soup.find_all('tr', {'data-toggle': 'tooltip'})
    print(f"Found {len(rows)} rows with data-toggle='tooltip'")

    # Let's also try to find rows without the data-toggle attribute
    all_rows = soup.find_all('tr')
    print(f"Total rows: {len(all_rows)}")

    # Look for the table structure
    tables = soup.find_all('table')
    print(f"Found {len(tables)} tables")

    for i, table in enumerate(tables):
        print(f"Table {i}: {table.get('class', 'No class')}")
        rows_in_table = table.find_all('tr')
        print(f"  Rows in table: {len(rows_in_table)}")
        if len(rows_in_table) > 0:
            # Show first row content
            first_row = rows_in_table[0]
            cells = first_row.find_all(['td', 'th'])
            print(f"  First row has {len(cells)} cells:")
            for j, cell in enumerate(cells[:5]):  # Show first 5 cells
                print(f"    Cell {j}: {cell.get_text(strip=True)[:50]}")

if __name__ == "__main__":
    debug_search()