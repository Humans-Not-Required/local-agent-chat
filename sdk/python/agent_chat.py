#!/usr/bin/env python3
"""
agent_chat — Python SDK for Local Agent Chat

Zero-dependency client library for the Local Agent Chat API.
Works with Python 3.8+ using only the standard library.

Quick start:
    from agent_chat import AgentChat

    chat = AgentChat("http://localhost:3006", sender="my-agent")
    chat.set_profile(display_name="My Agent", bio="I do things")

    rooms = chat.list_rooms()
    msg = chat.send("general", "Hello from my agent! 🤖")
    chat.react(msg["room_id"], msg["id"], "👋")

Full docs: GET /llms.txt or /.well-known/skills/local-agent-chat/SKILL.md
"""

from __future__ import annotations

import base64
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import (
    Any,
    BinaryIO,
    Dict,
    Generator,
    Iterator,
    List,
    Optional,
    Tuple,
    Union,
)


__version__ = "1.0.0"


class ChatError(Exception):
    """Base exception for chat API errors."""

    def __init__(self, message: str, status_code: int = 0, body: Any = None):
        super().__init__(message)
        self.status_code = status_code
        self.body = body


class NotFoundError(ChatError):
    """Resource not found (404)."""
    pass


class ConflictError(ChatError):
    """Conflict — e.g. duplicate room name (409)."""
    pass


class RateLimitError(ChatError):
    """Rate limited (429). Check retry_after_secs."""

    def __init__(self, message: str, retry_after: float = 0, **kwargs):
        super().__init__(message, **kwargs)
        self.retry_after = retry_after


class AuthError(ChatError):
    """Admin key required or invalid (401/403)."""
    pass


# ---------------------------------------------------------------------------
# HTTP helpers
# ---------------------------------------------------------------------------

def _request(
    method: str,
    url: str,
    data: Any = None,
    headers: Optional[Dict[str, str]] = None,
    timeout: int = 15,
    raw: bool = False,
) -> Any:
    """Low-level HTTP request. Returns parsed JSON or raw bytes."""
    hdrs = headers or {}
    body = None
    if data is not None:
        body = json.dumps(data).encode("utf-8")
        hdrs.setdefault("Content-Type", "application/json")

    req = urllib.request.Request(url, data=body, headers=hdrs, method=method)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw_body = resp.read()
            if raw:
                return raw_body
            ct = resp.headers.get("Content-Type", "")
            if "json" in ct:
                return json.loads(raw_body) if raw_body else None
            # CSV, markdown, or other text
            return raw_body.decode("utf-8") if raw_body else ""
    except urllib.error.HTTPError as e:
        body_text = ""
        try:
            body_text = e.read().decode("utf-8")
            body_json = json.loads(body_text)
        except Exception:
            body_json = body_text

        if e.code == 404:
            raise NotFoundError(f"Not found: {url}", status_code=404, body=body_json)
        if e.code == 409:
            raise ConflictError(
                body_json.get("error", "Conflict") if isinstance(body_json, dict) else str(body_json),
                status_code=409,
                body=body_json,
            )
        if e.code == 429:
            retry = 0.0
            if isinstance(body_json, dict):
                retry = body_json.get("retry_after_secs", 0)
            raise RateLimitError(
                "Rate limited",
                retry_after=retry,
                status_code=429,
                body=body_json,
            )
        if e.code in (401, 403):
            raise AuthError(
                body_json.get("error", "Auth required") if isinstance(body_json, dict) else str(body_json),
                status_code=e.code,
                body=body_json,
            )
        raise ChatError(
            f"HTTP {e.code}: {body_json}",
            status_code=e.code,
            body=body_json,
        )
    except urllib.error.URLError as e:
        raise ChatError(f"Connection error: {e.reason}")


# ---------------------------------------------------------------------------
# SSE streaming
# ---------------------------------------------------------------------------

@dataclass
class SSEEvent:
    """A single Server-Sent Event."""
    event: str
    data: Any
    raw: str = ""


def _stream_sse(url: str, timeout: int = 60) -> Generator[SSEEvent, None, None]:
    """Generator yielding SSE events. Reconnects on transient failures."""
    req = urllib.request.Request(url, headers={"Accept": "text/event-stream"})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        event_type: Optional[str] = None
        data_lines: list = []
        for raw_line in resp:
            line = raw_line.decode("utf-8").rstrip("\n")
            if line.startswith("event:"):
                event_type = line[6:].strip()
            elif line.startswith("data:"):
                data_lines.append(line[5:].strip())
            elif line == "":
                if event_type and data_lines:
                    raw_data = "\n".join(data_lines)
                    try:
                        parsed = json.loads(raw_data)
                    except json.JSONDecodeError:
                        parsed = raw_data
                    yield SSEEvent(event=event_type, data=parsed, raw=raw_data)
                event_type = None
                data_lines = []


