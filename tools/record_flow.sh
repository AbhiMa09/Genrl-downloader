#!/usr/bin/env bash
set -eo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRATCH_DIR="${SCRATCH_DIR:-/home/genrl/.gemini/antigravity/brain/d325a24e-cc96-4f27-959a-64ff796d34bc/scratch/test_db}"
RECORD_DIR="/home/genrl/.gemini/antigravity/brain/d325a24e-cc96-4f27-959a-64ff796d34bc"
FRAMES_DIR="$RECORD_DIR/flow_frames"
VIDEO_OUT="$RECORD_DIR/flow.mp4"
EVENT_LOG="$RECORD_DIR/flow_events.log"

MODE="normal"
EXTRA_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --qbit-down)
            MODE="qbit_down"
            shift
            ;;
        --jikan-down)
            MODE="jikan_down"
            shift
            ;;
        *)
            EXTRA_ARGS+=("$1")
            shift
            ;;
    esac
done

echo "=========================================================="
echo " Genrl Flow Recorder (Real App & Core Event Capture)"
echo "=========================================================="
echo "Mode:        $MODE"
echo "Scratch DB:  $SCRATCH_DIR"
echo "Frames:      $FRAMES_DIR"
echo "Video:       $VIDEO_OUT"
echo "Event Log:   $EVENT_LOG"
echo "=========================================================="

