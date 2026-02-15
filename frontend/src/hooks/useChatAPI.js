import { useCallback, useState } from 'react';
import { API } from '../utils';

const INITIAL_LIMIT = 200;
const LOAD_MORE_LIMIT = 50;

/**
 * useChatAPI — Data fetching and mutation API calls for the chat.
 *
 * Handles: room CRUD, message CRUD, file uploads, reactions, pins,
 * read positions, DMs, typing indicators, and unread counts.
 */
export default function useChatAPI({
  senderRef,
  lastSeqRef,
  setMessages,
  setFiles,
  setReactions,
  setRooms,
  setActiveRoom,
  setUnreadCounts,
  setDmConversations,
  saveAdminKey,
  setAdminKeyInfo,
}) {
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(false);

  // --- Data Fetching ---

  const fetchRooms = useCallback(async () => {
    try {
      const sender = senderRef.current;
      const url = sender ? `${API}/rooms?sender=${encodeURIComponent(sender)}` : `${API}/rooms`;
      const res = await fetch(url);
      if (res.ok) {
        const data = await res.json();
        setRooms(data);
        return data;
      }
    } catch (e) { /* ignore */ }
    return [];
  }, [setRooms, senderRef]);

  const fetchMessages = useCallback(async (roomId) => {
    setLoading(true);
    setHasMore(false);
    try {
      const res = await fetch(`${API}/rooms/${roomId}/messages?limit=${INITIAL_LIMIT}`);
      if (res.ok) {
        const data = await res.json();
        setMessages(data);
        setHasMore(data.length >= INITIAL_LIMIT);
        if (data.length > 0) {
          const lastMsg = data[data.length - 1];
          lastSeqRef.current = lastMsg.seq || null;
        }
      }
    } catch (e) { /* ignore */ }
    setLoading(false);
  }, [setMessages, lastSeqRef]);

  const fetchFiles = useCallback(async (roomId) => {
    try {
      const res = await fetch(`${API}/rooms/${roomId}/files`);
      if (res.ok) {
        const data = await res.json();
        setFiles(data);
      }
    } catch (e) { /* ignore */ }
  }, [setFiles]);

  const fetchReactions = useCallback(async (roomId) => {
    try {
      const res = await fetch(`${API}/rooms/${roomId}/reactions`);
      if (res.ok) {
        const data = await res.json();
        setReactions(data.reactions || {});
      }
    } catch (e) { /* ignore */ }
  }, [setReactions]);

  const fetchUnread = useCallback(async () => {
    if (!senderRef.current) return;
    try {
      const res = await fetch(`${API}/unread?sender=${encodeURIComponent(senderRef.current)}`);
      if (res.ok) {
        const data = await res.json();
        const counts = {};
        for (const room of data.rooms) {
          if (room.unread_count > 0) {
            counts[room.room_id] = room.unread_count;
          }
        }
        setUnreadCounts(counts);
      }
    } catch (e) { /* ignore */ }
  }, [senderRef, setUnreadCounts]);

  const fetchDmConversations = useCallback(async () => {
    if (!senderRef.current) return;
    try {
      const res = await fetch(`${API}/dm?sender=${encodeURIComponent(senderRef.current)}`);
      if (res.ok) {
        const data = await res.json();
        setDmConversations(data.conversations || []);
      }
    } catch (e) { /* ignore */ }
  }, [senderRef, setDmConversations]);

  const loadOlderMessages = useCallback(async (activeRoom, messages) => {
    if (!activeRoom || messages.length === 0) return;
    const oldestSeq = messages[0].seq;
    if (!oldestSeq) return;
    try {
      const res = await fetch(`${API}/rooms/${activeRoom.id}/messages?before_seq=${oldestSeq}&limit=${LOAD_MORE_LIMIT}`);
      if (res.ok) {
        const older = await res.json();
        if (older.length > 0) {
          setMessages(prev => [...older, ...prev]);
        }
        setHasMore(older.length >= LOAD_MORE_LIMIT);
      }
    } catch (e) { /* ignore */ }
  }, [setMessages]);

  const markRoomRead = useCallback((roomId, seq) => {
    if (!roomId || !senderRef.current) return;
    const effectiveSeq = seq || lastSeqRef.current;
    if (!effectiveSeq || effectiveSeq <= 0) return;
    // Optimistically clear unread for this room
    setUnreadCounts(prev => {
      const next = { ...prev };
      delete next[roomId];
      return next;
    });
    // Update server
    fetch(`${API}/rooms/${roomId}/read`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ sender: senderRef.current, last_read_seq: effectiveSeq }),
    }).catch(() => { /* ignore */ });
  }, [senderRef, lastSeqRef, setUnreadCounts]);

  // --- Mutations ---

  const handleCreateRoom = useCallback(async (name, description, sender) => {
    try {
      const res = await fetch(`${API}/rooms`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, description, created_by: sender }),
      });
      if (res.ok) {
        const room = await res.json();
        await fetchRooms();
        setActiveRoom(room);
        if (room.admin_key) {
          saveAdminKey(room.id, room.admin_key);
          setAdminKeyInfo({ roomName: room.name, adminKey: room.admin_key });
        }
      }
    } catch (e) { /* ignore */ }
  }, [fetchRooms, setActiveRoom, saveAdminKey, setAdminKeyInfo]);

  const toggleBookmark = useCallback(async (roomId, isCurrentlyBookmarked) => {
    const sender = senderRef.current;
    if (!sender) return;
    try {
      if (isCurrentlyBookmarked) {
        await fetch(`${API}/rooms/${roomId}/bookmark?sender=${encodeURIComponent(sender)}`, {
          method: 'DELETE',
        });
      } else {
        await fetch(`${API}/rooms/${roomId}/bookmark`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ sender }),
        });
      }
      await fetchRooms();
    } catch (e) { /* ignore */ }
  }, [fetchRooms, senderRef]);

  const handleSend = useCallback(async (activeRoom, sender, senderType, content, replyToId, isDmView) => {
    if (!activeRoom) return;
    try {
      const body = { sender, content, sender_type: senderType, metadata: { sender_type: senderType } };
      if (replyToId) body.reply_to = replyToId;
      const res = await fetch(`${API}/rooms/${activeRoom.id}/messages`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (res.ok) {
        fetchRooms();
        if (isDmView) fetchDmConversations();
      }
    } catch (e) { /* ignore */ }
  }, [fetchRooms, fetchDmConversations]);

  const handleEditMessage = useCallback(async (activeRoom, sender, messageId, newContent) => {
    if (!activeRoom) return;
    try {
      const res = await fetch(`${API}/rooms/${activeRoom.id}/messages/${messageId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ sender, content: newContent }),
      });
      if (res.ok) {
        const updated = await res.json();
        setMessages(prev => prev.map(m => m.id === updated.id ? { ...m, content: updated.content, edited_at: updated.edited_at } : m));
      }
    } catch (e) { /* ignore */ }
  }, [setMessages]);

  const handleDeleteMessage = useCallback(async (activeRoom, sender, messageId) => {
    if (!activeRoom) return;
    try {
      const res = await fetch(`${API}/rooms/${activeRoom.id}/messages/${messageId}?sender=${encodeURIComponent(sender)}`, {
        method: 'DELETE',
      });
      if (res.ok) {
        setMessages(prev => prev.filter(m => m.id !== messageId));
      }
    } catch (e) { /* ignore */ }
  }, [setMessages]);

  const handleUploadFile = useCallback(async (activeRoom, sender, filename, contentType, base64Data) => {
    if (!activeRoom) return;
    try {
      const res = await fetch(`${API}/rooms/${activeRoom.id}/files`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ sender, filename, content_type: contentType, data: base64Data }),
      });
      if (res.ok) {
        fetchRooms();
      }
    } catch (e) { /* ignore */ }
  }, [fetchRooms]);

  const handleDeleteFile = useCallback(async (activeRoom, sender, fileId) => {
    if (!activeRoom) return;
    try {
      const res = await fetch(`${API}/rooms/${activeRoom.id}/files/${fileId}?sender=${encodeURIComponent(sender)}`, {
        method: 'DELETE',
      });
      if (res.ok) {
        setFiles(prev => prev.filter(f => f.id !== fileId));
      }
    } catch (e) { /* ignore */ }
  }, [setFiles]);

  const handleToggleReaction = useCallback(async (activeRoom, sender, messageId, emoji) => {
    if (!activeRoom) return;
    try {
      await fetch(`${API}/rooms/${activeRoom.id}/messages/${messageId}/reactions`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ sender, emoji }),
      });
    } catch (e) { /* ignore */ }
  }, []);

  const handlePinMessage = useCallback(async (activeRoom, adminKeys, messageId) => {
    if (!activeRoom) return;
    let key = adminKeys[activeRoom.id];
    if (!key) {
      key = window.prompt('Enter the admin key for this room to pin/unpin messages:');
      if (!key) return;
      key = key.trim();
    }
    try {
      const res = await fetch(`${API}/rooms/${activeRoom.id}/messages/${messageId}/pin`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${key}` },
      });
      if (res.ok) {
        saveAdminKey(activeRoom.id, key);
      } else if (res.status === 403) {
        alert('Invalid admin key');
      }
      // 409 = already pinned, no-op
    } catch (e) { /* ignore */ }
  }, [saveAdminKey]);

  const handleUnpinMessage = useCallback(async (activeRoom, adminKeys, messageId) => {
    if (!activeRoom) return;
    let key = adminKeys[activeRoom.id];
    if (!key) {
      key = window.prompt('Enter the admin key for this room to pin/unpin messages:');
      if (!key) return;
      key = key.trim();
    }
    try {
      const res = await fetch(`${API}/rooms/${activeRoom.id}/messages/${messageId}/pin`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${key}` },
      });
      if (res.ok) {
        saveAdminKey(activeRoom.id, key);
      } else if (res.status === 403) {
        alert('Invalid admin key');
      }
    } catch (e) { /* ignore */ }
  }, [saveAdminKey]);

  const handleTyping = useCallback(async (activeRoom, sender, lastTypingSentRef) => {
    if (!activeRoom) return;
    const now = Date.now();
    if (now - lastTypingSentRef.current < 3000) return;
    lastTypingSentRef.current = now;
    try {
      await fetch(`${API}/rooms/${activeRoom.id}/typing`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ sender }),
      });
    } catch (e) { /* ignore */ }
  }, []);

  const handleStartDm = useCallback(async (sender, senderType, recipient, content) => {
    try {
      const res = await fetch(`${API}/dm`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ sender, recipient, content, sender_type: senderType }),
      });
      if (res.ok) {
        const data = await res.json();
        await fetchDmConversations();
        return { room_id: data.room_id, recipient };
      }
    } catch (e) { /* ignore */ }
    return null;
  }, [fetchDmConversations]);

  const handleRoomUpdate = useCallback((updated) => {
    setRooms(prev => prev.map(r => r.id === updated.id ? { ...r, ...updated } : r));
    setActiveRoom(prev => prev && prev.id === updated.id ? { ...prev, ...updated } : prev);
  }, [setRooms, setActiveRoom]);

  const handleRoomArchived = useCallback((updated) => {
    if (updated.archived_at) {
      setRooms(prev => prev.filter(r => r.id !== updated.id));
      setActiveRoom(prev => prev && prev.id === updated.id ? null : prev);
    } else {
      setRooms(prev => {
        if (prev.find(r => r.id === updated.id)) return prev.map(r => r.id === updated.id ? { ...r, ...updated } : r);
        return [...prev, updated];
      });
    }
  }, [setRooms, setActiveRoom]);

  return {
    loading,
    hasMore,
    fetchRooms,
    fetchMessages,
    fetchFiles,
    fetchReactions,
    fetchUnread,
    fetchDmConversations,
    loadOlderMessages,
    markRoomRead,
    handleCreateRoom,
    toggleBookmark,
    handleSend,
    handleEditMessage,
    handleDeleteMessage,
    handleUploadFile,
    handleDeleteFile,
    handleToggleReaction,
    handlePinMessage,
    handleUnpinMessage,
    handleTyping,
    handleStartDm,
    handleRoomUpdate,
    handleRoomArchived,
  };
}