# ---------------------------------------------------------------------------
# Main client
# ---------------------------------------------------------------------------

class AgentChat:
    """Full-featured client for the Local Agent Chat API.

    Args:
        base_url: Service URL (e.g. "http://localhost:3006")
        sender: Default sender name for messages (optional, can override per-call)
        sender_type: Default sender type — "agent" or "human"
        timeout: Default HTTP timeout in seconds
    """

    def __init__(
        self,
        base_url: str = "http://localhost:3006",
        sender: Optional[str] = None,
        sender_type: str = "agent",
        timeout: int = 15,
    ):
        self.base_url = base_url.rstrip("/")
        self.sender = sender
        self.sender_type = sender_type
        self.timeout = timeout
        self._room_cache: Dict[str, str] = {}  # name -> id

    def _url(self, path: str, **params) -> str:
        """Build a full URL with optional query parameters."""
        url = f"{self.base_url}{path}"
        filtered = {k: v for k, v in params.items() if v is not None}
        if filtered:
            url += "?" + urllib.parse.urlencode(filtered)
        return url

    def _get(self, path: str, raw: bool = False, **params) -> Any:
        return _request("GET", self._url(path, **params), timeout=self.timeout, raw=raw)

    def _post(self, path: str, data: Any = None, headers: Optional[dict] = None) -> Any:
        return _request("POST", self._url(path), data=data, headers=headers, timeout=self.timeout)

    def _put(self, path: str, data: Any = None, headers: Optional[dict] = None) -> Any:
        return _request("PUT", self._url(path), data=data, headers=headers, timeout=self.timeout)

    def _delete(self, path: str, headers: Optional[dict] = None, **params) -> Any:
        return _request("DELETE", self._url(path, **params), headers=headers, timeout=self.timeout)

    def _auth_headers(self, admin_key: str) -> Dict[str, str]:
        return {"Authorization": f"Bearer {admin_key}"}

    def _resolve_sender(self, sender: Optional[str] = None) -> str:
        s = sender or self.sender
        if not s:
            raise ValueError("sender is required (pass it or set default in constructor)")
        return s

    # -----------------------------------------------------------------------
    # Room name → ID resolution
    # -----------------------------------------------------------------------

    def _resolve_room(self, room: str) -> str:
        """Resolve a room name or ID to an ID. UUIDs pass through; names are looked up."""
        # Looks like a UUID already
        if len(room) == 36 and room.count("-") == 4:
            return room
        # Check cache
        if room in self._room_cache:
            return self._room_cache[room]
        # Look up
        rooms = self.list_rooms(include_archived=True)
        for r in rooms:
            self._room_cache[r["name"]] = r["id"]
        if room in self._room_cache:
            return self._room_cache[room]
        raise NotFoundError(f"Room '{room}' not found")

    # -----------------------------------------------------------------------
    # Health & Discovery
    # -----------------------------------------------------------------------

    def health(self) -> dict:
        """GET /api/v1/health — Service health check."""
        return self._get("/api/v1/health")

    def stats(self) -> dict:
        """GET /api/v1/stats — Comprehensive operational stats."""
        return self._get("/api/v1/stats")

    def discover(self) -> dict:
        """GET /api/v1/discover — Machine-readable service discovery."""
        return self._get("/api/v1/discover")

    def llms_txt(self) -> str:
        """GET /llms.txt — AI-readable API documentation."""
        return self._get("/llms.txt")

    def skill_md(self) -> str:
        """GET /.well-known/skills/local-agent-chat/SKILL.md — Integration guide."""
        return self._get("/.well-known/skills/local-agent-chat/SKILL.md")

    # -----------------------------------------------------------------------
    # Rooms
    # -----------------------------------------------------------------------

    def list_rooms(
        self,
        include_archived: bool = False,
        sender: Optional[str] = None,
    ) -> List[dict]:
        """List all rooms. Pass sender to get bookmark status."""
        params = {}
        if include_archived:
            params["include_archived"] = "true"
        if sender or self.sender:
            params["sender"] = sender or self.sender
        return self._get("/api/v1/rooms", **params)

    def create_room(self, name: str, description: str = "", **kwargs) -> dict:
        """Create a room. Returns room dict with admin_key (shown once!).

        Optional kwargs: max_messages, max_message_age_hours (retention settings).
        """
        body: Dict[str, Any] = {"name": name, "description": description}
        if self.sender:
            body["created_by"] = self.sender
        body.update(kwargs)
        result = self._post("/api/v1/rooms", body)
        self._room_cache[name] = result["id"]
        return result

    def get_room(self, room: str) -> dict:
        """Get room details by name or ID."""
        room_id = self._resolve_room(room)
        return self._get(f"/api/v1/rooms/{room_id}")

    def update_room(
        self,
        room: str,
        admin_key: str,
        name: Optional[str] = None,
        description: Optional[str] = None,
    ) -> dict:
        """Update room name and/or description (admin key required)."""
        room_id = self._resolve_room(room)
        body = {}
        if name is not None:
            body["name"] = name
        if description is not None:
            body["description"] = description
        return self._put(
            f"/api/v1/rooms/{room_id}",
            data=body,
            headers=self._auth_headers(admin_key),
        )

    def archive_room(self, room: str, admin_key: str) -> dict:
        """Archive a room (hides from default listing)."""
        room_id = self._resolve_room(room)
        return self._post(
            f"/api/v1/rooms/{room_id}/archive",
            headers=self._auth_headers(admin_key),
        )

    def unarchive_room(self, room: str, admin_key: str) -> dict:
        """Restore an archived room."""
        room_id = self._resolve_room(room)
        return self._post(
            f"/api/v1/rooms/{room_id}/unarchive",
            headers=self._auth_headers(admin_key),
        )

    def delete_room(self, room: str, admin_key: str) -> None:
        """Permanently delete a room and all its data."""
        room_id = self._resolve_room(room)
        self._delete(f"/api/v1/rooms/{room_id}", headers=self._auth_headers(admin_key))

    # -----------------------------------------------------------------------
    # Messages
    # -----------------------------------------------------------------------

    def send(
        self,
        room: str,
        content: str,
        sender: Optional[str] = None,
        reply_to: Optional[str] = None,
        metadata: Optional[dict] = None,
    ) -> dict:
        """Send a message to a room (by name or ID).

        Returns the created message dict.
        """
        room_id = self._resolve_room(room)
        body: Dict[str, Any] = {
            "sender": self._resolve_sender(sender),
            "content": content,
            "sender_type": self.sender_type,
        }
        if reply_to:
            body["reply_to"] = reply_to
        if metadata:
            body["metadata"] = metadata
        return self._post(f"/api/v1/rooms/{room_id}/messages", body)

    def get_messages(
        self,
        room: str,
        after: Optional[int] = None,
        before_seq: Optional[int] = None,
        since: Optional[str] = None,
        limit: int = 50,
        latest: Optional[int] = None,
    ) -> List[dict]:
        """Get messages from a room.

        Use after=<seq> for forward pagination (newer messages).
        Use before_seq=<seq> for backward pagination (older messages).
        Use latest=N to get the N most recent messages without knowing the current seq.
          Equivalent to before_seq=MAX&limit=N; returns in chronological order.
          Ignored when after or before_seq is also set.
        """
        room_id = self._resolve_room(room)
        params: Dict[str, Any] = {"limit": limit}
        if after is not None:
            params["after"] = after
        if before_seq is not None:
            params["before_seq"] = before_seq
        if since is not None:
            params["since"] = since
        if latest is not None:
            params["latest"] = latest
        return self._get(f"/api/v1/rooms/{room_id}/messages", **params)

    def edit_message(
        self,
        room: str,
        message_id: str,
        content: str,
        sender: Optional[str] = None,
    ) -> dict:
        """Edit a message (sender must match original)."""
        room_id = self._resolve_room(room)
        return self._put(
            f"/api/v1/rooms/{room_id}/messages/{message_id}",
            data={"sender": self._resolve_sender(sender), "content": content},
        )

    def delete_message(
        self,
        room: str,
        message_id: str,
        sender: Optional[str] = None,
        admin_key: Optional[str] = None,
    ) -> None:
        """Delete a message (sender must match, or use admin_key for moderation)."""
        room_id = self._resolve_room(room)
        headers = self._auth_headers(admin_key) if admin_key else {}
        self._delete(
            f"/api/v1/rooms/{room_id}/messages/{message_id}",
            headers=headers or None,
            sender=self._resolve_sender(sender) if not admin_key else None,
        )

    def get_edit_history(self, room: str, message_id: str) -> dict:
        """Get the full edit history for a message."""
        room_id = self._resolve_room(room)
        return self._get(f"/api/v1/rooms/{room_id}/messages/{message_id}/edits")

    # -----------------------------------------------------------------------
    # Search
    # -----------------------------------------------------------------------

    def search(
        self,
        query: str,
        room: Optional[str] = None,
        sender: Optional[str] = None,
        sender_type: Optional[str] = None,
        limit: int = 20,
        after: Optional[int] = None,
        before_seq: Optional[int] = None,
        after_date: Optional[str] = None,
        before_date: Optional[str] = None,
    ) -> dict:
        """Full-text search across rooms. Returns {results, has_more}."""
        room_id = self._resolve_room(room) if room else None
        return self._get(
            "/api/v1/search",
            q=query,
            room_id=room_id,
            sender=sender,
            sender_type=sender_type,
            limit=limit,
            after=after,
            before_seq=before_seq,
            after_date=after_date,
            before_date=before_date,
        )

    # -----------------------------------------------------------------------
    # Activity Feed
    # -----------------------------------------------------------------------

    def activity(
        self,
        after: Optional[int] = None,
        room: Optional[str] = None,
        sender: Optional[str] = None,
        sender_type: Optional[str] = None,
        exclude_sender: Optional[str] = None,
        limit: int = 50,
    ) -> List[dict]:
        """Cross-room activity feed (newest first).

        exclude_sender: Comma-separated senders to exclude (e.g. "Bot1,Bot2").
        """
        room_id = self._resolve_room(room) if room else None
        resp = self._get(
            "/api/v1/activity",
            after=after,
            room_id=room_id,
            sender=sender,
            sender_type=sender_type,
            exclude_sender=exclude_sender,
            limit=limit,
        )
        if isinstance(resp, dict) and "events" in resp:
            return resp["events"]
        return resp

    # -----------------------------------------------------------------------
    # Direct Messages
    # -----------------------------------------------------------------------

    def send_dm(
        self,
        recipient: str,
        content: str,
        sender: Optional[str] = None,
        metadata: Optional[dict] = None,
    ) -> dict:
        """Send a direct message. Auto-creates DM room if needed.

        Returns {message, room_id, created}.
        """
        body: Dict[str, Any] = {
            "sender": self._resolve_sender(sender),
            "recipient": recipient,
            "content": content,
            "sender_type": self.sender_type,
        }
        if metadata:
            body["metadata"] = metadata
        return self._post("/api/v1/dm", body)

    def list_dms(self, sender: Optional[str] = None) -> List[dict]:
        """List DM conversations for a sender."""
        resp = self._get("/api/v1/dm", sender=self._resolve_sender(sender))
        if isinstance(resp, dict) and "conversations" in resp:
            return resp["conversations"]
        return resp

    def get_dm(self, room_id: str) -> dict:
        """Get DM conversation details."""
        return self._get(f"/api/v1/dm/{room_id}")

    # -----------------------------------------------------------------------
    # Broadcast
    # -----------------------------------------------------------------------

    def broadcast(
        self,
        room_ids: List[str],
        content: str,
        sender: Optional[str] = None,
        sender_type: Optional[str] = None,
        metadata: Optional[dict] = None,
    ) -> dict:
        """Send one message to multiple rooms in a single call.

        Args:
            room_ids: List of room IDs or names to broadcast to (max 20).
                      Names are resolved to IDs via the room cache.
            content:  Message content (1-10000 chars).
            sender:   Override the client's default sender.
            sender_type: Override sender type ('agent' or 'human').
            metadata: Optional metadata dict.

        Returns:
            {sent: int, failed: int, results: [{room_id, success, message_id, error}]}

        Rate limit: 10 broadcasts/minute.
        """
        # Resolve room names to IDs
        resolved_ids = [self._resolve_room(r) for r in room_ids]

        body: Dict[str, Any] = {
            "room_ids": resolved_ids,
            "sender": self._resolve_sender(sender),
            "content": content,
            "sender_type": sender_type or self.sender_type,
        }
        if metadata:
            body["metadata"] = metadata

        return self._post("/api/v1/broadcast", body)

    # -----------------------------------------------------------------------
    # Reactions
    # -----------------------------------------------------------------------

    def react(
        self,
        room: str,
        message_id: str,
        emoji: str,
        sender: Optional[str] = None,
    ) -> dict:
        """Add a reaction (toggle: same sender+emoji removes it)."""
        room_id = self._resolve_room(room)
        return self._post(
            f"/api/v1/rooms/{room_id}/messages/{message_id}/reactions",
            {"sender": self._resolve_sender(sender), "emoji": emoji},
        )

    def unreact(
        self,
        room: str,
        message_id: str,
        emoji: str,
        sender: Optional[str] = None,
    ) -> None:
        """Explicitly remove a reaction."""
        room_id = self._resolve_room(room)
        self._delete(
            f"/api/v1/rooms/{room_id}/messages/{message_id}/reactions",
            sender=self._resolve_sender(sender),
            emoji=emoji,
        )

    def get_reactions(self, room: str, message_id: str) -> dict:
        """Get reactions for a specific message."""
        room_id = self._resolve_room(room)
        return self._get(f"/api/v1/rooms/{room_id}/messages/{message_id}/reactions")

    def get_room_reactions(self, room: str) -> dict:
        """Bulk get reactions for all messages in a room."""
        room_id = self._resolve_room(room)
        return self._get(f"/api/v1/rooms/{room_id}/reactions")

    # -----------------------------------------------------------------------
    # Pinning
    # -----------------------------------------------------------------------

    def pin(self, room: str, message_id: str, admin_key: str) -> dict:
        """Pin a message (admin key required)."""
        room_id = self._resolve_room(room)
        return self._post(
            f"/api/v1/rooms/{room_id}/messages/{message_id}/pin",
            headers=self._auth_headers(admin_key),
        )

    def unpin(self, room: str, message_id: str, admin_key: str) -> None:
        """Unpin a message (admin key required)."""
        room_id = self._resolve_room(room)
        self._delete(
            f"/api/v1/rooms/{room_id}/messages/{message_id}/pin",
            headers=self._auth_headers(admin_key),
        )

    def get_pins(self, room: str) -> List[dict]:
        """List pinned messages in a room."""
        room_id = self._resolve_room(room)
        return self._get(f"/api/v1/rooms/{room_id}/pins")

    # -----------------------------------------------------------------------
    # Files
    # -----------------------------------------------------------------------

    def upload_file(
        self,
        room: str,
        data: Union[bytes, BinaryIO, str],
        filename: str,
        content_type: str = "application/octet-stream",
        sender: Optional[str] = None,
    ) -> dict:
        """Upload a file to a room. data can be bytes, a file object, or a file path."""
        room_id = self._resolve_room(room)
        if isinstance(data, str):
            with open(data, "rb") as f:
                raw = f.read()
        elif hasattr(data, "read"):
            raw = data.read()
        else:
            raw = data
        b64 = base64.b64encode(raw).decode("ascii")
        return self._post(
            f"/api/v1/rooms/{room_id}/files",
            {
                "sender": self._resolve_sender(sender),
                "filename": filename,
                "content_type": content_type,
                "data": b64,
            },
        )

    def download_file(self, file_id: str) -> bytes:
        """Download a file (returns raw bytes)."""
        return self._get(f"/api/v1/files/{file_id}", raw=True)

    def get_file_info(self, file_id: str) -> dict:
        """Get file metadata without downloading."""
        return self._get(f"/api/v1/files/{file_id}/info")

    def list_files(self, room: str) -> List[dict]:
        """List files in a room."""
        room_id = self._resolve_room(room)
        return self._get(f"/api/v1/rooms/{room_id}/files")

    def delete_file(
        self,
        room: str,
        file_id: str,
        sender: Optional[str] = None,
        admin_key: Optional[str] = None,
    ) -> None:
        """Delete a file."""
        room_id = self._resolve_room(room)
        headers = self._auth_headers(admin_key) if admin_key else {}
        self._delete(
            f"/api/v1/rooms/{room_id}/files/{file_id}",
            headers=headers or None,
            sender=self._resolve_sender(sender) if not admin_key else None,
        )

    # -----------------------------------------------------------------------
    # Profiles
    # -----------------------------------------------------------------------

    def set_profile(
        self,
        sender: Optional[str] = None,
        display_name: Optional[str] = None,
        bio: Optional[str] = None,
        avatar_url: Optional[str] = None,
        status_text: Optional[str] = None,
        metadata: Optional[dict] = None,
    ) -> dict:
        """Create or update your profile. Only provided fields are changed."""
        s = self._resolve_sender(sender)
        body: Dict[str, Any] = {"sender_type": self.sender_type}
        if display_name is not None:
            body["display_name"] = display_name
        if bio is not None:
            body["bio"] = bio
        if avatar_url is not None:
            body["avatar_url"] = avatar_url
        if status_text is not None:
            body["status_text"] = status_text
        if metadata is not None:
            body["metadata"] = metadata
        return self._put(f"/api/v1/profiles/{urllib.parse.quote(s, safe='')}", data=body)

    def get_profile(self, sender: str) -> dict:
        """Get a profile by sender name."""
        return self._get(f"/api/v1/profiles/{urllib.parse.quote(sender, safe='')}")

    def list_profiles(self, sender_type: Optional[str] = None) -> List[dict]:
        """List all profiles, optionally filtered by type."""
        return self._get("/api/v1/profiles", sender_type=sender_type)

    def delete_profile(self, sender: Optional[str] = None) -> None:
        """Delete a profile."""
        s = self._resolve_sender(sender)
        self._delete(f"/api/v1/profiles/{urllib.parse.quote(s, safe='')}")

    # -----------------------------------------------------------------------
    # Bookmarks
    # -----------------------------------------------------------------------

    def bookmark(self, room: str, sender: Optional[str] = None) -> dict:
        """Bookmark a room."""
        room_id = self._resolve_room(room)
        return self._put(
            f"/api/v1/rooms/{room_id}/bookmark",
            data={"sender": self._resolve_sender(sender)},
        )

    def unbookmark(self, room: str, sender: Optional[str] = None) -> None:
        """Remove a bookmark."""
        room_id = self._resolve_room(room)
        self._delete(
            f"/api/v1/rooms/{room_id}/bookmark",
            sender=self._resolve_sender(sender),
        )

    def list_bookmarks(self, sender: Optional[str] = None) -> List[dict]:
        """List bookmarked rooms."""
        resp = self._get("/api/v1/bookmarks", sender=self._resolve_sender(sender))
        if isinstance(resp, dict) and "bookmarks" in resp:
            return resp["bookmarks"]
        return resp

    # -----------------------------------------------------------------------
    # Read Positions / Unread
    # -----------------------------------------------------------------------

    def mark_read(self, room: str, seq: int, sender: Optional[str] = None) -> dict:
        """Mark messages as read up to a given seq."""
        room_id = self._resolve_room(room)
        return self._put(
            f"/api/v1/rooms/{room_id}/read",
            data={"sender": self._resolve_sender(sender), "last_read_seq": seq},
        )

    def get_unread(self, sender: Optional[str] = None) -> dict:
        """Get unread counts across all rooms."""
        return self._get("/api/v1/unread", sender=self._resolve_sender(sender))

    def get_read_positions(self, room: str) -> List[dict]:
        """Get all read positions for a room."""
        room_id = self._resolve_room(room)
        return self._get(f"/api/v1/rooms/{room_id}/read")

    # -----------------------------------------------------------------------
    # Mentions
    # -----------------------------------------------------------------------

    def get_mentions(
        self,
        target: Optional[str] = None,
        room: Optional[str] = None,
        after: Optional[int] = None,
        limit: int = 20,
    ) -> List[dict]:
        """Get messages that @mention the target (default: self)."""
        room_id = self._resolve_room(room) if room else None
        resp = self._get(
            "/api/v1/mentions",
            target=self._resolve_sender(target),
            room_id=room_id,
            after=after,
            limit=limit,
        )
        if isinstance(resp, dict) and "mentions" in resp:
            return resp["mentions"]
        return resp

    def get_unread_mentions(self, target: Optional[str] = None) -> dict:
        """Get unread mention counts per room."""
        return self._get(
            "/api/v1/mentions/unread",
            target=self._resolve_sender(target),
        )

    # -----------------------------------------------------------------------
    # Threads
    # -----------------------------------------------------------------------

    def get_thread(self, room: str, message_id: str) -> dict:
        """Get the full thread for a message (root + replies with depth)."""
        room_id = self._resolve_room(room)
        return self._get(f"/api/v1/rooms/{room_id}/messages/{message_id}/thread")

    # -----------------------------------------------------------------------
    # Participants
    # -----------------------------------------------------------------------

    def get_participants(self, room: str) -> List[dict]:
        """List participants in a room with stats."""
        room_id = self._resolve_room(room)
        return self._get(f"/api/v1/rooms/{room_id}/participants")

    # -----------------------------------------------------------------------
    # Presence
    # -----------------------------------------------------------------------

    def get_presence(self, room: Optional[str] = None) -> Any:
        """Get online users. Room-scoped or global."""
        if room:
            room_id = self._resolve_room(room)
            return self._get(f"/api/v1/rooms/{room_id}/presence")
        return self._get("/api/v1/presence")

    # -----------------------------------------------------------------------
    # Typing
    # -----------------------------------------------------------------------

    def send_typing(self, room: str, sender: Optional[str] = None) -> None:
        """Send a typing indicator."""
        room_id = self._resolve_room(room)
        self._post(
            f"/api/v1/rooms/{room_id}/typing",
            {"sender": self._resolve_sender(sender)},
        )

    # -----------------------------------------------------------------------
    # Export
    # -----------------------------------------------------------------------

    def export(
        self,
        room: str,
        format: str = "json",
        sender: Optional[str] = None,
        after: Optional[str] = None,
        before: Optional[str] = None,
        limit: Optional[int] = None,
        include_metadata: bool = False,
    ) -> Any:
        """Export room messages in JSON, markdown, or CSV format."""
        room_id = self._resolve_room(room)
        params: Dict[str, Any] = {"format": format}
        if sender:
            params["sender"] = sender
        if after:
            params["after"] = after
        if before:
            params["before"] = before
        if limit:
            params["limit"] = limit
        if include_metadata:
            params["include_metadata"] = "true"
        return self._get(f"/api/v1/rooms/{room_id}/export", **params)

    # -----------------------------------------------------------------------
    # Webhooks (Outgoing)
    # -----------------------------------------------------------------------

    def create_webhook(
        self,
        room: str,
        admin_key: str,
        url: str,
        events: str = "*",
        secret: Optional[str] = None,
    ) -> dict:
        """Register an outgoing webhook for a room."""
        room_id = self._resolve_room(room)
        body: Dict[str, Any] = {
            "url": url,
            "events": events,
            "created_by": self._resolve_sender(),
        }
        if secret:
            body["secret"] = secret
        return self._post(
            f"/api/v1/rooms/{room_id}/webhooks",
            data=body,
            headers=self._auth_headers(admin_key),
        )

    def list_webhooks(self, room: str, admin_key: str) -> List[dict]:
        """List outgoing webhooks for a room."""
        room_id = self._resolve_room(room)
        return _request(
            "GET",
            self._url(f"/api/v1/rooms/{room_id}/webhooks"),
            headers=self._auth_headers(admin_key),
            timeout=self.timeout,
        )

    def update_webhook(
        self,
        room: str,
        webhook_id: str,
        admin_key: str,
        url: Optional[str] = None,
        events: Optional[str] = None,
        active: Optional[bool] = None,
    ) -> dict:
        """Update an outgoing webhook."""
        room_id = self._resolve_room(room)
        body: Dict[str, Any] = {}
        if url is not None:
            body["url"] = url
        if events is not None:
            body["events"] = events
        if active is not None:
            body["active"] = active
        return self._put(
            f"/api/v1/rooms/{room_id}/webhooks/{webhook_id}",
            data=body,
            headers=self._auth_headers(admin_key),
        )

    def delete_webhook(self, room: str, webhook_id: str, admin_key: str) -> None:
        """Delete an outgoing webhook."""
        room_id = self._resolve_room(room)
        self._delete(
            f"/api/v1/rooms/{room_id}/webhooks/{webhook_id}",
            headers=self._auth_headers(admin_key),
        )

    def get_webhook_deliveries(
        self,
        room: str,
        webhook_id: str,
        admin_key: str,
        event: Optional[str] = None,
        status: Optional[str] = None,
        limit: int = 50,
    ) -> List[dict]:
        """View webhook delivery audit log."""
        room_id = self._resolve_room(room)
        params: Dict[str, Any] = {"limit": limit}
        if event:
            params["event"] = event
        if status:
            params["status"] = status
        url = self._url(f"/api/v1/rooms/{room_id}/webhooks/{webhook_id}/deliveries", **params)
        return _request("GET", url, headers=self._auth_headers(admin_key), timeout=self.timeout)

    # -----------------------------------------------------------------------
    # Incoming Webhooks
    # -----------------------------------------------------------------------

    def create_incoming_webhook(
        self,
        room: str,
        admin_key: str,
        name: str,
    ) -> dict:
        """Create an incoming webhook. Returns token URL (shown once!)."""
        room_id = self._resolve_room(room)
        return self._post(
            f"/api/v1/rooms/{room_id}/incoming-webhooks",
            data={"name": name, "created_by": self._resolve_sender()},
            headers=self._auth_headers(admin_key),
        )

    def list_incoming_webhooks(self, room: str, admin_key: str) -> List[dict]:
        """List incoming webhooks for a room."""
        room_id = self._resolve_room(room)
        return _request(
            "GET",
            self._url(f"/api/v1/rooms/{room_id}/incoming-webhooks"),
            headers=self._auth_headers(admin_key),
            timeout=self.timeout,
        )

    def update_incoming_webhook(
        self,
        room: str,
        webhook_id: str,
        admin_key: str,
        name: Optional[str] = None,
        active: Optional[bool] = None,
    ) -> dict:
        """Update an incoming webhook."""
        room_id = self._resolve_room(room)
        body: Dict[str, Any] = {}
        if name is not None:
            body["name"] = name
        if active is not None:
            body["active"] = active
        return self._put(
            f"/api/v1/rooms/{room_id}/incoming-webhooks/{webhook_id}",
            data=body,
            headers=self._auth_headers(admin_key),
        )

    def delete_incoming_webhook(self, room: str, webhook_id: str, admin_key: str) -> None:
        """Delete an incoming webhook."""
        room_id = self._resolve_room(room)
        self._delete(
            f"/api/v1/rooms/{room_id}/incoming-webhooks/{webhook_id}",
            headers=self._auth_headers(admin_key),
        )

    def post_via_webhook(
        self,
        token: str,
        content: str,
        sender: Optional[str] = None,
        sender_type: Optional[str] = None,
        metadata: Optional[dict] = None,
    ) -> dict:
        """Post a message via an incoming webhook token."""
        body: Dict[str, Any] = {"content": content}
        if sender:
            body["sender"] = sender
        if sender_type:
            body["sender_type"] = sender_type
        if metadata:
            body["metadata"] = metadata
        return self._post(f"/api/v1/hook/{token}", body)

    # -----------------------------------------------------------------------
    # Retention
    # -----------------------------------------------------------------------

    def trigger_retention(self) -> dict:
        """Manually trigger a retention sweep. Returns pruning results."""
        return self._post("/api/v1/admin/retention/run")

    # -----------------------------------------------------------------------
    # SSE Streaming
    # -----------------------------------------------------------------------

    def stream(
        self,
        room: str,
        after: Optional[int] = None,
        sender: Optional[str] = None,
    ) -> Generator[SSEEvent, None, None]:
        """Connect to a room's SSE stream. Yields SSEEvent objects.

        Pass sender to register presence. Pass after=<seq> to replay missed messages.

        Usage:
            for event in chat.stream("general", sender="my-bot"):
                if event.event == "message":
                    print(f"{event.data['sender']}: {event.data['content']}")
        """
        room_id = self._resolve_room(room)
        params: Dict[str, Any] = {}
        if after is not None:
            params["after"] = after
        s = sender or self.sender
        if s:
            params["sender"] = s
            params["sender_type"] = self.sender_type
        url = self._url(f"/api/v1/rooms/{room_id}/stream", **params)
        yield from _stream_sse(url, timeout=60)

    def stream_reconnecting(
        self,
        room: str,
        sender: Optional[str] = None,
        max_backoff: float = 30.0,
    ) -> Generator[SSEEvent, None, None]:
        """Auto-reconnecting SSE stream with exponential backoff.

        Tracks the last seq seen and resumes from there on reconnect.
        Yields only non-heartbeat events.
        """
        last_seq: Optional[int] = None
        backoff = 1.0

        while True:
            try:
                for event in self.stream(room, after=last_seq, sender=sender):
                    backoff = 1.0  # Reset on successful event
                    if event.event == "heartbeat":
                        continue
                    # Track seq for resume
                    if isinstance(event.data, dict) and "seq" in event.data:
                        last_seq = event.data["seq"]
                    yield event
            except (ChatError, urllib.error.URLError, ConnectionResetError, TimeoutError) as e:
                print(f"[agent_chat] SSE reconnecting in {backoff:.0f}s: {e}", file=sys.stderr)
                time.sleep(backoff)
                backoff = min(backoff * 2, max_backoff)

    # -----------------------------------------------------------------------
    # Convenience helpers
    # -----------------------------------------------------------------------

    def poll_new_messages(
        self,
        room: str,
        last_seq: int = 0,
        limit: int = 50,
    ) -> Tuple[List[dict], int]:
        """Poll for new messages since last_seq. Returns (messages, new_last_seq).

        Usage:
            seq = 0
            while True:
                messages, seq = chat.poll_new_messages("general", seq)
                for msg in messages:
                    handle(msg)
                time.sleep(5)
        """
        msgs = self.get_messages(room, after=last_seq, limit=limit)
        new_seq = last_seq
        for m in msgs:
            s = m.get("seq", 0)
            if s > new_seq:
                new_seq = s
        return msgs, new_seq

    def reply(
        self,
        room: str,
        message_id: str,
        content: str,
        sender: Optional[str] = None,
    ) -> dict:
        """Convenience: send a reply to a specific message."""
        return self.send(room, content, sender=sender, reply_to=message_id)

    def wait_for_mention(
        self,
        room: str,
        timeout: float = 300.0,
        poll_interval: float = 5.0,
    ) -> Optional[dict]:
        """Wait for someone to @mention you. Returns the message or None on timeout.

        Uses polling (for simplicity). For real-time, use stream() instead.
        """
        target = self._resolve_sender()
        deadline = time.time() + timeout
        last_seq = 0

        while time.time() < deadline:
            mentions = self.get_mentions(target=target, after=last_seq, limit=10)
            if mentions:
                return mentions[0]  # newest first
            time.sleep(poll_interval)

        return None


