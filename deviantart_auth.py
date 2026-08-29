"""
DeviantArt PKCE OAuth2 Authentication Module
Handles full PKCE flow for DeviantArt API (Public client, client_id=76027)
"""
import os
import sys
import json
import hashlib
import base64
import secrets
import threading
import time
import urllib.parse
import webbrowser
import http.server
import socketserver
from pathlib import Path
from typing import Optional, Dict, Any
import requests
import logging

logger = logging.getLogger(__name__)

# DeviantArt app configuration
CLIENT_ID = "76027"
REDIRECT_PORT = 8765
REDIRECT_URI = f"http://127.0.0.1:{REDIRECT_PORT}/callback"
SCOPE = "browse"
AUTHORIZE_URL = "https://www.deviantart.com/oauth2/authorize"
TOKEN_URL = "https://www.deviantart.com/oauth2/token"
API_BASE = "https://www.deviantart.com/api/v1/oauth2"

# Config file location (same as qBittorrent settings)
CONFIG_DIR = Path.home() / "AppData" / "Roaming" / "GeneralDownloader"
CONFIG_FILE = CONFIG_DIR / "config.json"

# In-memory token storage
_access_token: Optional[str] = None
_access_token_expiry: float = 0
_refresh_token: Optional[str] = None
_auth_lock = threading.Lock()


def _load_config() -> Dict[str, Any]:
    """Load config from JSON file"""
    if CONFIG_FILE.exists():
        try:
            with open(CONFIG_FILE, 'r', encoding='utf-8') as f:
                return json.load(f)
        except Exception as e:
            logger.warning(f"Failed to load config: {e}")
    return {}


def _save_config(config: Dict[str, Any]) -> None:
    """Save config to JSON file"""
    try:
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        with open(CONFIG_FILE, 'w', encoding='utf-8') as f:
            json.dump(config, f, indent=2)
    except Exception as e:
        logger.error(f"Failed to save config: {e}")


def _generate_pkce_pair() -> tuple[str, str]:
    """Generate PKCE code_verifier and code_challenge (S256)"""
    code_verifier = base64.urlsafe_b64encode(secrets.token_bytes(32)).decode('ascii').rstrip('=')
    code_challenge = base64.urlsafe_b64encode(
        hashlib.sha256(code_verifier.encode('ascii')).digest()
    ).decode('ascii').rstrip('=')
    return code_verifier, code_challenge


class CallbackHandler(http.server.BaseHTTPRequestHandler):
    """HTTP request handler for OAuth callback"""
    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path == '/callback':
            params = urllib.parse.parse_qs(parsed.query)
            code = params.get('code', [None])[0]
            state = params.get('state', [None])[0]
            error = params.get('error', [None])[0]
            
            self.server.auth_code = code
            self.server.auth_state = state
            self.server.auth_error = error
            
            self.send_response(200)
            self.send_header('Content-type', 'text/html')
            self.end_headers()
            self.wfile.write(b"""
            <html><body style='font-family: sans-serif; text-align: center; padding: 50px;'>
                <h2 style='color: #4CAF50;'>Authorization Successful!</h2>
                <p>You can close this tab and return to the application.</p>
                <p><small>This window will close automatically in 5 seconds...</small></p>
                <script>setTimeout(() => window.close(), 5000);</script>
            </body></html>
            """)
        else:
            self.send_response(404)
            self.end_headers()
    
    def log_message(self, format, *args):
        pass  # Suppress default log messages


def _run_local_server(state: str, code_verifier: str) -> Optional[str]:
    """Run local HTTP server to catch OAuth callback"""
    try:
        with socketserver.TCPServer(("127.0.0.1", REDIRECT_PORT), CallbackHandler) as httpd:
            httpd.auth_code = None
            httpd.auth_state = None
            httpd.auth_error = None
            httpd.timeout = 300  # 5 minute timeout
            
            logger.info(f"Waiting for OAuth callback on {REDIRECT_URI}...")
            
            while httpd.auth_code is None and httpd.auth_error is None:
                httpd.handle_request()
            
            if httpd.auth_error:
                raise Exception(f"OAuth error: {httpd.auth_error}")
            
            if httpd.auth_state != state:
                raise Exception("State mismatch - possible CSRF attack")
            
            return httpd.auth_code
    except OSError as e:
        if e.errno == 98 or e.errno == 10048:  # Address already in use
            raise Exception(f"Port {REDIRECT_PORT} is already in use. Please close other instances.")
        raise


def _exchange_code_for_tokens(code: str, code_verifier: str) -> Dict[str, Any]:
    """Exchange authorization code for access/refresh tokens"""
    data = {
        'grant_type': 'authorization_code',
        'client_id': CLIENT_ID,
        'code': code,
        'redirect_uri': REDIRECT_URI,
        'code_verifier': code_verifier,
    }
    headers = {'Content-Type': 'application/x-www-form-urlencoded'}
    
    response = requests.post(TOKEN_URL, data=data, headers=headers, timeout=30)
    response.raise_for_status()
    return response.json()


