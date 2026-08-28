"""
Nyaa.si scraper for searching anime torrents
"""
import requests
from bs4 import BeautifulSoup
import re
import json
from typing import List, Dict, Optional
import time
import urllib.parse

class NyaaScraper:
    def __init__(self):
        self.base_url = "https://nyaa.si"
        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
        })

    def search_anime(self, anime_name: str, category: str = "1_2") -> List[Dict]:
        """
        Search for anime on nyaa.si
        category: 1_2 = Anime -> English-translated
                 1_3 = Anime -> Non-English-translated
                 1_4 = Anime -> Raw
        """
        # Clean the search term
        clean_name = re.sub(r'[\[\(].*?[\]\)]', '', anime_name).strip()
        clean_name = re.sub(r'\b(1080p|720p|480p|HEVC|x265|x264|BD|WEB|DUAL|AUDIO|SUB)\b', '', clean_name, flags=re.IGNORECASE).strip()

        # URL encode the search term
        encoded_name = urllib.parse.quote_plus(clean_name)
        url = f"{self.base_url}/?f=0&c={category}&q={encoded_name}"

        try:
            response = self.session.get(url, timeout=10)
            response.raise_for_status()
            return self._parse_search_results(response.text)
        except Exception as e:
            print(f"Error searching nyaa.si: {e}")
            return []

    def _parse_search_results(self, html: str) -> List[Dict]:
        """Parse nyaa.si search results"""
        soup = BeautifulSoup(html, 'html.parser')
        results = []

        # Find the torrents table
        table = soup.find('table', class_='torrent-list')
        if not table:
            return results

        # Get all rows except the header (first row)
        rows = table.find_all('tr')[1:]  # Skip header

        for row in rows:
            try:
                cells = row.find_all('td')
                if len(cells) < 8:
                    continue

                # Extract title and links from cell 1
                title_cell = cells[1]
                title_links = title_cell.find_all('a')
                title = ""
                detail_link = ""

                # Look for the main title link (usually the one with the longest text or specific pattern)
                for link in title_links:
                    link_text = link.get_text(strip=True)
                    # Skip short links like numbers, comments, etc.
                    if len(link_text) > 10 and not link_text.isdigit() and 'comment' not in link.get('class', []):
                        title = link_text
                        detail_link = link.get('href', '')
                        break

                # If we didn't find a good title, use the first meaningful link
                if not title and title_links:
                    for link in title_links:
                        link_text = link.get_text(strip=True)
                        if len(link_text) > 5:
                            title = link_text
                            detail_link = link.get('href', '')
                            break

                # Extract magnet link from cell 2 (download links cell)
                magnet_link = ""
                downloads_cell = cells[2]
                download_links = downloads_cell.find_all('a')
                for link in download_links:
                    href = link.get('href', '')
                    if href.startswith('magnet:'):
                        magnet_link = href
                        break

                # Extract size from cell 3
                size_text = cells[3].get_text(strip=True)

                # Extract date from cell 4
                date_text = cells[4].get_text(strip=True)

                # Extract seeders and leechers from cells 5 and 6
                seeders = int(cells[5].get_text(strip=True) or 0)
                leechers = int(cells[6].get_text(strip=True) or 0)

                # Extract category from cell 0
                category_cell = cells[0]
                category_text = category_cell.get_text(strip=True)

                if title and magnet_link:
                    # Make detail URL absolute if needed
                    if detail_link and detail_link.startswith('/'):
                        detail_url = self.base_url + detail_link
                    elif detail_link:
                        detail_url = detail_link
                    else:
                        detail_url = ""

                    results.append({
                        'title': title,
                        'magnet': magnet_link,
                        'size': size_text,
                        'date': date_text,
                        'seeders': seeders,
                        'leechers': leechers,
                        'category': category_text,
                        'detail_url': detail_url
                    })
            except Exception as e:
                print(f"Error parsing row: {e}")
                continue

        # Sort by seeders descending
        results.sort(key=lambda x: x['seeders'], reverse=True)
        return results

    def get_torrents_for_anime(self, anime_name: str, limit: int = 100) -> List[Dict]:
        """
        Get torrents for an anime, trying multiple categories
        """
        all_results = []

        # Try different categories
        categories = ["1_2", "1_3", "1_4"]  # English, Non-English, Raw

        for category in categories:
            results = self.search_anime(anime_name, category)
            all_results.extend(results)
            if len(all_results) >= limit:
                break
            time.sleep(0.5)  # Be respectful to the server

        # Remove duplicates based on magnet link
        seen = set()
        unique_results = []
        for result in all_results:
            if result['magnet'] not in seen:
                seen.add(result['magnet'])
                unique_results.append(result)

        # Sort by seeders and return top results
        unique_results.sort(key=lambda x: x['seeders'], reverse=True)
        return unique_results[:limit]

if __name__ == "__main__":
    # Test the scraper
    scraper = NyaaScraper()
    anime_name = "One Piece"
    print(f"Searching for: {anime_name}")
    results = scraper.get_torrents_for_anime(anime_name, limit=5)

    print(f"\nFound {len(results)} results:")
    for i, torrent in enumerate(results, 1):
        print(f"{i}. {torrent['title']}")
        print(f"   Size: {torrent['size']} | Seeders: {torrent['seeders']} | Leeches: {torrent['leechers']}")
        print(f"   Magnet: {torrent['magnet'][:50]}...")
        print()