# 1. Reset Scratch DB from pristine real DB copy
mkdir -p "$SCRATCH_DIR" "$FRAMES_DIR"
rm -f "$FRAMES_DIR"/*.png 2>/dev/null || true
rm -f "$SCRATCH_DIR"/genrl.db* 2>/dev/null || true

REAL_DB="$HOME/.local/share/genrl-downloader/genrl.db"
if [ ! -f "$REAL_DB" ]; then
    echo "ERROR: Real database not found at $REAL_DB"
    exit 1
fi

echo "[*] Backing up real DB to scratch directory via sqlite3 .backup..."
sqlite3 "$REAL_DB" ".backup '$SCRATCH_DIR/genrl.db'"
sqlite3 "$REAL_DB" ".backup '$SCRATCH_DIR/genrl.db.pristine'"

# Set first_run_complete = false on scratch DB only so wizard starts at Step 0
sqlite3 "$SCRATCH_DIR/genrl.db" "UPDATE settings SET value = 'false' WHERE key = 'first_run_complete';"
sqlite3 "$SCRATCH_DIR/genrl.db.pristine" "UPDATE settings SET value = 'false' WHERE key = 'first_run_complete';"

# Ensure akumasan2 username
sqlite3 "$SCRATCH_DIR/genrl.db" "UPDATE settings SET value = 'akumasan2' WHERE key IN ('state.mal_account', 'state.mal_public_username');"
sqlite3 "$SCRATCH_DIR/genrl.db.pristine" "UPDATE settings SET value = 'akumasan2' WHERE key IN ('state.mal_account', 'state.mal_public_username');"

if [ "$MODE" = "qbit_down" ]; then
    echo "[!] Mode qbit_down: Setting qbit_url to unreachable port 9992..."
    sqlite3 "$SCRATCH_DIR/genrl.db" "UPDATE settings SET value = 'http://127.0.0.1:9992' WHERE key = 'qbit_url';"
    sqlite3 "$SCRATCH_DIR/genrl.db.pristine" "UPDATE settings SET value = 'http://127.0.0.1:9992' WHERE key = 'qbit_url';"
elif [ "$MODE" = "jikan_down" ]; then
    echo "[!] Mode jikan_down: Setting jikan_base_url to unreachable port 9999..."
    sqlite3 "$SCRATCH_DIR/genrl.db" "UPDATE settings SET value = 'http://127.0.0.1:9999/v4' WHERE key = 'jikan_base_url';"
    sqlite3 "$SCRATCH_DIR/genrl.db.pristine" "UPDATE settings SET value = 'http://127.0.0.1:9999/v4' WHERE key = 'jikan_base_url';"
fi

echo "[OK] Scratch DB ready (Real DB untouched: $(sha256sum "$REAL_DB" | cut -d' ' -f1))"

# 2. Start core event logging background worker
> "$EVENT_LOG"
echo "[*] Initializing core event logger..."

(
    LAST_ID=0
    while true; do
        if [ -f "$SCRATCH_DIR/genrl.db" ]; then
            ROWS=$(sqlite3 -separator ' | ' "$SCRATCH_DIR/genrl.db" "SELECT id, strftime('%Y-%m-%d %H:%M:%S', at, 'unixepoch'), level, message FROM events WHERE id > $LAST_ID ORDER BY id ASC;" 2>/dev/null || true)
            if [ -n "$ROWS" ]; then
                while IFS= read -r line; do
                    [ -z "$line" ] && continue
                    ID=$(echo "$line" | cut -d'|' -f1 | tr -d ' ')
                    if [ -n "$ID" ] && [ "$ID" -gt "$LAST_ID" ]; then
                        LAST_ID=$ID
                    fi
                    echo "[$ID] $line" >> "$EVENT_LOG"
                done <<< "$ROWS"
            fi
        fi
        sleep 0.5
    done
) &
LOGGER_PID=$!

# 3. Start periodic screenshot worker (Spectacle for native KDE Wayland captures, 1 frame every 2s)
echo "[*] Starting screen frame capture worker (1 frame every 2s)..."
(
    while true; do
        TS=$(date +%s)
        FRAME_FILE="$FRAMES_DIR/frame_${TS}.png"
        spectacle -b -n -a -o "$FRAME_FILE" 2>/dev/null || spectacle -b -n -f -o "$FRAME_FILE" 2>/dev/null || true
        sleep 2
    done
) &
FRAME_RECORDER_PID=$!

cleanup() {
    echo ""
    echo "[*] Stopping background monitors and recorders..."
    kill $LOGGER_PID 2>/dev/null || true
    kill $FRAME_RECORDER_PID 2>/dev/null || true
    wait $LOGGER_PID 2>/dev/null || true
    wait $FRAME_RECORDER_PID 2>/dev/null || true

    # Encode spectacle frames into flow.mp4 (guaranteed non-black native Wayland capture)
    FRAME_COUNT=$(ls -1 "$FRAMES_DIR"/*.png 2>/dev/null | wc -l)
    if [ "$FRAME_COUNT" -gt 0 ]; then
        echo "[*] Compiling $FRAME_COUNT captured frames into $VIDEO_OUT..."
        ffmpeg -y -framerate 1 -pattern_type glob -i "$FRAMES_DIR/*.png" -c:v libx264 -pix_fmt yuv420p -vf "scale=trunc(iw/2)*2:trunc(ih/2)*2" "$VIDEO_OUT" 2>/dev/null || true
        echo "[OK] Generated $VIDEO_OUT ($FRAME_COUNT frames from spectacle, non-black)"
    else
        echo "[WARN] No frames captured."
    fi
    echo "[OK] Event log saved to $EVENT_LOG"
}
trap cleanup EXIT

# 5. Launch the real Flutter Linux app
APP_BIN="$REPO_ROOT/app/build/linux/x64/debug/bundle/genrl_downloader"
if [ ! -f "$APP_BIN" ]; then
    echo "[*] Building Linux debug bundle..."
    (cd "$REPO_ROOT/app" && flutter build linux --debug)
fi

echo "[*] Launching real Flutter app in wizard mode against scratch DB..."
echo "    Interact with the wizard (walk 5 steps, review, launch)."
echo "    Close the window when finished."
echo "=========================================================="

GDK_BACKEND=x11 DISPLAY="${DISPLAY:-:0}" "$APP_BIN" --data-dir="$SCRATCH_DIR" --wizard "${EXTRA_ARGS[@]}"