def _refresh_access_token(refresh_token: str) -> Dict[str, Any]:
    """Refresh access token using refresh token"""
    data = {
        'grant_type': 'refresh_token',
        'client_id': CLIENT_ID,
        'refresh_token': refresh_token,
    }
    headers = {'Content-Type': 'application/x-www-form-urlencoded'}
    
    response = requests.post(TOKEN_URL, data=data, headers=headers, timeout=30)
    response.raise_for_status()
    return response.json()


def _save_refresh_token(refresh_token: str) -> None:
    """Save refresh token to config"""
    config = _load_config()
    config['deviantart_refresh_token'] = refresh_token
    _save_config(config)
    logger.info("Saved DeviantArt refresh token to config")


def _load_refresh_token() -> Optional[str]:
    """Load refresh token from config"""
    config = _load_config()
    return config.get('deviantart_refresh_token')


def _perform_interactive_login() -> str:
    """Perform the one-time interactive PKCE login flow"""
    logger.info("Starting DeviantArt PKCE OAuth flow...")
    
    # Generate PKCE pair
    code_verifier, code_challenge = _generate_pkce_pair()
    state = secrets.token_urlsafe(32)
    
    # Build authorize URL
    params = {
        'response_type': 'code',
        'client_id': CLIENT_ID,
        'redirect_uri': REDIRECT_URI,
        'scope': SCOPE,
        'code_challenge': code_challenge,
        'code_challenge_method': 'S256',
        'state': state,
    }
    authorize_url = f"{AUTHORIZE_URL}?{urllib.parse.urlencode(params)}"
    
    logger.info("Opening browser for DeviantArt authorization...")
    print(f"\nOpening browser for DeviantArt authorization...")
    print(f"If browser doesn't open, go to: {authorize_url}\n")
    webbrowser.open(authorize_url)
    
    # Run local server to catch callback
    code = _run_local_server(state, code_verifier)
    
    # Exchange code for tokens
    logger.info("Exchanging authorization code for tokens...")
    tokens = _exchange_code_for_tokens(code, code_verifier)
    
    access_token = tokens['access_token']
    refresh_token = tokens.get('refresh_token')
    expires_in = tokens.get('expires_in', 3600)
    
    if refresh_token:
        _save_refresh_token(refresh_token)
    
    logger.info("Successfully obtained access token")
    return access_token


def get_access_token() -> str:
    """
    Get a valid DeviantArt access token.
    Handles all PKCE flow logic internally:
    1. Check config for refresh_token -> silent refresh
    2. If no refresh_token -> interactive login
    3. If refresh fails -> fallback to interactive login (interactive only)
    Returns valid access token or raises exception.
    """
    global _access_token, _access_token_expiry, _refresh_token
    
    with _auth_lock:
        # Check if we have a valid in-memory token
        if _access_token and time.time() < _access_token_expiry - 60:  # 60s buffer
            return _access_token
        
        # Try silent refresh with saved refresh_token
        saved_refresh = _load_refresh_token()
        if saved_refresh and _refresh_token != saved_refresh:
            _refresh_token = saved_refresh
        
        if _refresh_token:
            try:
                logger.info("Attempting silent token refresh...")
                tokens = _refresh_access_token(_refresh_token)
                _access_token = tokens['access_token']
                _access_token_expiry = time.time() + tokens.get('expires_in', 3600)
                
                # Update refresh token if rotated
                new_refresh = tokens.get('refresh_token')
                if new_refresh and new_refresh != _refresh_token:
                    _refresh_token = new_refresh
                    _save_refresh_token(_refresh_token)
                
                logger.info("Token refreshed successfully")
                return _access_token
            except requests.HTTPError as e:
                if e.response is not None and e.response.status_code in (400, 401):
                    logger.warning("Refresh token expired/revoked, clearing saved token")
                    config = _load_config()
                    config.pop('deviantart_refresh_token', None)
                    _save_config(config)
                    _refresh_token = None
                else:
                    logger.warning(f"Token refresh failed: {e}")
            except Exception as e:
                logger.warning(f"Token refresh error: {e}")
        
        # Need interactive login
        # Check if we're in an interactive context (GUI/main thread)
        # For scheduler/background jobs, we should NOT trigger browser login
        if not sys.stdin.isatty() and not os.environ.get('DEVIANTART_ALLOW_LOGIN'):
            raise Exception("DeviantArt auth required but no interactive session available. "
                          "Run main_window.py first to authorize, or set DEVIANTART_ALLOW_LOGIN=1")
        
        access_token = _perform_interactive_login()
        _access_token = access_token
        _access_token_expiry = time.time() + 3600  # Default 1 hour
        return _access_token


# For testing
if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    try:
        token = get_access_token()
        print(f"\nSuccess! Access token obtained (length: {len(token)})")
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)