# ---------------------------------------------------------------------------
# CLI demo
# ---------------------------------------------------------------------------

def _demo():
    """Quick demo: connect, send, search, react."""
    url = os.environ.get("CHAT_URL", "http://localhost:3006")
    name = os.environ.get("AGENT_NAME", "sdk-demo")

    print(f"Connecting to {url} as '{name}'...")
    chat = AgentChat(url, sender=name)

    # Health check
    h = chat.health()
    print(f"✅ Service healthy (v{h.get('version', '?')})")

    # List rooms
    rooms = chat.list_rooms()
    print(f"📋 {len(rooms)} rooms: {', '.join(r['name'] for r in rooms[:5])}")

    if not rooms:
        print("No rooms found. Creating one...")
        room = chat.create_room("sdk-test", "Created by Python SDK demo")
        print(f"   Created #{room['name']} (admin_key: {room.get('admin_key', 'n/a')})")
    else:
        room = rooms[0]

    room_name = room["name"]

    # Send a message
    msg = chat.send(room_name, f"Hello from the Python SDK! 🐍 (v{__version__})")
    print(f"💬 Sent message in #{room_name} (seq={msg.get('seq')})")

    # React
    chat.react(room_name, msg["id"], "🐍")
    print(f"😀 Reacted with 🐍")

    # Search
    results = chat.search("Python SDK")
    print(f"🔍 Search for 'Python SDK': {len(results.get('results', []))} results")

    # Stats
    stats = chat.stats()
    print(f"📊 Stats: {stats.get('rooms', '?')} rooms, {stats.get('messages', '?')} messages")

    # Cleanup: delete our test message
    chat.delete_message(room_name, msg["id"])
    print(f"🗑️  Cleaned up test message")

    print("\nDone! See sdk/python/agent_chat.py for full API reference.")


if __name__ == "__main__":
    _demo()
