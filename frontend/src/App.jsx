import React, { useState, useEffect, useRef, useCallback } from 'react';

const API = '/api/v1';

// --- Helpers ---

function timeAgo(dateStr) {
  if (!dateStr) return '';
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  return `${days}d ago`;
}

function formatTime(dateStr) {
  if (!dateStr) return '';
  const d = new Date(dateStr);
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function formatDate(dateStr) {
  if (!dateStr) return '';
  const d = new Date(dateStr);
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);
  if (d.toDateString() === today.toDateString()) return 'Today';
  if (d.toDateString() === yesterday.toDateString()) return 'Yesterday';
  return d.toLocaleDateString([], { month: 'short', day: 'numeric', year: 'numeric' });
}

// Generate a consistent color for a sender name
function senderColor(name) {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  const colors = ['#60a5fa', '#f472b6', '#34d399', '#fbbf24', '#a78bfa', '#fb923c', '#22d3ee', '#e879f9'];
  return colors[Math.abs(hash) % colors.length];
}

// --- Components ---

function RoomList({ rooms, activeRoom, onSelect, onCreateRoom, unreadCounts }) {
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState('');
  const [newDesc, setNewDesc] = useState('');

  const handleCreate = async (e) => {
    e.preventDefault();
    if (!newName.trim()) return;
    await onCreateRoom(newName.trim(), newDesc.trim());
    setNewName('');
    setNewDesc('');
    setCreating(false);
  };

  return (
    <div style={styles.sidebar}>
      <div style={styles.sidebarHeader}>
        <h2 style={{ fontSize: '1rem', fontWeight: 600 }}>💬 Rooms</h2>
        <button onClick={() => setCreating(!creating)} style={styles.iconBtn} title="Create room">+</button>
      </div>
      {creating && (
        <form onSubmit={handleCreate} style={styles.createForm}>
          <input
            value={newName}
            onChange={e => setNewName(e.target.value)}
            placeholder="Room name"
            style={styles.input}
            autoFocus
          />
          <input
            value={newDesc}
            onChange={e => setNewDesc(e.target.value)}
            placeholder="Description (optional)"
            style={styles.input}
          />
          <div style={{ display: 'flex', gap: 6 }}>
            <button type="submit" style={styles.btnPrimary}>Create</button>
            <button type="button" onClick={() => setCreating(false)} style={styles.btnSecondary}>Cancel</button>
          </div>
        </form>
      )}
      <div style={styles.roomList}>
        {rooms.map(room => {
          const unread = unreadCounts[room.id] || 0;
          return (
            <div
              key={room.id}
              onClick={() => onSelect(room)}
              style={{
                ...styles.roomItem,
                background: activeRoom?.id === room.id ? '#1e293b' : 'transparent',
                borderLeft: activeRoom?.id === room.id ? '3px solid #3b82f6' : '3px solid transparent',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <div style={{ fontWeight: unread > 0 ? 700 : 500, color: activeRoom?.id === room.id ? '#f1f5f9' : unread > 0 ? '#f1f5f9' : '#cbd5e1' }}>
                  #{room.name}
                </div>
                {unread > 0 && (
                  <span style={styles.unreadBadge}>
                    {unread > 99 ? '99+' : unread}
                  </span>
                )}
              </div>
              <div style={{ fontSize: '0.75rem', color: '#64748b', marginTop: 2 }}>
                {room.message_count || 0} msgs
                {room.last_activity && ` · ${timeAgo(room.last_activity)}`}
              </div>
            </div>
          );
        })}
        {rooms.length === 0 && (
          <div style={{ padding: '16px', color: '#64748b', fontSize: '0.85rem' }}>No rooms yet</div>
        )}
      </div>
    </div>
  );
}

function ReplyPreview({ replyToId, messages, style: extraStyle }) {
  if (!replyToId) return null;
  const original = messages.find(m => m.id === replyToId);
  if (!original) return null;

  const preview = original.content.length > 80 ? original.content.slice(0, 80) + '…' : original.content;
  return (
    <div style={{ ...styles.replyPreview, ...extraStyle }}>
      <div style={{ width: 3, background: senderColor(original.sender), borderRadius: 2, flexShrink: 0 }} />
      <div style={{ overflow: 'hidden' }}>
        <div style={{ fontSize: '0.7rem', fontWeight: 600, color: senderColor(original.sender) }}>{original.sender}</div>
        <div style={{ fontSize: '0.75rem', color: '#94a3b8', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{preview}</div>
      </div>
    </div>
  );
}

function MessageBubble({ msg, isOwn, onEdit, onDelete, onReply, allMessages }) {
  const [showActions, setShowActions] = useState(false);
  const [editing, setEditing] = useState(false);
  const [editText, setEditText] = useState(msg.content);

  const handleSaveEdit = () => {
    const trimmed = editText.trim();
    if (trimmed && trimmed !== msg.content) {
      onEdit(msg.id, trimmed);
    }
    setEditing(false);
  };

  const handleEditKeyDown = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSaveEdit();
    }
    if (e.key === 'Escape') {
      setEditText(msg.content);
      setEditing(false);
    }
  };

  // Toggle actions on click (mobile-friendly) or show on hover (desktop)
  const handleBubbleClick = (e) => {
    if (!editing) {
      // Don't toggle if clicking inside action buttons
      if (e.target.closest('[data-actions]')) return;
      setShowActions(prev => !prev);
    }
  };

  return (
    <div
      style={{
        position: 'relative',
        alignSelf: isOwn ? 'flex-end' : 'flex-start',
        maxWidth: '75%',
      }}
      onMouseEnter={() => setShowActions(true)}
      onMouseLeave={() => setShowActions(false)}
    >
      {/* Action buttons on hover/tap */}
      {showActions && !editing && (
        <div style={styles.msgActions} data-actions>
          <button
            onClick={(e) => { e.stopPropagation(); onReply(msg); setShowActions(false); }}
            style={styles.msgActionBtn}
            title="Reply"
          >↩</button>
          {isOwn && (
            <>
              <button
                onClick={(e) => { e.stopPropagation(); setEditText(msg.content); setEditing(true); setShowActions(false); }}
                style={styles.msgActionBtn}
                title="Edit"
              >✎</button>
              <button
                onClick={(e) => { e.stopPropagation(); onDelete(msg.id); }}
                style={{ ...styles.msgActionBtn, color: '#ef4444' }}
                title="Delete"
              >✕</button>
            </>
          )}
        </div>
      )}
      <div
        onClick={handleBubbleClick}
        style={{
          ...styles.messageBubble,
          background: isOwn ? '#1e3a5f' : '#1e293b',
          borderRadius: isOwn ? '12px 12px 4px 12px' : '12px 12px 12px 4px',
          cursor: !editing ? 'pointer' : 'default',
        }}
      >
        {editing ? (
          <div>
            <textarea
              value={editText}
              onChange={e => setEditText(e.target.value)}
              onKeyDown={handleEditKeyDown}
              style={styles.editInput}
              autoFocus
              rows={2}
            />
            <div style={{ display: 'flex', gap: 6, marginTop: 6, justifyContent: 'flex-end' }}>
              <button onClick={() => { setEditText(msg.content); setEditing(false); }} style={styles.editCancelBtn}>Cancel</button>
              <button onClick={handleSaveEdit} style={styles.editSaveBtn}>Save</button>
            </div>
          </div>
        ) : (
          <>
            {msg.reply_to && <ReplyPreview replyToId={msg.reply_to} messages={allMessages} />}
            <div style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>{msg.content}</div>
            <div style={{ fontSize: '0.7rem', color: '#64748b', marginTop: 4, textAlign: 'right', display: 'flex', justifyContent: 'flex-end', gap: 6, alignItems: 'center' }}>
              {msg.edited_at && <span style={{ fontStyle: 'italic' }}>(edited)</span>}
              {formatTime(msg.created_at)}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function MessageGroup({ messages, isOwn, onEdit, onDelete, onReply, allMessages }) {
  const sender = messages[0].sender;
  const color = senderColor(sender);

  return (
    <div style={{ marginBottom: 16, display: 'flex', flexDirection: 'column', alignItems: isOwn ? 'flex-end' : 'flex-start' }}>
      <div style={{ fontSize: '0.8rem', fontWeight: 600, color, marginBottom: 4, paddingLeft: isOwn ? 0 : 4, paddingRight: isOwn ? 4 : 0 }}>
        {sender}
      </div>
      {messages.map(msg => (
        <MessageBubble
          key={msg.id}
          msg={msg}
          isOwn={isOwn}
          onEdit={onEdit}
          onDelete={onDelete}
          onReply={onReply}
          allMessages={allMessages}
        />
      ))}
    </div>
  );
}

function DateSeparator({ date }) {
  return (
    <div style={styles.dateSeparator}>
      <span style={styles.dateLine} />
      <span style={styles.dateLabel}>{date}</span>
      <span style={styles.dateLine} />
    </div>
  );
}

function TypingIndicator({ typingUsers }) {
  if (typingUsers.length === 0) return null;

  let text;
  if (typingUsers.length === 1) {
    text = `${typingUsers[0]} is typing`;
  } else if (typingUsers.length === 2) {
    text = `${typingUsers[0]} and ${typingUsers[1]} are typing`;
  } else {
    text = `${typingUsers[0]} and ${typingUsers.length - 1} others are typing`;
  }

  return (
    <div style={styles.typingIndicator}>
      <span style={styles.typingDots}>
        <span style={styles.typingDot}>•</span>
        <span style={{ ...styles.typingDot, animationDelay: '0.2s' }}>•</span>
        <span style={{ ...styles.typingDot, animationDelay: '0.4s' }}>•</span>
      </span>
      <span>{text}</span>
    </div>
  );
}

function ChatArea({ room, messages, sender, onSend, onEditMessage, onDeleteMessage, onTyping, typingUsers, loading, connected }) {
  const [text, setText] = useState('');
  const [replyTo, setReplyTo] = useState(null); // { id, sender, content }
  const messagesEndRef = useRef(null);
  const containerRef = useRef(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const inputRef = useRef(null);

  // Clear reply state when room changes
  useEffect(() => {
    setReplyTo(null);
  }, [room?.id]);

  useEffect(() => {
    if (autoScroll) {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages, autoScroll]);

  const handleScroll = () => {
    const el = containerRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 60;
    setAutoScroll(atBottom);
  };

  const handleSubmit = (e) => {
    e.preventDefault();
    if (!text.trim()) return;
    onSend(text.trim(), replyTo?.id || null);
    setText('');
    setReplyTo(null);
  };

  const handleReply = (msg) => {
    setReplyTo({ id: msg.id, sender: msg.sender, content: msg.content });
    inputRef.current?.focus();
  };

  const handleTextChange = (e) => {
    setText(e.target.value);
    if (e.target.value.trim()) {
      onTyping();
    }
  };

  const handleKeyDown = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
    if (e.key === 'Escape' && replyTo) {
      setReplyTo(null);
    }
  };

  if (!room) {
    return (
      <div style={styles.chatArea}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: '#64748b' }}>
          <div style={{ textAlign: 'center' }}>
            <div style={{ fontSize: '2.5rem', marginBottom: 12 }}>💬</div>
            <div style={{ fontSize: '1.1rem', fontWeight: 500 }}>Local Agent Chat</div>
            <div style={{ fontSize: '0.85rem', marginTop: 8 }}>Select a room to start chatting</div>
          </div>
        </div>
      </div>
    );
  }

  // Group consecutive messages by sender and date
  const grouped = [];
  let currentGroup = null;
  let currentDate = null;

  for (const msg of messages) {
    const msgDate = formatDate(msg.created_at);
    if (msgDate !== currentDate) {
      if (currentGroup) grouped.push(currentGroup);
      currentGroup = null;
      currentDate = msgDate;
      grouped.push({ type: 'date', date: msgDate });
    }
    if (currentGroup && currentGroup.sender === msg.sender) {
      currentGroup.messages.push(msg);
    } else {
      if (currentGroup) grouped.push(currentGroup);
      currentGroup = { type: 'messages', sender: msg.sender, messages: [msg] };
    }
  }
  if (currentGroup) grouped.push(currentGroup);

  return (
    <div style={styles.chatArea}>
      {/* Header */}
      <div style={styles.chatHeader}>
        <div>
          <span style={{ fontWeight: 600, fontSize: '1rem' }}>#{room.name}</span>
          {room.description && (
            <span style={{ color: '#64748b', fontSize: '0.85rem', marginLeft: 12 }}>{room.description}</span>
          )}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <div style={{
            width: 8, height: 8, borderRadius: '50%',
            background: connected ? '#34d399' : '#ef4444',
          }} />
          <span style={{ fontSize: '0.75rem', color: '#64748b' }}>
            {connected ? 'Live' : 'Reconnecting...'}
          </span>
        </div>
      </div>

      {/* Messages */}
      <div ref={containerRef} onScroll={handleScroll} style={styles.messageContainer}>
        {loading && (
          <div style={{ textAlign: 'center', padding: 20, color: '#64748b' }}>Loading messages...</div>
        )}
        {!loading && messages.length === 0 && (
          <div style={{ textAlign: 'center', padding: 40, color: '#64748b' }}>
            <div style={{ fontSize: '1.5rem', marginBottom: 8 }}>🎉</div>
            <div>No messages yet. Be the first to say something!</div>
          </div>
        )}
        {grouped.map((item, i) => {
          if (item.type === 'date') {
            return <DateSeparator key={`date-${i}`} date={item.date} />;
          }
          return (
            <MessageGroup
              key={`group-${i}`}
              messages={item.messages}
              isOwn={item.sender === sender}
              onEdit={onEditMessage}
              onDelete={onDeleteMessage}
              onReply={handleReply}
              allMessages={messages}
            />
          );
        })}
        <div ref={messagesEndRef} />
      </div>

      {/* Typing indicator */}
      <TypingIndicator typingUsers={typingUsers} />

      {/* Scroll to bottom button */}
      {!autoScroll && (
        <button
          onClick={() => {
            messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
            setAutoScroll(true);
          }}
          style={styles.scrollBtn}
        >
          ↓ New messages
        </button>
      )}

      {/* Reply bar */}
      {replyTo && (
        <div style={styles.replyBar}>
          <div style={{ width: 3, background: senderColor(replyTo.sender), borderRadius: 2, flexShrink: 0 }} />
          <div style={{ flex: 1, overflow: 'hidden' }}>
            <span style={{ fontSize: '0.75rem', fontWeight: 600, color: senderColor(replyTo.sender) }}>
              Replying to {replyTo.sender}
            </span>
            <div style={{ fontSize: '0.75rem', color: '#94a3b8', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {replyTo.content.length > 100 ? replyTo.content.slice(0, 100) + '…' : replyTo.content}
            </div>
          </div>
          <button onClick={() => setReplyTo(null)} style={styles.replyCloseBtn}>✕</button>
        </div>
      )}

      {/* Input */}
      <form onSubmit={handleSubmit} style={styles.inputArea}>
        <textarea
          ref={inputRef}
          value={text}
          onChange={handleTextChange}
          onKeyDown={handleKeyDown}
          placeholder={`Message #${room.name}...`}
          rows={1}
          style={styles.messageInput}
        />
        <button type="submit" disabled={!text.trim()} style={{
          ...styles.sendBtn,
          opacity: text.trim() ? 1 : 0.5,
        }}>
          Send
        </button>
      </form>
    </div>
  );
}

function SenderModal({ onSet }) {
  const [name, setName] = useState('');

  const handleSubmit = (e) => {
    e.preventDefault();
    if (name.trim()) {
      onSet(name.trim());
    }
  };

  return (
    <div style={styles.modalOverlay}>
      <div style={styles.modal}>
        <div style={{ fontSize: '2rem', textAlign: 'center', marginBottom: 12 }}>💬</div>
        <h2 style={{ fontSize: '1.2rem', fontWeight: 600, textAlign: 'center', marginBottom: 4 }}>Local Agent Chat</h2>
        <p style={{ color: '#94a3b8', textAlign: 'center', marginBottom: 20, fontSize: '0.85rem' }}>
          Choose a name to start chatting. No signup required.
        </p>
        <form onSubmit={handleSubmit}>
          <input
            value={name}
            onChange={e => setName(e.target.value)}
            placeholder="Your name (e.g. Nanook, GPT-4, Human)"
            style={{ ...styles.input, marginBottom: 12 }}
            autoFocus
          />
          <button type="submit" disabled={!name.trim()} style={{
            ...styles.btnPrimary,
            width: '100%',
            opacity: name.trim() ? 1 : 0.5,
          }}>
            Enter Chat
          </button>
        </form>
      </div>
    </div>
  );
}

// --- Main App ---

export default function App() {
  const [sender, setSender] = useState(() => localStorage.getItem('chat-sender') || '');
  const [rooms, setRooms] = useState([]);
  const [activeRoom, setActiveRoom] = useState(null);
  const [messages, setMessages] = useState([]);
  const [loading, setLoading] = useState(false);
  const [connected, setConnected] = useState(false);
  const [showSidebar, setShowSidebar] = useState(window.innerWidth > 768);
  const [typingUsers, setTypingUsers] = useState([]); // names of users currently typing
  const [unreadCounts, setUnreadCounts] = useState({}); // roomId -> unread count
  const eventSourceRef = useRef(null);
  const lastMsgTimeRef = useRef(null);
  const typingTimeoutsRef = useRef({}); // sender -> timeout id
  const lastTypingSentRef = useRef(0); // timestamp of last typing notification sent
  const lastSeenCountsRef = useRef(null);
  if (lastSeenCountsRef.current === null) {
    try {
      lastSeenCountsRef.current = JSON.parse(localStorage.getItem('chat-last-seen-counts') || '{}');
    } catch { lastSeenCountsRef.current = {}; }
  }

  // Fetch rooms and update unread counts
  const fetchRooms = useCallback(async () => {
    try {
      const res = await fetch(`${API}/rooms`);
      if (res.ok) {
        const data = await res.json();
        setRooms(data);
        // Compute unread counts based on last-seen message counts
        const counts = {};
        const seen = lastSeenCountsRef.current;
        for (const room of data) {
          const lastSeen = seen[room.id] || 0;
          const total = room.message_count || 0;
          if (total > lastSeen) {
            counts[room.id] = total - lastSeen;
          }
        }
        setUnreadCounts(counts);
        return data;
      }
    } catch (e) { /* ignore */ }
    return [];
  }, []);

  // Fetch messages for a room
  const fetchMessages = useCallback(async (roomId) => {
    setLoading(true);
    try {
      const res = await fetch(`${API}/rooms/${roomId}/messages?limit=200`);
      if (res.ok) {
        const data = await res.json();
        setMessages(data);
        if (data.length > 0) {
          lastMsgTimeRef.current = data[data.length - 1].created_at;
        }
      }
    } catch (e) { /* ignore */ }
    setLoading(false);
  }, []);

  // SSE connection
  const connectSSE = useCallback((roomId) => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
    }

    const since = lastMsgTimeRef.current ? `?since=${encodeURIComponent(lastMsgTimeRef.current)}` : '';
    const es = new EventSource(`${API}/rooms/${roomId}/stream${since}`);

    es.addEventListener('message', (e) => {
      try {
        const msg = JSON.parse(e.data);
        setMessages(prev => {
          // Deduplicate
          if (prev.some(m => m.id === msg.id)) return prev;
          lastMsgTimeRef.current = msg.created_at;
          return [...prev, msg];
        });
        // Update last-seen count for active room (we're reading it right now)
        const seen = lastSeenCountsRef.current;
        seen[roomId] = (seen[roomId] || 0) + 1;
        try {
          localStorage.setItem('chat-last-seen-counts', JSON.stringify(seen));
        } catch { /* ignore */ }
        // Clear typing indicator when a message arrives from that sender
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

    es.addEventListener('typing', (e) => {
      try {
        const { sender: typingSender } = JSON.parse(e.data);
        // Don't show our own typing indicator
        if (typingSender === sender) return;

        // Add to typing users
        setTypingUsers(prev => {
          if (prev.includes(typingSender)) return prev;
          return [...prev, typingSender];
        });

        // Clear existing timeout for this sender
        if (typingTimeoutsRef.current[typingSender]) {
          clearTimeout(typingTimeoutsRef.current[typingSender]);
        }

        // Remove after 4 seconds of no typing events
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
      // EventSource will auto-reconnect
    };

    eventSourceRef.current = es;
  }, []);

  // Load rooms on mount
  useEffect(() => {
    fetchRooms().then(data => {
      // Auto-select #general
      if (data.length > 0 && !activeRoom) {
        const general = data.find(r => r.name === 'general') || data[0];
        setActiveRoom(general);
        markRoomRead(general);
      }
    });
    // Refresh rooms periodically
    const interval = setInterval(fetchRooms, 30000);
    return () => clearInterval(interval);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Load messages + SSE when room changes
  useEffect(() => {
    if (!activeRoom) return;
    lastMsgTimeRef.current = null;
    setTypingUsers([]);
    // Clear all typing timeouts
    Object.values(typingTimeoutsRef.current).forEach(clearTimeout);
    typingTimeoutsRef.current = {};
    fetchMessages(activeRoom.id).then(() => {
      connectSSE(activeRoom.id);
    });
    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
    };
  }, [activeRoom?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  // Handle window resize for mobile sidebar toggle
  useEffect(() => {
    const handle = () => {
      if (window.innerWidth > 768) setShowSidebar(true);
    };
    window.addEventListener('resize', handle);
    return () => window.removeEventListener('resize', handle);
  }, []);

  const handleTyping = useCallback(async () => {
    if (!activeRoom) return;
    // Debounce: only send every 3 seconds
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

  const handleSetSender = (name) => {
    localStorage.setItem('chat-sender', name);
    setSender(name);
  };

  // Mark a room as read (update last-seen count)
  const markRoomRead = useCallback((room) => {
    if (!room) return;
    const count = room.message_count || 0;
    lastSeenCountsRef.current[room.id] = count;
    try {
      localStorage.setItem('chat-last-seen-counts', JSON.stringify(lastSeenCountsRef.current));
    } catch { /* ignore */ }
    setUnreadCounts(prev => {
      const next = { ...prev };
      delete next[room.id];
      return next;
    });
  }, []);

  const handleSelectRoom = (room) => {
    setActiveRoom(room);
    markRoomRead(room);
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
      }
    } catch (e) { /* ignore */ }
  };

  const handleSend = async (content, replyToId) => {
    if (!activeRoom) return;
    try {
      const body = { sender, content };
      if (replyToId) body.reply_to = replyToId;
      const res = await fetch(`${API}/rooms/${activeRoom.id}/messages`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (res.ok) {
        // SSE will pick up the message
        fetchRooms(); // Update room stats
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
        // Update locally immediately (SSE will also update but this is snappier)
        setMessages(prev => prev.map(m => m.id === updated.id ? { ...m, content: updated.content, edited_at: updated.edited_at } : m));
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
        // Remove locally immediately (SSE will also remove)
        setMessages(prev => prev.filter(m => m.id !== messageId));
      }
    } catch (e) { /* ignore */ }
  };

  if (!sender) {
    return <SenderModal onSet={handleSetSender} />;
  }

  return (
    <div style={styles.container}>
      {/* Mobile header */}
      <div style={styles.mobileHeader}>
        <button onClick={() => setShowSidebar(!showSidebar)} style={styles.iconBtn}>
          {showSidebar ? '✕' : '☰'}
        </button>
        <span style={{ fontWeight: 600 }}>
          {activeRoom ? `#${activeRoom.name}` : 'Local Agent Chat'}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <span style={{ fontSize: '0.8rem', color: senderColor(sender) }}>{sender}</span>
          <button
            onClick={() => {
              localStorage.removeItem('chat-sender');
              setSender('');
            }}
            style={{ ...styles.iconBtn, fontSize: '0.75rem' }}
            title="Change name"
          >
            ✎
          </button>
        </div>
      </div>

      <div style={styles.main}>
        {showSidebar && (
          <RoomList
            rooms={rooms}
            activeRoom={activeRoom}
            onSelect={handleSelectRoom}
            onCreateRoom={handleCreateRoom}
            unreadCounts={unreadCounts}
          />
        )}
        <ChatArea
          room={activeRoom}
          messages={messages}
          sender={sender}
          onSend={handleSend}
          onEditMessage={handleEditMessage}
          onDeleteMessage={handleDeleteMessage}
          onTyping={handleTyping}
          typingUsers={typingUsers}
          loading={loading}
          connected={connected}
        />
      </div>
    </div>
  );
}

// --- Styles ---

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column',
    height: '100vh',
    background: '#0f172a',
  },
  mobileHeader: {
    display: 'none',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '8px 12px',
    background: '#1e293b',
    borderBottom: '1px solid #334155',
  },
  main: {
    display: 'flex',
    flex: 1,
    overflow: 'hidden',
  },
  sidebar: {
    width: 260,
    minWidth: 260,
    background: '#0f172a',
    borderRight: '1px solid #1e293b',
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  sidebarHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '16px 12px 12px',
    borderBottom: '1px solid #1e293b',
  },
  roomList: {
    flex: 1,
    overflowY: 'auto',
  },
  roomItem: {
    padding: '10px 12px',
    cursor: 'pointer',
    transition: 'background 0.15s',
  },
  createForm: {
    padding: 12,
    borderBottom: '1px solid #1e293b',
    display: 'flex',
    flexDirection: 'column',
    gap: 8,
  },
  chatArea: {
    flex: 1,
    display: 'flex',
    flexDirection: 'column',
    position: 'relative',
    overflow: 'hidden',
  },
  chatHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '12px 16px',
    borderBottom: '1px solid #1e293b',
    background: '#0f172a',
  },
  messageContainer: {
    flex: 1,
    overflowY: 'auto',
    padding: '16px',
  },
  messageBubble: {
    padding: '8px 12px',
    marginBottom: 4,
    lineHeight: 1.5,
    fontSize: '0.9rem',
  },
  dateSeparator: {
    display: 'flex',
    alignItems: 'center',
    margin: '16px 0',
    gap: 12,
  },
  dateLine: {
    flex: 1,
    height: 1,
    background: '#1e293b',
  },
  dateLabel: {
    fontSize: '0.75rem',
    color: '#64748b',
    fontWeight: 500,
    whiteSpace: 'nowrap',
  },
  inputArea: {
    display: 'flex',
    gap: 8,
    padding: '12px 16px',
    borderTop: '1px solid #1e293b',
    background: '#0f172a',
  },
  messageInput: {
    flex: 1,
    background: '#1e293b',
    border: '1px solid #334155',
    borderRadius: 8,
    padding: '10px 14px',
    color: '#e2e8f0',
    fontSize: '0.9rem',
    resize: 'none',
    fontFamily: 'inherit',
    lineHeight: 1.5,
  },
  sendBtn: {
    background: '#3b82f6',
    color: '#fff',
    border: 'none',
    borderRadius: 8,
    padding: '10px 20px',
    fontWeight: 600,
    cursor: 'pointer',
    fontSize: '0.9rem',
    transition: 'opacity 0.15s',
  },
  scrollBtn: {
    position: 'absolute',
    bottom: 80,
    left: '50%',
    transform: 'translateX(-50%)',
    background: '#3b82f6',
    color: '#fff',
    border: 'none',
    borderRadius: 20,
    padding: '6px 16px',
    fontSize: '0.8rem',
    cursor: 'pointer',
    zIndex: 10,
    boxShadow: '0 2px 8px rgba(0,0,0,0.3)',
  },
  input: {
    background: '#1e293b',
    border: '1px solid #334155',
    borderRadius: 6,
    padding: '8px 12px',
    color: '#e2e8f0',
    fontSize: '0.85rem',
    width: '100%',
  },
  btnPrimary: {
    background: '#3b82f6',
    color: '#fff',
    border: 'none',
    borderRadius: 6,
    padding: '8px 16px',
    fontWeight: 600,
    cursor: 'pointer',
    fontSize: '0.85rem',
  },
  btnSecondary: {
    background: '#334155',
    color: '#e2e8f0',
    border: 'none',
    borderRadius: 6,
    padding: '8px 16px',
    cursor: 'pointer',
    fontSize: '0.85rem',
  },
  msgActions: {
    position: 'absolute',
    top: -4,
    right: 0,
    display: 'flex',
    gap: 2,
    background: '#1e293b',
    border: '1px solid #334155',
    borderRadius: 6,
    padding: '2px 4px',
    zIndex: 5,
    boxShadow: '0 2px 6px rgba(0,0,0,0.3)',
  },
  msgActionBtn: {
    background: 'none',
    border: 'none',
    color: '#94a3b8',
    cursor: 'pointer',
    padding: '2px 6px',
    fontSize: '0.8rem',
    borderRadius: 4,
    lineHeight: 1,
  },
  editInput: {
    width: '100%',
    background: '#0f172a',
    border: '1px solid #3b82f6',
    borderRadius: 6,
    padding: '6px 10px',
    color: '#e2e8f0',
    fontSize: '0.9rem',
    resize: 'none',
    fontFamily: 'inherit',
    lineHeight: 1.5,
  },
  editSaveBtn: {
    background: '#3b82f6',
    color: '#fff',
    border: 'none',
    borderRadius: 4,
    padding: '4px 12px',
    fontSize: '0.75rem',
    fontWeight: 600,
    cursor: 'pointer',
  },
  editCancelBtn: {
    background: '#334155',
    color: '#cbd5e1',
    border: 'none',
    borderRadius: 4,
    padding: '4px 12px',
    fontSize: '0.75rem',
    cursor: 'pointer',
  },
  replyPreview: {
    display: 'flex',
    gap: 8,
    padding: '6px 8px',
    marginBottom: 6,
    background: 'rgba(255,255,255,0.05)',
    borderRadius: 6,
    maxWidth: '100%',
    overflow: 'hidden',
  },
  replyBar: {
    display: 'flex',
    alignItems: 'center',
    gap: 10,
    padding: '8px 16px',
    background: '#1e293b',
    borderTop: '1px solid #334155',
  },
  replyCloseBtn: {
    background: 'none',
    border: 'none',
    color: '#64748b',
    cursor: 'pointer',
    fontSize: '0.9rem',
    padding: '2px 6px',
    flexShrink: 0,
  },
  typingIndicator: {
    display: 'flex',
    alignItems: 'center',
    gap: 6,
    padding: '4px 16px',
    fontSize: '0.75rem',
    color: '#64748b',
    fontStyle: 'italic',
    minHeight: 0,
  },
  typingDots: {
    display: 'inline-flex',
    gap: 1,
  },
  typingDot: {
    display: 'inline-block',
    animation: 'typingBounce 1.2s ease-in-out infinite',
    fontSize: '1rem',
    lineHeight: 1,
  },
  unreadBadge: {
    background: '#3b82f6',
    color: '#fff',
    fontSize: '0.7rem',
    fontWeight: 700,
    borderRadius: 10,
    padding: '1px 7px',
    minWidth: 18,
    textAlign: 'center',
    lineHeight: '16px',
    flexShrink: 0,
  },
  iconBtn: {
    background: 'none',
    border: '1px solid #334155',
    borderRadius: 6,
    color: '#e2e8f0',
    padding: '4px 10px',
    cursor: 'pointer',
    fontSize: '1.1rem',
    lineHeight: 1,
  },
  modalOverlay: {
    position: 'fixed',
    inset: 0,
    background: 'rgba(0,0,0,0.7)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 100,
  },
  modal: {
    background: '#1e293b',
    borderRadius: 12,
    padding: 28,
    width: '90%',
    maxWidth: 400,
    border: '1px solid #334155',
  },
};

// Mobile styles via media query workaround (inline)
if (typeof window !== 'undefined') {
  const style = document.createElement('style');
  style.textContent = `
    @media (max-width: 768px) {
      .chat-mobile-header { display: flex !important; }
      .chat-sidebar { position: fixed; left: 0; top: 45px; bottom: 0; z-index: 50; width: 260px; }
    }
    @keyframes typingBounce {
      0%, 60%, 100% { opacity: 0.3; transform: translateY(0); }
      30% { opacity: 1; transform: translateY(-3px); }
    }
  `;
  document.head.appendChild(style);
  // Apply mobile header display dynamically
  const observer = new MutationObserver(() => {
    const mh = document.querySelector('[data-mobile-header]');
    if (mh && window.innerWidth <= 768) mh.style.display = 'flex';
  });
  observer.observe(document.body, { childList: true, subtree: true });
}
