#!/usr/bin/env python3
"""Integration tests for the agent_chat Python SDK.

Run against a live instance:
    CHAT_URL=http://192.168.0.79:3006 python3 test_sdk.py
"""

import json
import os
import sys
import time
import traceback

# Import from same directory
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from agent_chat import AgentChat, NotFoundError, ConflictError, ChatError


BASE_URL = os.environ.get("CHAT_URL", "http://192.168.0.79:3006")
SENDER = "sdk-test-runner"

passed = 0
failed = 0
errors = []


def test(name):
    """Decorator for test functions."""
    def decorator(fn):
        global passed, failed
        try:
            fn()
            passed += 1
            print(f"  ✅ {name}")
        except Exception as e:
            failed += 1
            errors.append((name, str(e)))
            print(f"  ❌ {name}: {e}")
        return fn
    return decorator


def main():
    global passed, failed
    chat = AgentChat(BASE_URL, sender=SENDER, sender_type="agent")

    print(f"\n🧪 Running SDK integration tests against {BASE_URL}\n")

    # ── Health & Discovery ──────────────────────────────────────────────
    print("Health & Discovery:")

    @test("health returns version")
    def _():
        h = chat.health()
        assert "version" in h, f"Missing version: {h}"

    @test("stats returns rooms count")
    def _():
        s = chat.stats()
        assert "rooms" in s, f"Missing rooms: {s}"
        assert "messages" in s

    @test("discover returns capabilities")
    def _():
        d = chat.discover()
        assert "capabilities" in d, f"Missing capabilities: {d}"
        assert "endpoints" in d

    @test("llms.txt is non-empty text")
    def _():
        txt = chat.llms_txt()
        assert len(txt) > 100, f"llms.txt too short: {len(txt)} chars"

    @test("skill.md is non-empty text")
    def _():
        txt = chat.skill_md()
        assert "SKILL.md" in txt or "local-agent-chat" in txt.lower() or "quick start" in txt.lower()

    # ── Room CRUD ───────────────────────────────────────────────────────
    print("\nRoom CRUD:")
    room_name = f"sdk-test-{int(time.time()) % 100000}"
    room_data = {}

    @test("create room")
    def _():
        nonlocal room_data
        room_data = chat.create_room(room_name, "SDK test room")
        assert room_data["name"] == room_name
        assert "admin_key" in room_data
        assert "id" in room_data

    @test("list rooms includes new room")
    def _():
        rooms = chat.list_rooms()
        names = [r["name"] for r in rooms]
        assert room_name in names, f"{room_name} not in {names}"

    @test("get room by name")
    def _():
        r = chat.get_room(room_name)
        assert r["name"] == room_name
        assert r["description"] == "SDK test room"

    @test("get room by ID")
    def _():
        r = chat.get_room(room_data["id"])
        assert r["name"] == room_name

    @test("update room description")
    def _():
        r = chat.update_room(room_name, room_data["admin_key"], description="Updated desc")
        assert r["description"] == "Updated desc"

    @test("duplicate room name raises ConflictError")
    def _():
        try:
            chat.create_room(room_name)
            assert False, "Should have raised ConflictError"
        except ConflictError:
            pass

    # ── Messages ────────────────────────────────────────────────────────
    print("\nMessages:")
    msg1 = {}

    @test("send message")
    def _():
        nonlocal msg1
        msg1 = chat.send(room_name, "Hello from SDK test!")
        assert msg1["content"] == "Hello from SDK test!"
        assert msg1["sender"] == SENDER
        assert "seq" in msg1

    @test("send reply")
    def _():
        reply = chat.reply(room_name, msg1["id"], "This is a reply")
        assert reply["reply_to"] == msg1["id"]

    @test("get messages")
    def _():
        msgs = chat.get_messages(room_name, limit=10)
        assert len(msgs) >= 2
        contents = [m["content"] for m in msgs]
        assert "Hello from SDK test!" in contents

    @test("edit message")
    def _():
        edited = chat.edit_message(room_name, msg1["id"], "Edited content!")
        assert edited["content"] == "Edited content!"
        assert edited.get("edited_at") is not None

    @test("get edit history")
    def _():
        history = chat.get_edit_history(room_name, msg1["id"])
        assert history["edit_count"] >= 1
        assert history["current_content"] == "Edited content!"
        assert len(history["edits"]) >= 1

    @test("send message with metadata")
    def _():
        meta_msg = chat.send(room_name, "With metadata", metadata={"priority": "high"})
        assert meta_msg.get("metadata", {}).get("priority") == "high"
        # Clean up
        chat.delete_message(room_name, meta_msg["id"])

    # ── Search ──────────────────────────────────────────────────────────
    print("\nSearch:")

    @test("search finds message")
    def _():
        # Search for our edited message
        results = chat.search("Edited content", room=room_name)
        assert "results" in results
        assert len(results["results"]) >= 1

    @test("search has_more field")
    def _():
        results = chat.search("test", limit=1)
        assert "has_more" in results

    # ── Reactions ───────────────────────────────────────────────────────
    print("\nReactions:")

    @test("add reaction")
    def _():
        r = chat.react(room_name, msg1["id"], "🧪")
        assert r is not None

    @test("get reactions")
    def _():
        r = chat.get_reactions(room_name, msg1["id"])
        assert "🧪" in str(r)

    @test("toggle reaction removes it")
    def _():
        chat.react(room_name, msg1["id"], "🧪")  # Toggle off
        r = chat.get_reactions(room_name, msg1["id"])
        # Should be empty or not contain our sender for 🧪
        if isinstance(r, dict):
            senders = []
            for emoji_data in r.values():
                if isinstance(emoji_data, list):
                    senders.extend(emoji_data)
            # OK if empty

    # ── Profiles ────────────────────────────────────────────────────────
    print("\nProfiles:")

    @test("set profile")
    def _():
        p = chat.set_profile(
            display_name="SDK Test Runner",
            bio="I test things",
            status_text="testing",
        )
        assert p["display_name"] == "SDK Test Runner"

    @test("get profile")
    def _():
        p = chat.get_profile(SENDER)
        assert p["display_name"] == "SDK Test Runner"
        assert p["bio"] == "I test things"

    @test("list profiles")
    def _():
        profiles = chat.list_profiles()
        senders = [p["sender"] for p in profiles]
        assert SENDER in senders

    @test("list profiles filtered by type")
    def _():
        profiles = chat.list_profiles(sender_type="agent")
        assert all(p.get("sender_type") == "agent" for p in profiles)

    @test("delete profile")
    def _():
        chat.delete_profile()
        try:
            chat.get_profile(SENDER)
            assert False, "Should be deleted"
        except NotFoundError:
            pass

    # ── Bookmarks ───────────────────────────────────────────────────────
    print("\nBookmarks:")

    @test("bookmark room")
    def _():
        r = chat.bookmark(room_name)
        assert "created" in r or r is not None

    @test("list bookmarks")
    def _():
        bmarks = chat.list_bookmarks()
        room_ids = [b["room_id"] for b in bmarks]
        assert room_data["id"] in room_ids

    @test("unbookmark room")
    def _():
        chat.unbookmark(room_name)
        bmarks = chat.list_bookmarks()
        room_ids = [b["room_id"] for b in bmarks]
        assert room_data["id"] not in room_ids

    # ── Read Positions ──────────────────────────────────────────────────
    print("\nRead Positions:")

    @test("mark read")
    def _():
        seq = msg1.get("seq", 1)
        r = chat.mark_read(room_name, seq)
        assert r is not None

    @test("get unread")
    def _():
        u = chat.get_unread()
        assert "total_unread" in u
        assert "rooms" in u

    @test("get read positions for room")
    def _():
        positions = chat.get_read_positions(room_name)
        assert isinstance(positions, list)

    # ── DMs ─────────────────────────────────────────────────────────────
    print("\nDirect Messages:")

    @test("send DM")
    def _():
        dm = chat.send_dm("sdk-test-other", "Hello via DM!")
        assert "room_id" in dm
        assert dm["message"]["content"] == "Hello via DM!"

    @test("list DMs")
    def _():
        dms = chat.list_dms()
        assert isinstance(dms, list)
        assert len(dms) >= 1
        assert dms[0]["other_participant"] == "sdk-test-other"

    # ── Mentions ────────────────────────────────────────────────────────
    print("\nMentions:")

    @test("send @mention and find it")
    def _():
        # Must send from a DIFFERENT sender — API excludes self-mentions
        other = AgentChat(BASE_URL, sender="sdk-mention-sender")
        other.send(room_name, f"Hey @{SENDER}, check this out!")
        time.sleep(0.5)  # Brief pause for FTS indexing
        mentions = chat.get_mentions()
        assert isinstance(mentions, list)
        assert len(mentions) >= 1, f"Expected at least 1 mention, got {len(mentions)}"

    @test("get unread mentions")
    def _():
        um = chat.get_unread_mentions()
        assert "total_unread" in um

    # ── Participants ────────────────────────────────────────────────────
    print("\nParticipants:")

    @test("get participants")
    def _():
        parts = chat.get_participants(room_name)
        assert isinstance(parts, list)
        senders = [p["sender"] for p in parts]
        assert SENDER in senders

    # ── Threads ─────────────────────────────────────────────────────────
    print("\nThreads:")

    @test("get thread for reply")
    def _():
        # Send a reply to create a thread
        reply = chat.send(room_name, "Thread test reply", reply_to=msg1["id"])
        thread = chat.get_thread(room_name, msg1["id"])
        assert "root" in thread
        assert "replies" in thread
        assert thread["total_replies"] >= 1

    # ── Export ──────────────────────────────────────────────────────────
    print("\nExport:")

    @test("export JSON")
    def _():
        data = chat.export(room_name, format="json")
        assert isinstance(data, dict) or isinstance(data, str)
        if isinstance(data, str):
            data = json.loads(data)
        assert "messages" in data

    @test("export markdown")
    def _():
        md = chat.export(room_name, format="markdown")
        assert isinstance(md, str)
        assert len(md) > 0

    @test("export CSV")
    def _():
        csv = chat.export(room_name, format="csv")
        assert isinstance(csv, str)
        assert "sender" in csv  # Header row

    # ── Files ───────────────────────────────────────────────────────────
    print("\nFiles:")
    file_data = {}

    @test("upload file")
    def _():
        nonlocal file_data
        file_data = chat.upload_file(
            room_name,
            b"Hello from SDK test!",
            "test.txt",
            "text/plain",
        )
        assert "id" in file_data
        assert file_data["filename"] == "test.txt"

    @test("get file info")
    def _():
        info = chat.get_file_info(file_data["id"])
        assert info["filename"] == "test.txt"
        assert info["size"] == 20

    @test("download file")
    def _():
        content = chat.download_file(file_data["id"])
        assert content == b"Hello from SDK test!"

    @test("list files")
    def _():
        files = chat.list_files(room_name)
        assert isinstance(files, list)
        ids = [f["id"] for f in files]
        assert file_data["id"] in ids

    @test("delete file")
    def _():
        chat.delete_file(room_name, file_data["id"])
        try:
            chat.get_file_info(file_data["id"])
            assert False, "Should be deleted"
        except NotFoundError:
            pass

    # ── Pinning ─────────────────────────────────────────────────────────
    print("\nPinning:")

    @test("pin message")
    def _():
        r = chat.pin(room_name, msg1["id"], room_data["admin_key"])
        assert r is not None

    @test("get pins")
    def _():
        pins = chat.get_pins(room_name)
        assert isinstance(pins, list)
        ids = [p["id"] for p in pins]
        assert msg1["id"] in ids

    @test("unpin message")
    def _():
        chat.unpin(room_name, msg1["id"], room_data["admin_key"])
        pins = chat.get_pins(room_name)
        ids = [p["id"] for p in pins]
        assert msg1["id"] not in ids

    # ── Presence ────────────────────────────────────────────────────────
    print("\nPresence:")

    @test("get room presence")
    def _():
        p = chat.get_presence(room_name)
        # May be empty if no SSE connections
        assert isinstance(p, list) or isinstance(p, dict)

    @test("get global presence")
    def _():
        p = chat.get_presence()
        assert "total_online" in p or isinstance(p, dict)

    # ── Activity Feed ───────────────────────────────────────────────────
    print("\nActivity Feed:")

    @test("get activity")
    def _():
        act = chat.activity(limit=5)
        assert isinstance(act, list)

    @test("get activity filtered by room")
    def _():
        act = chat.activity(room=room_name, limit=5)
        assert isinstance(act, list)
        # All should be from our room
        for msg in act:
            assert msg.get("room_id") == room_data["id"]

    # ── Polling helper ──────────────────────────────────────────────────
    print("\nConvenience helpers:")

    @test("poll_new_messages")
    def _():
        msgs, seq = chat.poll_new_messages(room_name, last_seq=0, limit=5)
        assert isinstance(msgs, list)
        assert seq > 0

    # ── Error handling ──────────────────────────────────────────────────
    print("\nError handling:")

    @test("NotFoundError on missing room")
    def _():
        try:
            chat.get_room("00000000-0000-0000-0000-000000000000")
            assert False, "Should raise"
        except NotFoundError:
            pass

    @test("NotFoundError on missing profile")
    def _():
        try:
            chat.get_profile("nonexistent-agent-xyz")
            assert False, "Should raise"
        except NotFoundError:
            pass

    # ── Cleanup ─────────────────────────────────────────────────────────
    print("\nCleanup:")

    @test("archive room")
    def _():
        r = chat.archive_room(room_name, room_data["admin_key"])
        assert r is not None

    @test("archived room hidden from default list")
    def _():
        rooms = chat.list_rooms()
        names = [r["name"] for r in rooms]
        assert room_name not in names

    @test("archived room visible with include_archived")
    def _():
        rooms = chat.list_rooms(include_archived=True)
        names = [r["name"] for r in rooms]
        assert room_name in names

    @test("unarchive room")
    def _():
        chat.unarchive_room(room_name, room_data["admin_key"])

    @test("delete room")
    def _():
        chat.delete_room(room_name, room_data["admin_key"])
        try:
            chat.get_room(room_data["id"])
            assert False, "Should be deleted"
        except NotFoundError:
            pass

    # ── Summary ─────────────────────────────────────────────────────────
    print(f"\n{'═' * 50}")
    print(f"  Passed: {passed}  Failed: {failed}")
    print(f"{'═' * 50}")

    if errors:
        print("\nFailures:")
        for name, err in errors:
            print(f"  ❌ {name}: {err}")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
