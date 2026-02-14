import React, { useState, useEffect, useRef, useCallback } from 'react';
import { styles, injectGlobalStyles } from './styles';
import { API, senderColor, playNotificationSound } from './utils';
import { RoomList, ChatArea, SenderModal, AdminKeyModal, ProfileModal } from './components';

// Inject global CSS on load
injectGlobalStyles();

export default function App() {
  const [sender, setSender] = useState(() => localStorage.getItem('chat-sender') || '');
  const [senderType, setSenderType] = useState(() => localStorage.getItem('chat-sender-type') || 'agent');
  const [rooms, setRooms] = useState([]);
  const [activeRoom, setActiveRoom] = useState(null);
  const [messages, setMessages] = useState([]);
  const [files, setFiles] = useState([]);
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(false);
  const [reactions, setReactions] = useState({});
  const [adminKeyInfo, setAdminKeyInfo] = useState(null);
  const [adminKeys, setAdminKeys] = useState(() => {
    try { return JSON.parse(localStorage.getItem('chat-admin-keys') || '{}'); } catch { return {}; }
  });
  const [connected, setConnected] = useState(false);
  const [showSidebar, setShowSidebar] = useState(window.innerWidth > 768);
  const [typingUsers, setTypingUsers] = useState([]);
  const [unreadCounts, setUnreadCounts] = useState({});
  const [soundEnabled, setSoundEnabled] = useState(() => localStorage.getItem('chat-sound') !== 'off');
  const [onlineUsers, setOnlineUsers] = useState([]);
  const [showProfileModal, setShowProfileModal] = useState(false);
  const senderRef = useRef(sender);
  const soundEnabledRef = useRef(soundEnabled);
  const eventSourceRef = useRef(null);
  const lastSeqRef = useRef(null);
  const typingTimeoutsRef = useRef({});
  const lastTypingSentRef = useRef(0);
  const readPositionTimerRef = useRef(null);
  const activeRoomRef = useRef(null);

  const saveAdminKey = useCallback((roomId, key) => {
    setAdminKeys(prev => {
      const next = { ...prev, [roomId]: key };
      try { localStorage.setItem('chat-admin-keys', JSON.stringify(next)); } catch { /* ignore */ }
      return next;
    });
  }, []);

  useEffect(() => { senderRef.current = sender; }, [sender]);
  useEffect(() => {
    soundEnabledRef.current = soundEnabled;
    localStorage.setItem('chat-sound', soundEnabled ? 'on' : 'off');
  }, [soundEnabled]);

  useEffect(() => {
    const total = Object.values(unreadCounts).reduce((sum, n) => sum + n, 0);
    document.title = total > 0 ? `(${total}) Local Agent Chat` : 'Local Agent Chat';
  }, [unreadCounts]);

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
  }, []);

  const fetchRooms = useCallback(async () => {
    try {
      const res = await fetch(`${API}/rooms`);
      if (res.ok) {
        const data = await res.json();
        setRooms(data);
        return data;
      }
    } catch (e) { /* ignore */ }
    return [];
  }, []);

  const INITIAL_LIMIT = 200;
  const LOAD_MORE_LIMIT = 50;

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
  }, []);

  const fetchFiles = useCallback(async (roomId) => {
    try {
      const res = await fetch(`${API}/rooms/${roomId}/files`);
      if (res.ok) {
        const data = await res.json();
        setFiles(data);
      }
    } catch (e) { /* ignore */ }
  }, []);

  const fetchReactions = useCallback(async (roomId) => {
    try {
      const res = await fetch(`${API}/rooms/${roomId}/reactions`);
      if (res.ok) {
        const data = await res.json();
        setReactions(data.reactions || {});
      }
    } catch (e) { /* ignore */ }
  }, []);

  const loadOlderMessages = useCallback(async () => {
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
  }, [activeRoom, messages]);

  const connectSSE = useCallback((roomId) => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
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

    es.addEventListener('typing', (e) => {
      try {
        const { sender: typingSender } = JSON.parse(e.data);
        if (typingSender === sender) return;

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

    es.onopen = () => setConnected(true);
    es.onerror = () => {
      setConnected(false);
    };

    eventSourceRef.current = es;
  }, []);

  useEffect(() => {
    fetchRooms().then(data => {
      if (data.length > 0 && !activeRoom) {
        const general = data.find(r => r.name === 'general') || data[0];
        setActiveRoom(general);
      }
    });
    fetchUnread();
    const roomInterval = setInterval(fetchRooms, 30000);
    const unreadInterval = setInterval(fetchUnread, 30000);
    return () => { clearInterval(roomInterval); clearInterval(unreadInterval); };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!activeRoom) return;
    activeRoomRef.current = activeRoom;
    lastSeqRef.current = null;
    setTypingUsers([]);
    setOnlineUsers([]);
    setFiles([]);
    Object.values(typingTimeoutsRef.current).forEach(clearTimeout);
    typingTimeoutsRef.current = {};
    setReactions({});
    Promise.all([
      fetchMessages(activeRoom.id),
      fetchFiles(activeRoom.id),
      fetchReactions(activeRoom.id),
    ]).then(() => {
      // Mark room as read after loading messages (lastSeqRef is set by fetchMessages)
      if (lastSeqRef.current) {
        markRoomRead(activeRoom.id, lastSeqRef.current);
      }
      connectSSE(activeRoom.id);
    });
    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
      if (readPositionTimerRef.current) {
        clearTimeout(readPositionTimerRef.current);
        readPositionTimerRef.current = null;
      }
    };
  }, [activeRoom?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    const handle = () => {
      if (window.innerWidth > 768) setShowSidebar(true);
    };
    window.addEventListener('resize', handle);

    // Mark room as read when tab becomes visible again
    const handleVisibility = () => {
      if (!document.hidden && activeRoomRef.current && lastSeqRef.current) {
        markRoomRead(activeRoomRef.current.id, lastSeqRef.current);
      }
    };
    document.addEventListener('visibilitychange', handleVisibility);

    return () => {
      window.removeEventListener('resize', handle);
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleTyping = useCallback(async () => {
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
  }, [activeRoom?.id, sender]);

  const handleSetSender = (name, type) => {
    localStorage.setItem('chat-sender', name);
    localStorage.setItem('chat-sender-type', type || 'agent');
    setSender(name);
    setSenderType(type || 'agent');
  };

  const handleRoomUpdate = useCallback((updated) => {
    setRooms(prev => prev.map(r => r.id === updated.id ? { ...r, ...updated } : r));
    setActiveRoom(prev => prev && prev.id === updated.id ? { ...prev, ...updated } : prev);
  }, []);

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
  }, []);

  const handleSelectRoom = (room) => {
    setActiveRoom(room);
    // Read position will be marked after messages are loaded (in useEffect)
    if (window.innerWidth <= 768) setShowSidebar(false);
  };

  const handleCreateRoom = async (name, description) => {
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
  };

  const handleSend = async (content, replyToId) => {
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
      }
    } catch (e) { /* ignore */ }
  };

  const handleEditMessage = async (messageId, newContent) => {
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
  };

  const handleUploadFile = async (filename, contentType, base64Data) => {
    if (!activeRoom) return;
    try {
      const res = await fetch(`${API}/rooms/${activeRoom.id}/files`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          sender,
          filename,
          content_type: contentType,
          data: base64Data,
        }),
      });
      if (res.ok) {
        fetchRooms();
      }
    } catch (e) { /* ignore */ }
  };

  const handleDeleteFile = async (fileId) => {
    if (!activeRoom) return;
    try {
      const res = await fetch(`${API}/rooms/${activeRoom.id}/files/${fileId}?sender=${encodeURIComponent(sender)}`, {
        method: 'DELETE',
      });
      if (res.ok) {
        setFiles(prev => prev.filter(f => f.id !== fileId));
      }
    } catch (e) { /* ignore */ }
  };

  const handleDeleteMessage = async (messageId) => {
    if (!activeRoom) return;
    try {
      const res = await fetch(`${API}/rooms/${activeRoom.id}/messages/${messageId}?sender=${encodeURIComponent(sender)}`, {
        method: 'DELETE',
      });
      if (res.ok) {
        setMessages(prev => prev.filter(m => m.id !== messageId));
      }
    } catch (e) { /* ignore */ }
  };

  const handleToggleReaction = async (messageId, emoji) => {
    if (!activeRoom) return;
    try {
      await fetch(`${API}/rooms/${activeRoom.id}/messages/${messageId}/reactions`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ sender, emoji }),
      });
    } catch (e) { /* ignore */ }
  };

  const getAdminKey = useCallback((roomId) => {
    if (adminKeys[roomId]) return adminKeys[roomId];
    const key = window.prompt('Enter the admin key for this room to pin/unpin messages:');
    if (!key) return null;
    return key.trim();
  }, [adminKeys]);

  const handlePinMessage = async (messageId) => {
    if (!activeRoom) return;
    const key = getAdminKey(activeRoom.id);
    if (!key) return;
    try {
      const res = await fetch(`${API}/rooms/${activeRoom.id}/messages/${messageId}/pin`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${key}` },
      });
      if (res.ok) {
        // Save the key on success
        saveAdminKey(activeRoom.id, key);
      } else if (res.status === 403) {
        alert('Invalid admin key');
      } else if (res.status === 409) {
        // Already pinned, no-op
      }
    } catch (e) { /* ignore */ }
  };

  const handleUnpinMessage = async (messageId) => {
    if (!activeRoom) return;
    const key = getAdminKey(activeRoom.id);
    if (!key) return;
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
  };

  if (!sender) {
    return <SenderModal onSet={handleSetSender} />;
  }

  return (
    <div style={styles.container}>
      {adminKeyInfo && (
        <AdminKeyModal
          roomName={adminKeyInfo.roomName}
          adminKey={adminKeyInfo.adminKey}
          onDismiss={() => setAdminKeyInfo(null)}
        />
      )}
      {showProfileModal && (
        <ProfileModal
          sender={sender}
          onClose={() => setShowProfileModal(false)}
        />
      )}
      {/* Mobile header */}
      <div className="chat-mobile-header" data-mobile-header style={styles.mobileHeader}>
        <button onClick={() => setShowSidebar(!showSidebar)} style={styles.iconBtn}>
          {showSidebar ? '✕' : '☰'}
        </button>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, flex: 1, minWidth: 0 }}>
          <span style={{ fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {activeRoom ? `#${activeRoom.name}` : 'Local Agent Chat'}
          </span>
          {connected && (
            <div style={{ width: 6, height: 6, borderRadius: '50%', background: '#34d399', flexShrink: 0 }} />
          )}
        </div>
        <span style={{ fontSize: '0.7rem', color: senderColor(sender), whiteSpace: 'nowrap', flexShrink: 0 }}>
          {senderType === 'human' ? '👤' : '🤖'} {sender}
        </span>
      </div>

      <div style={styles.main}>
        {showSidebar && (
          <>
            <div
              className="chat-sidebar-backdrop"
              onClick={() => { if (window.innerWidth <= 768) setShowSidebar(false); }}
            />
            <RoomList
              rooms={rooms}
              activeRoom={activeRoom}
              onSelect={handleSelectRoom}
              onCreateRoom={handleCreateRoom}
              unreadCounts={unreadCounts}
              sender={sender}
              senderType={senderType}
              onChangeSender={() => {
                localStorage.removeItem('chat-sender');
                localStorage.removeItem('chat-sender-type');
                setSender('');
                setSenderType('agent');
              }}
              onEditProfile={() => setShowProfileModal(true)}
            />
          </>
        )}
        <ChatArea
          room={activeRoom}
          messages={messages}
          files={files}
          sender={sender}
          reactions={reactions}
          onSend={handleSend}
          onEditMessage={handleEditMessage}
          onDeleteMessage={handleDeleteMessage}
          onDeleteFile={handleDeleteFile}
          onUploadFile={handleUploadFile}
          onReact={handleToggleReaction}
          onPin={handlePinMessage}
          onUnpin={handleUnpinMessage}
          adminKey={activeRoom ? adminKeys[activeRoom.id] : null}
          onTyping={handleTyping}
          typingUsers={typingUsers}
          loading={loading}
          connected={connected}
          rooms={rooms}
          onSelectRoom={handleSelectRoom}
          onRoomUpdate={handleRoomUpdate}
          soundEnabled={soundEnabled}
          onToggleSound={() => setSoundEnabled(prev => !prev)}
          hasMore={hasMore}
          onLoadOlder={loadOlderMessages}
          onlineUsers={onlineUsers}
        />
      </div>
      <footer style={{ textAlign: 'center', padding: '4px 16px', fontSize: '0.6rem', color: '#475569', flexShrink: 0, borderTop: '1px solid #1e293b' }}>
        Made for AI, by AI.{' '}
        <a href="https://github.com/Humans-Not-Required" target="_blank" rel="noopener noreferrer" style={{ color: '#6366f1', textDecoration: 'none' }}>Humans not required</a>.
      </footer>
    </div>
  );
}
