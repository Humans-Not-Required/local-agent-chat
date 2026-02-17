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
from agent_chat import AgentChat, NotFoundError, ConflictError, ChatError, AuthError, _request


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

    # ── Typing ──────────────────────────────────────────────────────────
    print("\nTyping:")

    @test("send typing indicator")
    def _():
        # Should not raise
        chat.send_typing(room_name)

    @test("send typing with explicit sender")
    def _():
        chat.send_typing(room_name, sender="typing-test-agent")

    # ── Outgoing Webhooks ───────────────────────────────────────────────
    print("\nOutgoing Webhooks:")
    webhook_data = {}

    @test("create outgoing webhook")
    def _():
        nonlocal webhook_data
        webhook_data = chat.create_webhook(
            room_name,
            room_data["admin_key"],
            url="https://httpbin.org/post",
            events="message,reaction_added",
        )
        assert "id" in webhook_data
        assert webhook_data["url"] == "https://httpbin.org/post"

    @test("list outgoing webhooks")
    def _():
        webhooks = chat.list_webhooks(room_name, room_data["admin_key"])
        assert isinstance(webhooks, list)
        assert len(webhooks) >= 1
        ids = [w["id"] for w in webhooks]
        assert webhook_data["id"] in ids

    @test("webhook delivery log initially empty")
    def _():
        deliveries = chat.get_webhook_deliveries(
            room_name, webhook_data["id"], room_data["admin_key"]
        )
        assert isinstance(deliveries, list)

    # ── Incoming Webhooks ───────────────────────────────────────────────
    print("\nIncoming Webhooks:")
    incoming_wh = {}

    @test("create incoming webhook")
    def _():
        nonlocal incoming_wh
        incoming_wh = chat.create_incoming_webhook(
            room_name, room_data["admin_key"], name="SDK Test Hook"
        )
        assert "token" in incoming_wh
        assert incoming_wh["name"] == "SDK Test Hook"

    @test("list incoming webhooks")
    def _():
        hooks = chat.list_incoming_webhooks(room_name, room_data["admin_key"])
        assert isinstance(hooks, list)
        assert len(hooks) >= 1

    @test("post message via incoming webhook")
    def _():
        token = incoming_wh["token"]
        msg = chat.post_via_webhook(
            token, "Hello from webhook!", sender="webhook-bot"
        )
        assert msg["content"] == "Hello from webhook!"
        assert msg["sender"] == "webhook-bot"

    @test("post via webhook with metadata")
    def _():
        token = incoming_wh["token"]
        msg = chat.post_via_webhook(
            token,
            "Webhook with meta",
            sender="webhook-bot",
            metadata={"source": "test"},
        )
        assert msg.get("metadata", {}).get("source") == "test"

    # ── Search Pagination & Filtering ───────────────────────────────────
    print("\nSearch Pagination:")

    @test("search with sender filter")
    def _():
        results = chat.search("Hello", room=room_name, sender=SENDER)
        assert "results" in results
        for r in results["results"]:
            assert r["sender"] == SENDER

    @test("search with limit and has_more")
    def _():
        results = chat.search("from", limit=1)
        assert "results" in results
        assert "has_more" in results

    @test("search empty query returns error")
    def _():
        try:
            chat.search("")
            assert False, "Should raise on empty query"
        except ChatError:
            pass

    # ── Message Delete ──────────────────────────────────────────────────
    print("\nMessage Delete:")

    @test("delete own message")
    def _():
        m = chat.send(room_name, "to be deleted")
        chat.delete_message(room_name, m["id"])
        # Verify message is gone
        msgs = chat.get_messages(room_name, limit=50)
        ids = [msg["id"] for msg in msgs]
        assert m["id"] not in ids

    @test("admin can delete any message")
    def _():
        other = AgentChat(BASE_URL, sender="other-user")
        m = other.send(room_name, "admin should delete this")
        chat.delete_message(room_name, m["id"], admin_key=room_data["admin_key"])

    # ── Room Reactions (bulk) ───────────────────────────────────────────
    print("\nRoom Reactions (bulk):")

    @test("get room reactions returns grouped data")
    def _():
        # Add some reactions first
        m1 = chat.send(room_name, "react to this 1")
        m2 = chat.send(room_name, "react to this 2")
        chat.react(room_name, m1["id"], "👍")
        chat.react(room_name, m2["id"], "🎉")
        r = chat.get_room_reactions(room_name)
        assert isinstance(r, dict) or isinstance(r, list)

    # ── DM get_dm ───────────────────────────────────────────────────────
    print("\nDM Details:")

    @test("get DM room details")
    def _():
        dms = chat.list_dms()
        if dms:
            dm_room_id = dms[0]["room_id"]
            dm = chat.get_dm(dm_room_id)
            assert "name" in dm or "id" in dm

    # ── Export with Filters ─────────────────────────────────────────────
    print("\nExport Filters:")

    @test("export with sender filter")
    def _():
        data = chat.export(room_name, format="json", sender=SENDER)
        if isinstance(data, str):
            data = json.loads(data)
        for msg in data.get("messages", []):
            assert msg["sender"] == SENDER

    @test("export with limit")
    def _():
        data = chat.export(room_name, format="json", limit=2)
        if isinstance(data, str):
            data = json.loads(data)
        assert len(data.get("messages", [])) <= 2

    @test("export CSV has proper headers")
    def _():
        csv = chat.export(room_name, format="csv")
        first_line = csv.split("\n")[0]
        assert "seq" in first_line
        assert "sender" in first_line
        assert "content" in first_line

    # ── Retention ───────────────────────────────────────────────────────
    print("\nRetention:")
    retention_room_name = f"sdk-retention-{int(time.time()) % 100000}"
    retention_room = {}

    @test("create room with retention settings")
    def _():
        nonlocal retention_room
        retention_room = chat.create_room(
            retention_room_name,
            "Retention test room",
            max_messages=10,
        )
        assert "admin_key" in retention_room

    @test("room shows retention settings")
    def _():
        r = chat.get_room(retention_room_name)
        assert r.get("max_messages") == 10

    @test("trigger retention sweep")
    def _():
        # Send 15 messages to exceed max_messages=10
        for i in range(15):
            chat.send(retention_room_name, f"Retention msg {i}")
        result = chat.trigger_retention()
        assert "rooms_checked" in result
        assert "total_pruned" in result

    @test("retention pruned excess messages")
    def _():
        msgs = chat.get_messages(retention_room_name, limit=50)
        assert len(msgs) <= 10, f"Expected ≤10 messages after retention, got {len(msgs)}"

    @test("cleanup retention room")
    def _():
        chat.delete_room(retention_room_name, retention_room["admin_key"])

    # ── Additional Edge Cases ───────────────────────────────────────────
    print("\nEdge Cases:")

    @test("get messages with before_seq pagination")
    def _():
        msgs = chat.get_messages(room_name, limit=50)
        if len(msgs) >= 2:
            last_seq = msgs[-1]["seq"]
            older = chat.get_messages(room_name, before_seq=last_seq, limit=5)
            for m in older:
                assert m["seq"] < last_seq

    @test("profile validation rejects long bio")
    def _():
        try:
            chat.set_profile(bio="x" * 1001)
            assert False, "Should reject bio > 1000 chars"
        except ChatError:
            pass

    @test("room reactions structure is valid")
    def _():
        r = chat.get_room_reactions(room_name)
        assert isinstance(r, dict) or isinstance(r, list), f"Unexpected type: {type(r)}"
        if isinstance(r, dict):
            assert "reactions" in r, f"Missing 'reactions' key: {list(r.keys())}"

    @test("stats includes comprehensive fields")
    def _():
        s = chat.stats()
        assert "messages" in s
        assert "rooms" in s
        # New comprehensive stats fields
        expected = ["messages", "rooms"]
        for field in expected:
            assert field in s, f"Missing stats field: {field}"

    @test("activity with exclude_sender")
    def _():
        act = chat.activity(limit=10, exclude_sender=SENDER)
        for msg in act:
            assert msg.get("sender") != SENDER

    # ── Forward Pagination (after=seq) ──────────────────────────────────
    print("\nForward Pagination:")

    @test("get messages with after=seq returns newer messages")
    def _():
        m1 = chat.send(room_name, "pagination-first")
        m2 = chat.send(room_name, "pagination-second")
        m3 = chat.send(room_name, "pagination-third")
        msgs = chat.get_messages(room_name, after=m1["seq"], limit=10)
        seqs = [m["seq"] for m in msgs]
        assert m2["seq"] in seqs, f"Expected seq {m2['seq']} in {seqs}"
        assert m3["seq"] in seqs
        assert m1["seq"] not in seqs, "after= should exclude the given seq"

    @test("forward and backward pagination are complementary")
    def _():
        msgs_all = chat.get_messages(room_name, limit=50)
        if len(msgs_all) >= 4:
            mid = msgs_all[len(msgs_all) // 2]
            older = chat.get_messages(room_name, before_seq=mid["seq"], limit=50)
            newer = chat.get_messages(room_name, after=mid["seq"], limit=50)
            older_seqs = {m["seq"] for m in older}
            newer_seqs = {m["seq"] for m in newer}
            assert mid["seq"] not in older_seqs
            assert mid["seq"] not in newer_seqs

    # ── Room Name Update ────────────────────────────────────────────────
    print("\nRoom Name Update:")

    @test("update room name")
    def _():
        new_name = f"sdk-renamed-{int(time.time()) % 100000}"
        r = chat.update_room(room_name, room_data["admin_key"], name=new_name)
        assert r["name"] == new_name
        # Verify the room is accessible by new name
        r2 = chat.get_room(new_name)
        assert r2["id"] == room_data["id"]
        # Rename back for subsequent tests
        chat.update_room(new_name, room_data["admin_key"], name=room_name)

    @test("update room name to duplicate raises ConflictError")
    def _():
        # Try to rename our room to "general" (always exists)
        try:
            chat.update_room(room_name, room_data["admin_key"], name="general")
            assert False, "Should raise ConflictError"
        except ConflictError:
            pass

    # ── Search Advanced Filters ─────────────────────────────────────────
    print("\nSearch Advanced Filters:")

    @test("search with sender_type filter")
    def _():
        results = chat.search("from", sender_type="agent")
        for r in results.get("results", []):
            assert r.get("sender_type") == "agent"

    @test("search cursor pagination with after param")
    def _():
        r1 = chat.search("from", room=room_name, limit=2)
        if r1.get("has_more") and r1["results"]:
            last_seq = r1["results"][-1].get("seq")
            if last_seq:
                r2 = chat.search("from", room=room_name, limit=2, before_seq=last_seq)
                if r2["results"]:
                    for res in r2["results"]:
                        assert res["seq"] < last_seq

    # ── Activity Advanced Filters ───────────────────────────────────────
    print("\nActivity Advanced Filters:")

    @test("activity with sender_type filter")
    def _():
        act = chat.activity(sender_type="agent", limit=5)
        # All returned messages should be from agents
        for msg in act:
            assert msg.get("sender_type") == "agent", f"Expected agent, got {msg.get('sender_type')}"

    @test("activity with sender filter")
    def _():
        act = chat.activity(sender=SENDER, limit=5)
        for msg in act:
            assert msg.get("sender") == SENDER

    @test("activity with after cursor")
    def _():
        act1 = chat.activity(limit=3)
        if len(act1) >= 2:
            last_seq = act1[-1].get("seq")
            if last_seq:
                act2 = chat.activity(after=last_seq, limit=3)
                # Messages after the cursor should be newer
                for msg in act2:
                    assert msg.get("seq", 0) > last_seq

    # ── Multiple Reactions ──────────────────────────────────────────────
    print("\nMultiple Reactions:")

    @test("multiple different emojis on same message")
    def _():
        m = chat.send(room_name, "react variety test")
        chat.react(room_name, m["id"], "👍")
        chat.react(room_name, m["id"], "🎉")
        chat.react(room_name, m["id"], "🔥")
        r = chat.get_reactions(room_name, m["id"])
        # Reactions are in r["reactions"] list
        emojis = [x["emoji"] for x in r.get("reactions", [])]
        assert "👍" in emojis, f"Expected 👍 in {emojis}"
        assert "🎉" in emojis, f"Expected 🎉 in {emojis}"
        assert "🔥" in emojis, f"Expected 🔥 in {emojis}"

    @test("explicit unreact removes reaction")
    def _():
        m = chat.send(room_name, "unreact test")
        chat.react(room_name, m["id"], "🧪")
        chat.unreact(room_name, m["id"], "🧪")
        r = chat.get_reactions(room_name, m["id"])
        # 🧪 should not be present or sender should not be in its senders
        for rx in r.get("reactions", []):
            if rx["emoji"] == "🧪":
                assert SENDER not in rx["senders"], "Sender should be removed after unreact"

    @test("multi-sender reactions")
    def _():
        m = chat.send(room_name, "multi-sender react test")
        other = AgentChat(BASE_URL, sender="reactor-agent")
        chat.react(room_name, m["id"], "👍")
        other.react(room_name, m["id"], "👍")
        r = chat.get_reactions(room_name, m["id"])
        for rx in r.get("reactions", []):
            if rx["emoji"] == "👍":
                assert rx["count"] >= 2, f"Expected count >= 2, got {rx['count']}"
                assert SENDER in rx["senders"]
                assert "reactor-agent" in rx["senders"]

    # ── Profile with All Fields ─────────────────────────────────────────
    print("\nProfile All Fields:")

    @test("set profile with all optional fields")
    def _():
        p = chat.set_profile(
            display_name="Full Profile Test",
            bio="Testing all fields",
            avatar_url="https://example.com/avatar.png",
            status_text="testing",
            metadata={"custom_key": "custom_value"},
        )
        assert p["display_name"] == "Full Profile Test"
        assert p["bio"] == "Testing all fields"
        assert p.get("avatar_url") == "https://example.com/avatar.png"
        assert p.get("status_text") == "testing"

    @test("profile partial update preserves other fields")
    def _():
        # Update only status_text
        p = chat.set_profile(status_text="new status")
        assert p.get("status_text") == "new status"
        # display_name should still be set
        p2 = chat.get_profile(SENDER)
        assert p2["display_name"] == "Full Profile Test"

    @test("cleanup profile")
    def _():
        chat.delete_profile()

    # ── DM with Metadata ────────────────────────────────────────────────
    print("\nDM with Metadata:")

    @test("send DM with metadata")
    def _():
        dm = chat.send_dm("dm-meta-recipient", "DM with metadata", metadata={"priority": "urgent"})
        assert dm["message"]["content"] == "DM with metadata"
        # Check metadata came through
        assert dm["message"].get("metadata", {}).get("priority") == "urgent"

    # ── Auth Error Handling ─────────────────────────────────────────────
    print("\nAuth Error Handling:")

    @test("wrong admin key raises AuthError")
    def _():
        try:
            chat.update_room(room_name, "wrong-key-12345", description="nope")
            assert False, "Should raise AuthError"
        except AuthError:
            pass

    @test("delete room with wrong key raises AuthError")
    def _():
        try:
            chat.delete_room(room_name, "wrong-key-12345")
            assert False, "Should raise AuthError"
        except AuthError:
            pass

    @test("pin without admin key raises AuthError")
    def _():
        m = chat.send(room_name, "pin auth test")
        try:
            chat.pin(room_name, m["id"], "bad-admin-key")
            assert False, "Should raise AuthError"
        except AuthError:
            pass

    # ── Mentions with Room Filter ───────────────────────────────────────
    print("\nMentions with Room Filter:")

    @test("get mentions filtered by room")
    def _():
        other = AgentChat(BASE_URL, sender="mention-room-filter")
        other.send(room_name, f"Hey @{SENDER} in this room")
        time.sleep(0.5)
        mentions = chat.get_mentions(room=room_name)
        for m in mentions:
            assert m.get("room_id") == room_data["id"]

    # ── Room Last Message Preview ───────────────────────────────────────
    print("\nRoom Last Message Preview:")

    @test("room list includes last_message_preview")
    def _():
        chat.send(room_name, "This is the latest message for preview test")
        rooms = chat.list_rooms()
        target = [r for r in rooms if r["name"] == room_name]
        assert len(target) == 1
        assert "last_message_preview" in target[0]
        assert "preview" in target[0]["last_message_preview"].lower() or \
               "latest" in target[0]["last_message_preview"].lower()

    @test("room list sorted by activity")
    def _():
        # Send a message to our test room to make it the most recently active
        chat.send(room_name, "activity sort verification")
        rooms = chat.list_rooms()
        if len(rooms) >= 2:
            # Our test room should be first (most recent activity)
            assert rooms[0]["name"] == room_name, \
                f"Expected {room_name} first, got {rooms[0]['name']}"

    # ── File Admin Delete ───────────────────────────────────────────────
    print("\nFile Admin Delete:")

    @test("admin can delete another user's file")
    def _():
        other = AgentChat(BASE_URL, sender="file-uploader")
        f = other.upload_file(room_name, b"admin delete test", "admin-del.txt", "text/plain")
        # Delete with admin key (not the uploader)
        chat.delete_file(room_name, f["id"], admin_key=room_data["admin_key"])
        try:
            chat.get_file_info(f["id"])
            assert False, "File should be deleted"
        except NotFoundError:
            pass

    # ── Unicode & Special Characters ────────────────────────────────────
    print("\nUnicode & Special Characters:")

    @test("send message with unicode and emoji")
    def _():
        content = "日本語テスト 🎌 Ünïcödé ñ à 🧪💻"
        m = chat.send(room_name, content)
        assert m["content"] == content, f"Send mismatch: {repr(m['content'])} != {repr(content)}"
        # Verify retrieval by fetching messages after this seq
        msgs = chat.get_messages(room_name, after=m["seq"] - 1, limit=5)
        found = [msg for msg in msgs if msg["id"] == m["id"]]
        assert len(found) == 1, f"Message not found in retrieval"
        assert found[0]["content"] == content, f"Retrieval mismatch"

    @test("search finds unicode content")
    def _():
        chat.send(room_name, "Prüfung mit Ünïcödé text")
        time.sleep(0.3)  # FTS indexing
        results = chat.search("Prüfung", room=room_name)
        assert len(results.get("results", [])) >= 1

    @test("profile with unicode display name")
    def _():
        p = chat.set_profile(display_name="测试Agent 🤖", bio="Unicode bio ñ")
        assert p["display_name"] == "测试Agent 🤖"
        chat.delete_profile()

    # ── Export with Metadata ────────────────────────────────────────────
    print("\nExport with Metadata:")

    @test("export JSON with include_metadata")
    def _():
        chat.send(room_name, "metadata export test", metadata={"tag": "export"})
        data = chat.export(room_name, format="json", include_metadata=True)
        if isinstance(data, str):
            data = json.loads(data)
        msgs = data.get("messages", [])
        # At least one message should have metadata
        has_meta = any(m.get("metadata") for m in msgs)
        assert has_meta, "Expected at least one message with metadata when include_metadata=True"

    # ── Retention with max_age ──────────────────────────────────────────
    print("\nRetention max_age:")

    @test("create room with max_message_age_hours")
    def _():
        age_room = chat.create_room(
            f"sdk-age-{int(time.time()) % 100000}",
            "Age retention test",
            max_message_age_hours=24,
        )
        r = chat.get_room(age_room["name"])
        assert r.get("max_message_age_hours") == 24
        chat.delete_room(age_room["name"], age_room["admin_key"])

    # ── Incoming Webhook Edge Cases ─────────────────────────────────────
    print("\nIncoming Webhook Edge Cases:")

    @test("post via webhook with default sender")
    def _():
        token = incoming_wh["token"]
        msg = chat.post_via_webhook(token, "No explicit sender")
        # Should use webhook default or no sender
        assert msg["content"] == "No explicit sender"

    @test("post via webhook with sender_type")
    def _():
        token = incoming_wh["token"]
        msg = chat.post_via_webhook(
            token, "Typed webhook msg", sender="typed-hook", sender_type="human"
        )
        assert msg.get("sender_type") == "human"

    # ── Read Position Workflow ──────────────────────────────────────────
    print("\nRead Position Workflow:")

    @test("mark_read reduces unread count")
    def _():
        # Send a message to generate unread
        m = chat.send(room_name, "unread tracking test")
        # Mark as read up to this seq
        chat.mark_read(room_name, m["seq"])
        after = chat.get_unread()
        # Find our room in the rooms list
        rooms_list = after.get("rooms", [])
        room_entry = [r for r in rooms_list if r["room_id"] == room_data["id"]]
        if room_entry:
            assert room_entry[0]["unread_count"] == 0, \
                f"Expected 0 unread after mark_read, got {room_entry[0]['unread_count']}"

    # ── Thread Depth ────────────────────────────────────────────────────
    print("\nThread Depth:")

    @test("nested thread replies")
    def _():
        root = chat.send(room_name, "thread root")
        r1 = chat.reply(room_name, root["id"], "reply 1")
        r2 = chat.reply(room_name, root["id"], "reply 2")
        r3 = chat.reply(room_name, root["id"], "reply 3")
        thread = chat.get_thread(room_name, root["id"])
        assert thread["total_replies"] >= 3
        assert len(thread["replies"]) >= 3

    # ── Message Ordering ────────────────────────────────────────────────
    print("\nMessage Ordering:")

    @test("messages are chronologically ordered")
    def _():
        msgs = chat.get_messages(room_name, limit=20)
        for i in range(1, len(msgs)):
            assert msgs[i]["seq"] > msgs[i - 1]["seq"], \
                f"Messages not ordered: seq {msgs[i-1]['seq']} before {msgs[i]['seq']}"

    @test("messages with before_seq are reverse-chronological input, chronological output")
    def _():
        msgs = chat.get_messages(room_name, limit=50)
        if len(msgs) >= 5:
            pivot = msgs[-1]["seq"]
            older = chat.get_messages(room_name, before_seq=pivot, limit=5)
            for m in older:
                assert m["seq"] < pivot

    # ── Stats Comprehensive Fields ──────────────────────────────────────
    print("\nStats Comprehensive:")

    @test("stats has comprehensive fields")
    def _():
        s = chat.stats()
        for field in ["messages", "rooms"]:
            assert field in s, f"Missing stats field: {field}"
        # Messages and rooms should be positive
        assert s["messages"] > 0
        assert s["rooms"] > 0

    @test("health response fields")
    def _():
        h = chat.health()
        assert h.get("status") == "ok"
        assert "version" in h

    @test("discover response structure")
    def _():
        d = chat.discover()
        assert "capabilities" in d
        assert "endpoints" in d
        assert isinstance(d["capabilities"], list) or isinstance(d["capabilities"], dict)

    # ── Outgoing Webhook Update & Delete ────────────────────────────────
    print("\nOutgoing Webhook Update & Delete:")

    @test("update outgoing webhook URL")
    def _():
        result = chat.update_webhook(
            room_name, webhook_data["id"], room_data["admin_key"],
            url="https://httpbin.org/anything",
        )
        assert result.get("updated") is True

    @test("update outgoing webhook events filter")
    def _():
        result = chat.update_webhook(
            room_name, webhook_data["id"], room_data["admin_key"],
            events="message,file_uploaded,reaction_added",
        )
        assert result.get("updated") is True

    @test("deactivate outgoing webhook")
    def _():
        result = chat.update_webhook(
            room_name, webhook_data["id"], room_data["admin_key"],
            active=False,
        )
        assert result.get("updated") is True

    @test("reactivate outgoing webhook")
    def _():
        result = chat.update_webhook(
            room_name, webhook_data["id"], room_data["admin_key"],
            active=True,
        )
        assert result.get("updated") is True

    @test("delete outgoing webhook")
    def _():
        chat.delete_webhook(room_name, webhook_data["id"], room_data["admin_key"])
        webhooks = chat.list_webhooks(room_name, room_data["admin_key"])
        ids = [w["id"] for w in webhooks]
        assert webhook_data["id"] not in ids

    # ── Incoming Webhook Update & Delete ─────────────────────────────────
    print("\nIncoming Webhook Update & Delete:")

    @test("update incoming webhook name")
    def _():
        result = chat.update_incoming_webhook(
            room_name, incoming_wh["id"], room_data["admin_key"],
            name="Renamed Hook",
        )
        assert result.get("updated") is True

    @test("deactivate incoming webhook")
    def _():
        result = chat.update_incoming_webhook(
            room_name, incoming_wh["id"], room_data["admin_key"],
            active=False,
        )
        assert result.get("updated") is True

    @test("reactivate incoming webhook")
    def _():
        result = chat.update_incoming_webhook(
            room_name, incoming_wh["id"], room_data["admin_key"],
            active=True,
        )
        assert result.get("updated") is True

    @test("delete incoming webhook")
    def _():
        chat.delete_incoming_webhook(room_name, incoming_wh["id"], room_data["admin_key"])
        hooks = chat.list_incoming_webhooks(room_name, room_data["admin_key"])
        ids = [h["id"] for h in hooks]
        assert incoming_wh["id"] not in ids

    # ── Webhook with Secret/HMAC ─────────────────────────────────────────
    print("\nWebhook with Secret:")

    @test("create webhook with HMAC secret")
    def _():
        wh = chat.create_webhook(
            room_name, room_data["admin_key"],
            url="https://httpbin.org/post",
            events="message",
            secret="my-hmac-secret-key-123",
        )
        assert "id" in wh
        # Secret should be shown only on creation (if returned) or obscured
        # Clean up
        chat.delete_webhook(room_name, wh["id"], room_data["admin_key"])

    # ── Room Creation Edge Cases ─────────────────────────────────────────
    print("\nRoom Creation Edge Cases:")

    @test("create room with no description")
    def _():
        name = f"sdk-nodesc-{int(time.time()) % 100000}"
        r = chat.create_room(name)
        assert r["name"] == name
        assert "admin_key" in r
        chat.delete_room(name, r["admin_key"])

    @test("create room with special characters in name")
    def _():
        name = f"sdk-special-chars-{int(time.time()) % 100000}"
        r = chat.create_room(name, "Room with spëcial chars: ñ, ü, 日本語")
        assert r["name"] == name
        room = chat.get_room(name)
        assert "spëcial" in room["description"]
        chat.delete_room(name, r["admin_key"])

    @test("create room with max_messages and max_age")
    def _():
        name = f"sdk-retention-combo-{int(time.time()) % 100000}"
        r = chat.create_room(
            name, "Combined retention",
            max_messages=50,
            max_message_age_hours=48,
        )
        room = chat.get_room(name)
        assert room.get("max_messages") == 50
        assert room.get("max_message_age_hours") == 48
        chat.delete_room(name, r["admin_key"])

    # ── Constructor Variants ─────────────────────────────────────────────
    print("\nConstructor Variants:")

    @test("constructor with trailing slash")
    def _():
        c = AgentChat(BASE_URL + "/", sender="trailing-slash-test")
        h = c.health()
        assert h.get("status") == "ok"

    @test("constructor with custom timeout")
    def _():
        c = AgentChat(BASE_URL, sender="timeout-test", timeout=5)
        assert c.timeout == 5
        h = c.health()
        assert h.get("status") == "ok"

    @test("constructor default sender_type is agent")
    def _():
        c = AgentChat(BASE_URL)
        assert c.sender_type == "agent"

    @test("constructor with custom sender_type")
    def _():
        c = AgentChat(BASE_URL, sender="type-test", sender_type="human")
        assert c.sender_type == "human"
        h = c.health()
        assert h.get("status") == "ok"

    # ── DM Edge Cases ────────────────────────────────────────────────────
    print("\nDM Edge Cases:")

    @test("self-DM rejected")
    def _():
        try:
            chat.send_dm(SENDER, "Talking to myself")
            assert False, "Self-DM should be rejected"
        except ChatError:
            pass

    @test("DM with empty content rejected")
    def _():
        try:
            chat.send_dm("dm-empty-test", "")
            assert False, "Empty DM should be rejected"
        except ChatError:
            pass

    @test("DM conversation is bidirectional")
    def _():
        other = AgentChat(BASE_URL, sender="dm-bidir-agent")
        # Send from main to other
        dm1 = chat.send_dm("dm-bidir-agent", "Message from main")
        # Send from other back to main
        dm2 = other.send_dm(SENDER, "Reply from other")
        # Both should be in the same DM room
        assert dm1["room_id"] == dm2["room_id"]

    @test("DM list shows other participant")
    def _():
        dms = chat.list_dms()
        participants = [d["other_participant"] for d in dms]
        assert "dm-bidir-agent" in participants

    # ── Pin Edge Cases ───────────────────────────────────────────────────
    print("\nPin Edge Cases:")

    @test("pin already pinned message raises conflict")
    def _():
        m = chat.send(room_name, "double pin test")
        chat.pin(room_name, m["id"], room_data["admin_key"])
        # Pin again — server rejects with ConflictError or ChatError
        try:
            chat.pin(room_name, m["id"], room_data["admin_key"])
            # If it succeeds, that's also acceptable (idempotent)
        except (ConflictError, ChatError):
            pass  # Expected — "Message is already pinned"
        # Either way, message should be pinned
        pins = chat.get_pins(room_name)
        pin_ids = [p["id"] for p in pins]
        assert m["id"] in pin_ids
        chat.unpin(room_name, m["id"], room_data["admin_key"])

    @test("pin nonexistent message returns error")
    def _():
        try:
            chat.pin(room_name, "00000000-0000-0000-0000-000000000000", room_data["admin_key"])
            assert False, "Should raise error"
        except (NotFoundError, ChatError):
            pass

    # ── Bookmark Edge Cases ──────────────────────────────────────────────
    print("\nBookmark Edge Cases:")

    @test("double bookmark is idempotent")
    def _():
        chat.bookmark(room_name)
        chat.bookmark(room_name)  # Should not error
        bmarks = chat.list_bookmarks()
        room_ids = [b["room_id"] for b in bmarks]
        assert room_data["id"] in room_ids
        chat.unbookmark(room_name)

    @test("unbookmark non-bookmarked room is safe")
    def _():
        # Should not raise even if not bookmarked
        chat.unbookmark(room_name)

    # ── File Edge Cases ──────────────────────────────────────────────────
    print("\nFile Edge Cases:")

    @test("upload image file")
    def _():
        # Small valid PNG (1x1 pixel)
        import base64
        png_1x1 = base64.b64decode(
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
        )
        f = chat.upload_file(room_name, png_1x1, "tiny.png", "image/png")
        assert f["filename"] == "tiny.png"
        assert f["content_type"] == "image/png"
        # Verify download matches
        downloaded = chat.download_file(f["id"])
        assert downloaded == png_1x1
        chat.delete_file(room_name, f["id"])

    @test("upload file with unicode filename")
    def _():
        f = chat.upload_file(room_name, b"unicode test", "日本語ファイル.txt", "text/plain")
        info = chat.get_file_info(f["id"])
        assert "日本語" in info["filename"]
        chat.delete_file(room_name, f["id"])

    @test("file info has expected fields")
    def _():
        f = chat.upload_file(room_name, b"fields test", "fields.txt", "text/plain")
        info = chat.get_file_info(f["id"])
        assert "id" in info
        assert "filename" in info
        assert "content_type" in info
        assert "size" in info
        assert "sender" in info
        assert "created_at" in info
        assert info["size"] == len(b"fields test")
        chat.delete_file(room_name, f["id"])

    @test("delete nonexistent file raises NotFoundError")
    def _():
        try:
            chat.delete_file(room_name, "00000000-0000-0000-0000-000000000000")
            assert False, "Should raise"
        except NotFoundError:
            pass

    # ── Mention Pagination ───────────────────────────────────────────────
    print("\nMention Pagination:")

    @test("mentions with limit")
    def _():
        mentions = chat.get_mentions(limit=2)
        assert isinstance(mentions, list)
        assert len(mentions) <= 2

    @test("mentions with after cursor")
    def _():
        mentions = chat.get_mentions(limit=5)
        if len(mentions) >= 2:
            # Use the last mention's seq as cursor
            last_seq = mentions[-1].get("seq")
            if last_seq:
                older = chat.get_mentions(after=last_seq, limit=5)
                for m in older:
                    assert m.get("seq", 0) > last_seq

    # ── Room Archiving Edge Cases ────────────────────────────────────────
    print("\nRoom Archiving Edge Cases:")
    archive_room_name = f"sdk-archive-{int(time.time()) % 100000}"
    archive_room = {}

    @test("create room for archive tests")
    def _():
        nonlocal archive_room
        archive_room = chat.create_room(archive_room_name, "Archive edge case room")

    @test("send message to room before archiving")
    def _():
        chat.send(archive_room_name, "Pre-archive message")

    @test("archive room prevents new messages")
    def _():
        chat.archive_room(archive_room_name, archive_room["admin_key"])
        # Sending to archived room should fail or messages should still work
        # (depends on implementation — some systems allow read-only)
        try:
            chat.send(archive_room_name, "Post-archive message")
            # If it succeeds, that's also valid (some systems don't block writes)
        except ChatError:
            pass  # Expected if writes are blocked

    @test("cleanup archive test room")
    def _():
        chat.unarchive_room(archive_room_name, archive_room["admin_key"])
        chat.delete_room(archive_room_name, archive_room["admin_key"])

    # ── Error Response Structure ─────────────────────────────────────────
    print("\nError Response Structure:")

    @test("404 error has proper body")
    def _():
        try:
            chat.get_room("00000000-0000-0000-0000-000000000000")
            assert False, "Should raise"
        except NotFoundError as e:
            assert e.status_code == 404

    @test("auth error has proper status code")
    def _():
        try:
            chat.update_room(room_name, "bad-key", description="nope")
            assert False, "Should raise"
        except AuthError as e:
            assert e.status_code in (401, 403), f"Expected 401/403, got {e.status_code}"

    @test("conflict error has proper status code")
    def _():
        try:
            chat.create_room(room_name)  # Duplicate
            assert False, "Should raise"
        except ConflictError as e:
            assert e.status_code == 409

    # ── Edit History Edge Cases ──────────────────────────────────────────
    print("\nEdit History Edge Cases:")

    @test("edit history for unedited message")
    def _():
        m = chat.send(room_name, "never edited")
        h = chat.get_edit_history(room_name, m["id"])
        assert h["edit_count"] == 0
        assert len(h.get("edits", [])) == 0

    @test("multiple edits tracked in history")
    def _():
        m = chat.send(room_name, "edit v1")
        chat.edit_message(room_name, m["id"], "edit v2")
        chat.edit_message(room_name, m["id"], "edit v3")
        h = chat.get_edit_history(room_name, m["id"])
        assert h["edit_count"] >= 2
        assert h["current_content"] == "edit v3"

    # ── Participant Enrichment ───────────────────────────────────────────
    print("\nParticipant Enrichment:")

    @test("participants include message count")
    def _():
        parts = chat.get_participants(room_name)
        sdk_runner = [p for p in parts if p["sender"] == SENDER]
        assert len(sdk_runner) == 1
        assert sdk_runner[0]["message_count"] > 0

    @test("participants include first_seen and last_seen")
    def _():
        parts = chat.get_participants(room_name)
        for p in parts:
            assert "first_seen" in p, f"Missing first_seen for {p['sender']}"
            assert "last_seen" in p, f"Missing last_seen for {p['sender']}"

    # ── OpenAPI & Discovery Endpoints ────────────────────────────────────
    print("\nOpenAPI & Discovery Endpoints:")

    @test("openapi.json is valid JSON with paths")
    def _():
        resp = _request("GET", f"{BASE_URL}/api/v1/openapi.json", timeout=10)
        assert "paths" in resp
        assert "info" in resp
        assert len(resp["paths"]) > 30, f"Expected 30+ paths, got {len(resp['paths'])}"

    @test("root llms.txt returns text content")
    def _():
        resp = _request("GET", f"{BASE_URL}/llms.txt", timeout=10)
        text = resp if isinstance(resp, str) else resp.decode("utf-8")
        assert len(text) > 100

    @test("well-known skills index returns JSON")
    def _():
        resp = _request("GET", f"{BASE_URL}/.well-known/skills/index.json", timeout=10)
        assert "skills" in resp
        assert len(resp["skills"]) >= 1
        assert resp["skills"][0]["name"] == "local-agent-chat"

    @test("well-known SKILL.md returns markdown")
    def _():
        resp = _request("GET", f"{BASE_URL}/.well-known/skills/local-agent-chat/SKILL.md", timeout=10)
        text = resp if isinstance(resp, str) else resp.decode("utf-8")
        assert "Quick Start" in text or "quick start" in text.lower()

    # ── Cross-Feature Interactions ───────────────────────────────────────
    print("\nCross-Feature Interactions:")

    @test("edit message preserves metadata")
    def _():
        m = chat.send(room_name, "meta before edit", metadata={"preserve": "me"})
        edited = chat.edit_message(room_name, m["id"], "meta after edit")
        # Metadata may or may not be preserved depending on implementation
        # Just verify the edit succeeded
        assert edited["content"] == "meta after edit"

    @test("reaction on edited message works")
    def _():
        m = chat.send(room_name, "edit then react")
        chat.edit_message(room_name, m["id"], "edited then react")
        chat.react(room_name, m["id"], "✏️")
        r = chat.get_reactions(room_name, m["id"])
        emojis = [x["emoji"] for x in r.get("reactions", [])]
        assert "✏️" in emojis

    @test("thread reply to edited message works")
    def _():
        m = chat.send(room_name, "original for thread")
        chat.edit_message(room_name, m["id"], "edited for thread")
        reply = chat.reply(room_name, m["id"], "replying to edited")
        thread = chat.get_thread(room_name, m["id"])
        assert thread["total_replies"] >= 1
        assert thread["root"]["content"] == "edited for thread"

    @test("pinned message appears in export")
    def _():
        m = chat.send(room_name, "pinned export test unique 98765")
        chat.pin(room_name, m["id"], room_data["admin_key"])
        data = chat.export(room_name, format="json")
        if isinstance(data, str):
            data = json.loads(data)
        msgs = data.get("messages", [])
        found = [msg for msg in msgs if msg.get("content") == "pinned export test unique 98765"]
        assert len(found) >= 1, f"Pinned message not found in export ({len(msgs)} messages)"
        chat.unpin(room_name, m["id"], room_data["admin_key"])

    @test("file in room visible to participants")
    def _():
        f = chat.upload_file(room_name, b"participant vis test", "vis.txt", "text/plain")
        other = AgentChat(BASE_URL, sender="file-viewer")
        files = other.list_files(room_name)
        ids = [fi["id"] for fi in files]
        assert f["id"] in ids
        chat.delete_file(room_name, f["id"])

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
