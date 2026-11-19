#!/usr/bin/env python3
import http.server
import json
import os
import socketserver
import urllib.parse
import urllib.request

PORT = int(os.environ.get("GENRL_TMDB_MOCK_PORT", "8999"))

class TMDBHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        qs = urllib.parse.parse_qs(parsed.query)
        path = parsed.path

        if path.endswith("/configuration"):
            data = {
                "images": {
                    "base_url": "http://image.tmdb.org/t/p/",
                    "secure_base_url": "https://image.tmdb.org/t/p/",
                    "backdrop_sizes": ["w300", "w780", "w1280", "original"],
                    "poster_sizes": ["w92", "w154", "w185", "w342", "w500", "w780", "original"]
                }
            }
        elif path.endswith("/search/movie"):
            data = {
                "page": 1,
                "total_pages": 1,
                "total_results": 2,
                "results": [
                    {
                        "id": 8844,
                        "title": "Jumanji",
                        "original_title": "Jumanji",
                        "release_date": "1995-12-15",
                        "overview": "When siblings Judy and Peter discover an enchanted board game that opens the door to a magical world, they unwittingly invite Alan -- an adult who has been trapped inside the game for 26 years -- into their living room.",
                        "poster_path": "/vgpXmVaVyUL7GGiDeiK1mKEKzcX.jpg",
                        "backdrop_path": "/7bXg7Zc0g8sHkU1g3v1q4j1b6d1.jpg"
                    },
                    {
                        "id": 353486,
                        "title": "Jumanji: Welcome to the Jungle",
                        "original_title": "Jumanji: Welcome to the Jungle",
                        "release_date": "2017-12-09",
                        "overview": "Four teenagers in detention discover an old video console with a game they have never heard of. When they decide to play, they are immediately sucked into the jungle world of Jumanji in the bodies of their avatars.",
                        "poster_path": "/bXrFLDMPOcvh04h7gH6F46A1r.jpg",
                        "backdrop_path": "/cpX1G6v6qYv4g9f2j6b0b8c.jpg"
                    }
                ]
            }
        elif path.endswith("/movie/8844"):
            data = {
                "id": 8844,
                "title": "Jumanji",
                "original_title": "Jumanji",
                "release_date": "1995-12-15",
                "overview": "When siblings Judy and Peter discover an enchanted board game that opens the door to a magical world, they unwittingly invite Alan -- an adult who has been trapped inside the game for 26 years -- into their living room.",
                "poster_path": "/vgpXmVaVyUL7GGiDeiK1mKEKzcX.jpg",
                "backdrop_path": "/7bXg7Zc0g8sHkU1g3v1q4j1b6d1.jpg",
                "runtime": 104,
                "status": "Released"
            }
        elif path.endswith("/movie/8844/external_ids"):
            data = {
                "id": 8844,
                "imdb_id": "tt0113497",
                "wikidata_id": "Q222939"
            }
        elif path.endswith("/search/tv"):
            data = {
                "page": 1,
                "total_pages": 1,
                "total_results": 1,
                "results": [
                    {
                        "id": 213713,
                        "name": "My Demon",
                        "original_name": "마이 데몬",
                        "first_air_date": "2023-11-24",
                        "overview": "A pitiless demon becomes powerless after getting entangled with an icy heiress, who may hold the key to his lost abilities -- and his heart.",
                        "poster_path": "/xBnscv5BrJREKVSvh0le61y4KDk.jpg",
                        "backdrop_path": "/9bA3B9aA1C2d3e4f5g6h7i8j9k.jpg"
                    }
                ]
            }
        elif path.endswith("/tv/213713"):
            data = {
                "id": 213713,
                "name": "My Demon",
                "original_name": "마이 데몬",
                "first_air_date": "2023-11-24",
                "overview": "A pitiless demon becomes powerless after getting entangled with an icy heiress, who may hold the key to his lost abilities -- and his heart.",
                "poster_path": "/xBnscv5BrJREKVSvh0le61y4KDk.jpg",
                "backdrop_path": "/9bA3B9aA1C2d3e4f5g6h7i8j9k.jpg",
                "number_of_seasons": 1,
                "number_of_episodes": 16,
                "status": "Ended",
                "seasons": [
                    {
                        "id": 1,
                        "season_number": 1,
                        "episode_count": 16,
                        "name": "Season 1"
                    }
                ]
            }
        elif path.endswith("/tv/213713/season/1"):
            episodes = []
            for ep_num in range(1, 17):
                episodes.append({
                    "id": 5000000 + ep_num,
                    "name": f"Episode {ep_num}",
                    "overview": f"My Demon Episode {ep_num}",
                    "season_number": 1,
                    "episode_number": ep_num,
                    "air_date": "2023-11-24"
                })
            data = {
                "id": 1,
                "season_number": 1,
                "episodes": episodes
            }
        elif path.endswith("/tv/213713/external_ids"):
            data = {
                "id": 213713,
                "imdb_id": "tt29415848",
                "tvdb_id": 439062
            }
        else:
            self.send_response(404)
            self.end_headers()
            return

        body = json.dumps(data).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

if __name__ == "__main__":
    socketserver.TCPServer.allow_reuse_address = True
    with socketserver.TCPServer(("127.0.0.1", PORT), TMDBHandler) as httpd:
        print(f"TMDB bridge listening on http://127.0.0.1:{PORT}")
        httpd.serve_forever()
