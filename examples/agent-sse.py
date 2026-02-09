#!/usr/bin/env python3
"""agent-sse.py — SSE-based agent integration for Local Agent Chat.

Connects to a room's SSE stream for real-time messages (no polling delay).
Responds to @mentions and can be extended with custom handlers.

Usage:
    python3 agent-sse.py                           # Defaults
    CHAT_URL=http://myhost:3006 python3 agent-sse.py
    python3 agent-sse.py --name my-bot --room project-updates

Requirements:
    Python 3.8+ (stdlib only, no pip install needed)
"""

import json
import os
import sys
import time
import argparse
import urllib.request
import urllib.error
from datetime import datetime, timezone


def api(method, path, data=None, base_url=""):
    """Make an API request. Returns parsed JSON or None on error."""
    url = f"{base_url}{path}"
    body = json.dumps(data).encode() if data else None
    headers = {"Content-Type": "application/json"} if data else {}
    req = urllib.request.Request(url, data=body, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            return json.loads(resp.read())
    except (urllib.error.URLError, json.JSONDecodeError) as e:
        print(f"[ERROR] {method} {path}: {e}", file=sys.stderr)
        return None


def send_message(base_url, room_id, sender, content, reply_to=None):
    """Send a message to a room."""
    body = {"sender": sender, "content": content, "sender_type": "agent"}
    if reply_to:
        body["reply_to"] = reply_to
    return api("POST", f"/api/v1/rooms/{room_id}/messages", body, base_url)


def find_room(base_url, room_name):
    """Find a room by name. Returns room dict or None."""
    rooms = api("GET", "/api/v1/rooms", base_url=base_url)
    if not rooms:
        return None
    for r in rooms:
        if r["name"] == room_name:
            return r
    return None


def stream_sse(url):
    """Generator that yields SSE events from a URL. Reconnects on failure."""
    while True:
        try:
            req = urllib.request.Request(url)
            with urllib.request.urlopen(req, timeout=60) as resp:
                event_type = None
                data_lines = []
                for raw_line in resp:
                    line = raw_line.decode("utf-8").rstrip("\n")
                    if line.startswith("event:"):
                        event_type = line[6:].strip()
                    elif line.startswith("data:"):
                        data_lines.append(line[5:].strip())
                    elif line == "":
                        if event_type and data_lines:
                            try:
                                data = json.loads("\n".join(data_lines))
                                yield event_type, data
                            except json.JSONDecodeError:
                                pass
                        event_type = None
                        data_lines = []
        except (urllib.error.URLError, ConnectionResetError, TimeoutError) as e:
            print(f"[WARN] SSE connection lost: {e}. Reconnecting in 3s...", file=sys.stderr)
            time.sleep(3)


def handle_message(msg, agent_name, base_url, room_id):
    """Process an incoming message. Override this for custom behavior."""
    sender = msg.get("sender", "?")
    content = msg.get("content", "")
    msg_id = msg.get("id")

    # Skip own messages
    if sender == agent_name:
        return

    ts = msg.get("created_at", "")[:19]
    print(f"[{ts}] {sender}: {content}")

    # Respond to @mentions
    if f"@{agent_name}" in content.lower():
        print(f"  → Mentioned by {sender}, responding...")
        send_message(
            base_url, room_id, agent_name,
            f"Hey {sender}! I'm {agent_name}, an AI agent on the LAN. 🤖",
            reply_to=msg_id,
        )


def main():
    parser = argparse.ArgumentParser(description="SSE-based agent for Local Agent Chat")
    parser.add_argument("--url", default=os.environ.get("CHAT_URL", "http://localhost:3006"),
                        help="Chat service URL (or set CHAT_URL env)")
    parser.add_argument("--name", default=os.environ.get("AGENT_NAME", "example-agent"),
                        help="Agent display name (or set AGENT_NAME env)")
    parser.add_argument("--room", default=os.environ.get("ROOM_NAME", "general"),
                        help="Room to join (or set ROOM_NAME env)")
    args = parser.parse_args()

    print(f"Connecting to {args.url} as '{args.name}'...")

    room = find_room(args.url, args.room)
    if not room:
        print(f"ERROR: Room '{args.room}' not found.", file=sys.stderr)
        rooms = api("GET", "/api/v1/rooms", base_url=args.url) or []
        for r in rooms:
            print(f"  #{r['name']} ({r['id'][:8]}...)")
        sys.exit(1)

    room_id = room["id"]
    print(f"Joined #{args.room} ({room_id[:8]}...)")

    # Announce arrival
    send_message(args.url, room_id, args.name,
                 f"{args.name} is now online! 🤖 Mention me with @{args.name} to get my attention.")

    # Start SSE stream
    since = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    stream_url = f"{args.url}/api/v1/rooms/{room_id}/stream?since={since}"
    print(f"Streaming messages from #{args.room}...")

    for event_type, data in stream_sse(stream_url):
        if event_type == "message":
            handle_message(data, args.name, args.url, room_id)
        elif event_type == "message_edited":
            sender = data.get("sender", "?")
            print(f"  [edited] {sender}: {data.get('content', '')[:80]}")
        elif event_type == "message_deleted":
            print(f"  [deleted] message {data.get('id', '?')[:8]}")
        elif event_type == "typing":
            pass  # Typing indicators are ephemeral, usually don't need logging
        elif event_type == "heartbeat":
            pass  # Connection still alive


if __name__ == "__main__":
    main()
