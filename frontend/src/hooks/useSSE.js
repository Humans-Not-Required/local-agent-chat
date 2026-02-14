import { useCallback, useRef, useState } from 'react';
import { API, playNotificationSound } from '../utils';

/**
 * useSSE — Manages SSE connection to a chat room.
 *
 * Handles: connection lifecycle, reconnection with backoff,
 * all real-time event dispatch (messages, reactions, presence, typing, etc.),
 * debounced read position updates, and initial presence fetch.
 */
export default function useSSE({
  senderRef,
  soundEnabledRef,
  lastSeqRef,
  activeRoomRef,
  typingTimeoutsRef,
  setMessages,
  setFiles,
  setReactions,
  setOnlineUsers,
  setTypingUsers,
  setRooms,
  setActiveRoom,
  setProfiles,
  markRoomRead,
}) {
  const [connected, setConnected] = useState(false);
  const eventSourceRef = useRef(null);
  const reconnectAttemptsRef = useRef(0);
  const reconnectTimerRef = useRef(null);
  const readPositionTimerRef = useRef(null);

  const connect = useCallback((roomId) => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
      eventSourceRef.current = null;
    }
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }

    const params = new URLSearchParams();
    if (lastSeqRef.current) params.set('after', lastSeqRef.current);
    if (senderRef.current) params.set('sender', senderRef.current);
    const st = localStorage.getItem('chat-sender-type');
    if (st) params.set('sender_type', st);
    const paramStr = params.toString();
    const es = new EventSource(`${API}/rooms/${roomId}/stream${paramStr ? '?' + paramStr : ''}`);

    // Fetch initial presence for this room
    fetch(`${API}/rooms/${roomId}/presence`)
      .then(r => r.ok ? r.json() : null)
      .then(data => { if (data) setOnlineUsers(data.online || []); })
      .catch(() => {});

    es.addEventListener('message', (e) => {
      try {
        const msg = JSON.parse(e.data);
        if (msg.sender !== senderRef.current && soundEnabledRef.current && document.hidden) {
          playNotificationSound();
        }
        setMessages(prev => {
          if (prev.some(m => m.id === msg.id)) return prev;
          if (msg.seq) lastSeqRef.current = msg.seq;
          return [...prev, msg];
        });
        // Debounce read position update — marks room as read 1s after last message arrives
        if (!document.hidden && msg.seq) {
          if (readPositionTimerRef.current) clearTimeout(readPositionTimerRef.current);
          readPositionTimerRef.current = setTimeout(() => {
            const ar = activeRoomRef.current;
            if (ar && ar.id === roomId && lastSeqRef.current) {
              markRoomRead(roomId, lastSeqRef.current);
            }
          }, 1000);
        }
        setTypingUsers(prev => prev.filter(s => s !== msg.sender));
        if (typingTimeoutsRef.current[msg.sender]) {
          clearTimeout(typingTimeoutsRef.current[msg.sender]);
          delete typingTimeoutsRef.current[msg.sender];
        }
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('message_edited', (e) => {
      try {
        const updated = JSON.parse(e.data);
        setMessages(prev => prev.map(m => m.id === updated.id ? { ...m, content: updated.content, edited_at: updated.edited_at } : m));
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('message_deleted', (e) => {
      try {
        const { id } = JSON.parse(e.data);
        setMessages(prev => prev.filter(m => m.id !== id));
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('file_uploaded', (e) => {
      try {
        const file = JSON.parse(e.data);
        setFiles(prev => {
          if (prev.some(f => f.id === file.id)) return prev;
          return [...prev, file];
        });
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('file_deleted', (e) => {
      try {
        const { id } = JSON.parse(e.data);
        setFiles(prev => prev.filter(f => f.id !== id));
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('reaction_added', (e) => {
      try {
        const r = JSON.parse(e.data);
        setReactions(prev => {
          const msgReactions = [...(prev[r.message_id] || [])];
          const existing = msgReactions.find(x => x.emoji === r.emoji);
          if (existing) {
            if (!existing.senders.includes(r.sender)) {
              existing.senders = [...existing.senders, r.sender];
              existing.count = existing.senders.length;
            }
          } else {
            msgReactions.push({ emoji: r.emoji, count: 1, senders: [r.sender] });
          }
          return { ...prev, [r.message_id]: msgReactions };
        });
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('reaction_removed', (e) => {
      try {
        const r = JSON.parse(e.data);
        setReactions(prev => {
          const msgReactions = [...(prev[r.message_id] || [])];
          const existing = msgReactions.find(x => x.emoji === r.emoji);
          if (existing) {
            existing.senders = existing.senders.filter(s => s !== r.sender);
            existing.count = existing.senders.length;
          }
          const filtered = msgReactions.filter(x => x.count > 0);
          if (filtered.length === 0) {
            const next = { ...prev };
            delete next[r.message_id];
            return next;
          }
          return { ...prev, [r.message_id]: filtered };
        });
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('message_pinned', (e) => {
      try {
        const pinned = JSON.parse(e.data);
        setMessages(prev => prev.map(m => m.id === pinned.id ? { ...m, pinned_at: pinned.pinned_at, pinned_by: pinned.pinned_by } : m));
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('message_unpinned', (e) => {
      try {
        const { id } = JSON.parse(e.data);
        setMessages(prev => prev.map(m => m.id === id ? { ...m, pinned_at: null, pinned_by: null } : m));
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('presence_joined', (e) => {
      try {
        const { sender: joinedSender, sender_type: joinedType } = JSON.parse(e.data);
        setOnlineUsers(prev => {
          if (prev.some(u => u.sender === joinedSender)) return prev;
          return [...prev, { sender: joinedSender, sender_type: joinedType, connected_at: new Date().toISOString() }];
        });
      } catch { /* ignore */ }
    });

    es.addEventListener('presence_left', (e) => {
      try {
        const { sender: leftSender } = JSON.parse(e.data);
        setOnlineUsers(prev => prev.filter(u => u.sender !== leftSender));
      } catch { /* ignore */ }
    });

    es.addEventListener('room_updated', (e) => {
      try {
        const updated = JSON.parse(e.data);
        setRooms(prev => prev.map(r => r.id === updated.id ? { ...r, ...updated } : r));
        setActiveRoom(prev => prev && prev.id === updated.id ? { ...prev, ...updated } : prev);
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('room_archived', (e) => {
      try {
        const room = JSON.parse(e.data);
        setRooms(prev => prev.filter(r => r.id !== room.id));
        setActiveRoom(prev => prev && prev.id === room.id ? null : prev);
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('room_unarchived', (e) => {
      try {
        const room = JSON.parse(e.data);
        setRooms(prev => {
          if (prev.find(r => r.id === room.id)) return prev;
          return [...prev, room];
        });
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('profile_updated', (e) => {
      try {
        const profile = JSON.parse(e.data);
        setProfiles(prev => ({ ...prev, [profile.sender]: profile }));
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('profile_deleted', (e) => {
      try {
        const { sender: deletedSender } = JSON.parse(e.data);
        setProfiles(prev => {
          const next = { ...prev };
          delete next[deletedSender];
          return next;
        });
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('typing', (e) => {
      try {
        const { sender: typingSender } = JSON.parse(e.data);
        if (typingSender === senderRef.current) return;

        setTypingUsers(prev => {
          if (prev.includes(typingSender)) return prev;
          return [...prev, typingSender];
        });

        if (typingTimeoutsRef.current[typingSender]) {
          clearTimeout(typingTimeoutsRef.current[typingSender]);
        }

        typingTimeoutsRef.current[typingSender] = setTimeout(() => {
          setTypingUsers(prev => prev.filter(s => s !== typingSender));
          delete typingTimeoutsRef.current[typingSender];
        }, 4000);
      } catch (err) { /* ignore */ }
    });

    es.addEventListener('heartbeat', () => {
      setConnected(true);
    });

    es.onopen = () => {
      setConnected(true);
      reconnectAttemptsRef.current = 0;
    };
    es.onerror = () => {
      setConnected(false);
      // Close stale EventSource and manually reconnect with updated cursor
      es.close();
      eventSourceRef.current = null;
      const attempt = reconnectAttemptsRef.current++;
      const delay = Math.min(1000 * Math.pow(2, attempt), 30000); // 1s → 2s → 4s → ... → 30s max
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = setTimeout(() => {
        if (activeRoomRef.current?.id === roomId) {
          connect(roomId);
        }
      }, delay);
    };

    eventSourceRef.current = es;
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const disconnect = useCallback(() => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
      eventSourceRef.current = null;
    }
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
    reconnectAttemptsRef.current = 0;
    if (readPositionTimerRef.current) {
      clearTimeout(readPositionTimerRef.current);
      readPositionTimerRef.current = null;
    }
  }, []);

  return { connected, connect, disconnect };
}
