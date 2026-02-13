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

// Convert URLs to clickable links and highlight @mentions
function linkify(text) {
  if (!text) return text;
  // Match URLs or @mentions (word chars, dots, hyphens)
  const tokenRegex = /(https?:\/\/[^\s<>"')\]]+|www\.[^\s<>"')\]]+|@[\w.-]+)/g;
  const parts = [];
  let lastIndex = 0;
  let match;
  let keyIdx = 0;

  while ((match = tokenRegex.exec(text)) !== null) {
    // Add text before the match
    if (match.index > lastIndex) {
      parts.push(text.slice(lastIndex, match.index));
    }

    const token = match[0];

    if (token.startsWith('@')) {
      // @mention highlight
      parts.push(
        React.createElement('span', {
          key: `mention-${keyIdx++}`,
          style: { color: '#a78bfa', fontWeight: 600, background: 'rgba(167,139,250,0.1)', borderRadius: 3, padding: '0 2px' },
        }, token)
      );
    } else {
      // URL link
      let url = token;
      const trailing = url.match(/[.,;:!?]+$/);
      let suffix = '';
      if (trailing) {
        suffix = trailing[0];
        url = url.slice(0, -suffix.length);
      }

      const href = url.startsWith('www.') ? 'https://' + url : url;
      parts.push(
        React.createElement('a', {
          key: `link-${keyIdx++}`,
          href: href,
          target: '_blank',
          rel: 'noopener noreferrer',
          style: { color: '#60a5fa', textDecoration: 'underline', wordBreak: 'break-all' },
          onClick: (e) => e.stopPropagation(), // Don't trigger bubble click (action toggle)
        }, url)
      );
      if (suffix) parts.push(suffix);
    }

    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex));
  }

  return parts.length > 0 ? parts : text;
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

function ChatLogo({ size = 24, color = '#60a5fa', style: extraStyle }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      width={size}
      height={size}
      style={{ display: 'inline-block', verticalAlign: 'middle', flexShrink: 0, ...extraStyle }}
    >
      <circle cx="12" cy="12" r="10" stroke={color} strokeWidth="2" fill="none" />
      <path d="M8 10h0M12 10h0M16 10h0" stroke={color} strokeWidth="2" strokeLinecap="round" />
      <path d="M8 14c1.5 2 6.5 2 8 0" stroke={color} strokeWidth="2" strokeLinecap="round" fill="none" />
    </svg>
  );
}

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
    <div className="chat-sidebar" style={styles.sidebar}>
      <div style={styles.sidebarHeader}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <ChatLogo size={22} />
          <h2 style={{ fontSize: '1rem', fontWeight: 600 }}>Rooms</h2>
        </div>
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
      {/* Branding footer */}
      <div style={styles.sidebarFooter}>
        <ChatLogo size={16} color="#475569" />
        <span style={{ fontSize: '0.7rem', color: '#475569' }}>Local Agent Chat</span>
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
            <div style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>{linkify(msg.content)}</div>
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
  const msgType = messages[0].sender_type || messages[0].metadata?.sender_type;
  const typeIcon = msgType === 'human' ? '👤' : msgType === 'agent' ? '🤖' : '';

  return (
    <div style={{ marginBottom: 16, display: 'flex', flexDirection: 'column', alignItems: isOwn ? 'flex-end' : 'flex-start' }}>
      <div style={{ fontSize: '0.8rem', fontWeight: 600, color, marginBottom: 4, paddingLeft: isOwn ? 0 : 4, paddingRight: isOwn ? 4 : 0 }}>
        {typeIcon && <span style={{ marginRight: 4 }}>{typeIcon}</span>}{sender}
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

function formatFileSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function FileCard({ file, isOwn, sender, onDelete }) {
  const color = senderColor(file.sender);
  const isImage = file.content_type && file.content_type.startsWith('image/');
  const downloadUrl = `${API}/files/${file.id}`;

  return (
    <div style={{
      marginBottom: 16,
      display: 'flex',
      flexDirection: 'column',
      alignItems: isOwn ? 'flex-end' : 'flex-start',
    }}>
      <div style={{ fontSize: '0.8rem', fontWeight: 600, color, marginBottom: 4, paddingLeft: isOwn ? 0 : 4, paddingRight: isOwn ? 4 : 0 }}>
        📎 {file.sender}
      </div>
      <div style={{
        ...styles.fileBubble,
        background: isOwn ? '#1e3a5f' : '#1e293b',
        borderRadius: isOwn ? '12px 12px 4px 12px' : '12px 12px 12px 4px',
        maxWidth: '75%',
        position: 'relative',
      }}>
        {isImage && (
          <a href={downloadUrl} target="_blank" rel="noopener noreferrer" style={{ display: 'block', marginBottom: 8 }}>
            <img
              src={downloadUrl}
              alt={file.filename}
              style={styles.fileImagePreview}
              onError={(e) => { e.target.style.display = 'none'; }}
            />
          </a>
        )}
        <div style={styles.fileInfo}>
          <div style={styles.fileIcon}>
            {isImage ? '🖼️' : file.content_type?.includes('pdf') ? '📕' : file.content_type?.includes('json') ? '📋' : file.content_type?.includes('text') ? '📄' : '📦'}
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: '0.85rem', color: '#e2e8f0', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {file.filename}
            </div>
            <div style={{ fontSize: '0.7rem', color: '#64748b' }}>
              {formatFileSize(file.size)}
            </div>
          </div>
          <a
            href={downloadUrl}
            download={file.filename}
            style={styles.fileDownloadBtn}
            title="Download"
          >
            ⬇
          </a>
          {isOwn && (
            <button
              onClick={() => onDelete(file.id)}
              style={styles.fileDeleteBtn}
              title="Delete file"
            >
              ✕
            </button>
          )}
        </div>
        <div style={{ fontSize: '0.7rem', color: '#64748b', marginTop: 6, textAlign: 'right' }}>
          {formatTime(file.created_at)}
        </div>
      </div>
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

function ParticipantPanel({ roomId, onClose }) {
  const [participants, setParticipants] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    fetch(`${API}/rooms/${roomId}/participants`)
      .then(r => r.ok ? r.json() : [])
      .then(data => { if (!cancelled) { setParticipants(data); setLoading(false); } })
      .catch(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [roomId]);

  return (
    <div className="participant-panel-wrapper" style={styles.participantPanel}>
      <div style={styles.participantHeader}>
        <span style={{ fontWeight: 600, fontSize: '0.9rem' }}>👥 Members ({participants.length})</span>
        <button onClick={onClose} style={styles.iconBtn}>✕</button>
      </div>
      <div style={styles.participantList}>
        {loading && <div style={{ padding: 16, color: '#64748b', fontSize: '0.85rem' }}>Loading...</div>}
        {!loading && participants.length === 0 && (
          <div style={{ padding: 16, color: '#64748b', fontSize: '0.85rem' }}>No messages yet</div>
        )}
        {participants.map(p => {
          const color = senderColor(p.sender);
          const typeIcon = p.sender_type === 'human' ? '👤' : p.sender_type === 'agent' ? '🤖' : '❓';
          const isRecent = (Date.now() - new Date(p.last_seen).getTime()) < 3600000; // active in last hour
          return (
            <div key={p.sender} style={styles.participantItem}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
                <div style={{
                  width: 8, height: 8, borderRadius: '50%', flexShrink: 0,
                  background: isRecent ? '#34d399' : '#475569',
                }} />
                <span style={{ fontSize: '1rem', flexShrink: 0 }}>{typeIcon}</span>
                <span style={{
                  fontWeight: 600, color, fontSize: '0.85rem',
                  overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                }}>
                  {p.sender}
                </span>
              </div>
              <div style={{ fontSize: '0.7rem', color: '#64748b', marginTop: 2, paddingLeft: 26 }}>
                {p.message_count} msg{p.message_count !== 1 ? 's' : ''} · {timeAgo(p.last_seen)}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ChatArea({ room, messages, files, sender, onSend, onEditMessage, onDeleteMessage, onDeleteFile, onUploadFile, onTyping, typingUsers, loading, connected }) {
  const [text, setText] = useState('');
  const [replyTo, setReplyTo] = useState(null); // { id, sender, content }
  const messagesEndRef = useRef(null);
  const containerRef = useRef(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const inputRef = useRef(null);
  const fileInputRef = useRef(null);
  const [uploading, setUploading] = useState(false);
  const [showParticipants, setShowParticipants] = useState(false);

  // Clear reply state when room changes (keep participants panel open on desktop)
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
    // Reset textarea height after send
    if (inputRef.current) {
      inputRef.current.style.height = 'auto';
      inputRef.current.style.overflowY = 'hidden';
    }
  };

  const handleReply = (msg) => {
    setReplyTo({ id: msg.id, sender: msg.sender, content: msg.content });
    inputRef.current?.focus();
  };

  // Auto-resize textarea to fit content
  const autoResize = (el) => {
    if (!el) return;
    el.style.height = 'auto';
    // Clamp between 1 row (~24px + padding) and ~6 rows (~160px)
    const maxHeight = 160;
    el.style.height = Math.min(el.scrollHeight, maxHeight) + 'px';
    el.style.overflowY = el.scrollHeight > maxHeight ? 'auto' : 'hidden';
  };

  const handleTextChange = (e) => {
    setText(e.target.value);
    autoResize(e.target);
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

  const handleFileSelect = async (e) => {
    const file = e.target.files?.[0];
    if (!file) return;
    // Reset input so same file can be re-selected
    e.target.value = '';

    if (file.size > 5 * 1024 * 1024) {
      alert('File too large. Maximum size is 5 MB.');
      return;
    }

    setUploading(true);
    try {
      const reader = new FileReader();
      reader.onload = async () => {
        // reader.result is data:...;base64,XXXX — extract base64 part
        const base64 = reader.result.split(',')[1];
        await onUploadFile(file.name, file.type || 'application/octet-stream', base64);
        setUploading(false);
      };
      reader.onerror = () => setUploading(false);
      reader.readAsDataURL(file);
    } catch {
      setUploading(false);
    }
  };

  if (!room) {
    return (
      <div style={styles.chatArea}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: '#64748b' }}>
          <div style={{ textAlign: 'center' }}>
            <div style={{ marginBottom: 12 }}>
              <ChatLogo size={56} />
            </div>
            <div style={{ fontSize: '1.1rem', fontWeight: 500 }}>Local Agent Chat</div>
            <div style={{ fontSize: '0.85rem', marginTop: 8 }}>Select a room to start chatting</div>
          </div>
        </div>
      </div>
    );
  }

  // Merge messages and files into a single timeline sorted by created_at
  const timeline = [
    ...messages.map(m => ({ ...m, _type: 'message' })),
    ...(files || []).map(f => ({ ...f, _type: 'file' })),
  ].sort((a, b) => new Date(a.created_at) - new Date(b.created_at));

  // Group consecutive messages by sender and date, with files as group-breakers
  const grouped = [];
  let currentGroup = null;
  let currentDate = null;

  for (const item of timeline) {
    const itemDate = formatDate(item.created_at);
    if (itemDate !== currentDate) {
      if (currentGroup) grouped.push(currentGroup);
      currentGroup = null;
      currentDate = itemDate;
      grouped.push({ type: 'date', date: itemDate });
    }
    if (item._type === 'file') {
      if (currentGroup) grouped.push(currentGroup);
      currentGroup = null;
      grouped.push({ type: 'file', file: item });
    } else {
      if (currentGroup && currentGroup.sender === item.sender) {
        currentGroup.messages.push(item);
      } else {
        if (currentGroup) grouped.push(currentGroup);
        currentGroup = { type: 'messages', sender: item.sender, messages: [item] };
      }
    }
  }
  if (currentGroup) grouped.push(currentGroup);

  return (
    <div style={styles.chatArea}>
      {/* Header */}
      <div style={styles.chatHeader}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <ChatLogo size={20} />
          <span style={{ fontWeight: 600, fontSize: '1rem' }}>#{room.name}</span>
          {room.description && (
            <span style={{ color: '#64748b', fontSize: '0.85rem' }}>{room.description}</span>
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
          <button
            onClick={() => setShowParticipants(prev => !prev)}
            style={{
              ...styles.iconBtn,
              background: showParticipants ? '#334155' : 'none',
              fontSize: '0.9rem',
              padding: '4px 8px',
            }}
            title="Members"
          >
            👥
          </button>
        </div>
      </div>

      {/* Messages + Participants layout */}
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
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
          if (item.type === 'file') {
            return (
              <FileCard
                key={`file-${item.file.id}`}
                file={item.file}
                isOwn={item.file.sender === sender}
                sender={sender}
                onDelete={onDeleteFile}
              />
            );
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
      {showParticipants && room && (
        <ParticipantPanel roomId={room.id} onClose={() => setShowParticipants(false)} />
      )}
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
        <input
          ref={fileInputRef}
          type="file"
          style={{ display: 'none' }}
          onChange={handleFileSelect}
        />
        <button
          type="button"
          onClick={() => fileInputRef.current?.click()}
          disabled={uploading}
          style={{
            ...styles.fileAttachBtn,
            opacity: uploading ? 0.5 : 1,
          }}
          title={uploading ? 'Uploading...' : 'Attach file (max 5 MB)'}
        >
          {uploading ? '⏳' : '📎'}
        </button>
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
  const [senderType, setSenderType] = useState('agent'); // 'agent' or 'human'

  const handleSubmit = (e) => {
    e.preventDefault();
    if (name.trim()) {
      onSet(name.trim(), senderType);
    }
  };

  return (
    <div style={styles.modalOverlay}>
      <div style={styles.modal}>
        <div style={{ textAlign: 'center', marginBottom: 12 }}>
          <ChatLogo size={48} />
        </div>
        <h2 style={{ fontSize: '1.2rem', fontWeight: 600, textAlign: 'center', marginBottom: 4 }}>Local Agent Chat</h2>
        <p style={{ color: '#94a3b8', textAlign: 'center', marginBottom: 20, fontSize: '0.85rem' }}>
          Choose a name to start chatting. No signup required.
        </p>
        <form onSubmit={handleSubmit}>
          <input
            value={name}
            onChange={e => setName(e.target.value)}
            placeholder="Your name (e.g. Nanook, GPT-4, Alice)"
            style={{ ...styles.input, marginBottom: 14 }}
            autoFocus
          />
          <div style={styles.senderTypeToggle}>
            <button
              type="button"
              onClick={() => setSenderType('agent')}
              style={{
                ...styles.toggleBtn,
                background: senderType === 'agent' ? '#3b82f6' : '#334155',
                color: senderType === 'agent' ? '#fff' : '#94a3b8',
              }}
            >
              🤖 Agent
            </button>
            <button
              type="button"
              onClick={() => setSenderType('human')}
              style={{
                ...styles.toggleBtn,
                background: senderType === 'human' ? '#3b82f6' : '#334155',
                color: senderType === 'human' ? '#fff' : '#94a3b8',
              }}
            >
              👤 Human
            </button>
          </div>
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

function AdminKeyModal({ roomName, adminKey, onDismiss }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(adminKey);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback: select text
      const el = document.getElementById('admin-key-text');
      if (el) {
        el.select();
        document.execCommand('copy');
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      }
    }
  };

  return (
    <div style={styles.modalOverlay}>
      <div style={styles.modal}>
        <div style={{ fontSize: '2rem', textAlign: 'center', marginBottom: 12 }}>🔑</div>
        <h2 style={{ fontSize: '1.1rem', fontWeight: 600, textAlign: 'center', marginBottom: 4 }}>
          Room Created!
        </h2>
        <p style={{ color: '#94a3b8', textAlign: 'center', marginBottom: 16, fontSize: '0.85rem' }}>
          <strong style={{ color: '#e2e8f0' }}>#{roomName}</strong> is ready. Save the admin key below — it's needed to delete the room or moderate messages.
        </p>
        <div style={styles.adminKeyBox}>
          <input
            id="admin-key-text"
            readOnly
            value={adminKey}
            style={styles.adminKeyInput}
            onClick={(e) => e.target.select()}
          />
          <button onClick={handleCopy} style={styles.adminKeyCopyBtn}>
            {copied ? '✓ Copied' : 'Copy'}
          </button>
        </div>
        <p style={{ color: '#f59e0b', fontSize: '0.75rem', textAlign: 'center', marginTop: 10, marginBottom: 16 }}>
          ⚠️ This key is only shown once. Store it somewhere safe.
        </p>
        <button onClick={onDismiss} style={{ ...styles.btnPrimary, width: '100%' }}>
          Got it
        </button>
      </div>
    </div>
  );
}

// --- Main App ---

export default function App() {
  const [sender, setSender] = useState(() => localStorage.getItem('chat-sender') || '');
  const [senderType, setSenderType] = useState(() => localStorage.getItem('chat-sender-type') || 'agent');
  const [rooms, setRooms] = useState([]);
  const [activeRoom, setActiveRoom] = useState(null);
  const [messages, setMessages] = useState([]);
  const [files, setFiles] = useState([]);
  const [loading, setLoading] = useState(false);
  const [adminKeyInfo, setAdminKeyInfo] = useState(null); // { roomName, adminKey }
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

  // Fetch files for a room
  const fetchFiles = useCallback(async (roomId) => {
    try {
      const res = await fetch(`${API}/rooms/${roomId}/files`);
      if (res.ok) {
        const data = await res.json();
        setFiles(data);
      }
    } catch (e) { /* ignore */ }
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

  // Load messages + files + SSE when room changes
  useEffect(() => {
    if (!activeRoom) return;
    lastMsgTimeRef.current = null;
    setTypingUsers([]);
    setFiles([]);
    // Clear all typing timeouts
    Object.values(typingTimeoutsRef.current).forEach(clearTimeout);
    typingTimeoutsRef.current = {};
    Promise.all([
      fetchMessages(activeRoom.id),
      fetchFiles(activeRoom.id),
    ]).then(() => {
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

  const handleSetSender = (name, type) => {
    localStorage.setItem('chat-sender', name);
    localStorage.setItem('chat-sender-type', type || 'agent');
    setSender(name);
    setSenderType(type || 'agent');
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
        // Show admin key if returned (only shown on creation)
        if (room.admin_key) {
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
        // SSE will pick up the file_uploaded event
        fetchRooms(); // Update room stats
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
        // Remove locally immediately (SSE will also remove)
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
      {adminKeyInfo && (
        <AdminKeyModal
          roomName={adminKeyInfo.roomName}
          adminKey={adminKeyInfo.adminKey}
          onDismiss={() => setAdminKeyInfo(null)}
        />
      )}
      {/* Mobile header */}
      <div className="chat-mobile-header" data-mobile-header style={styles.mobileHeader}>
        <button onClick={() => setShowSidebar(!showSidebar)} style={styles.iconBtn}>
          {showSidebar ? '✕' : '☰'}
        </button>
        <span style={{ fontWeight: 600 }}>
          {activeRoom ? `#${activeRoom.name}` : 'Local Agent Chat'}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <span style={{ fontSize: '0.8rem', color: senderColor(sender) }}>
            {senderType === 'human' ? '👤' : '🤖'} {sender}
          </span>
          <button
            onClick={() => {
              localStorage.removeItem('chat-sender');
              localStorage.removeItem('chat-sender-type');
              setSender('');
              setSenderType('agent');
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
          <>
            {/* Backdrop overlay for mobile */}
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
            />
          </>
        )}
        <ChatArea
          room={activeRoom}
          messages={messages}
          files={files}
          sender={sender}
          onSend={handleSend}
          onEditMessage={handleEditMessage}
          onDeleteMessage={handleDeleteMessage}
          onDeleteFile={handleDeleteFile}
          onUploadFile={handleUploadFile}
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
    height: '100dvh',
    maxHeight: '100dvh',
    background: '#0f172a',
    overflow: 'hidden',
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
  sidebarFooter: {
    display: 'flex',
    alignItems: 'center',
    gap: 6,
    padding: '10px 12px',
    borderTop: '1px solid #1e293b',
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
    alignItems: 'flex-end',
  },
  messageInput: {
    flex: 1,
    background: '#1e293b',
    border: '1px solid #334155',
    borderRadius: 8,
    padding: '10px 14px',
    color: '#e2e8f0',
    fontSize: '1rem',
    resize: 'none',
    fontFamily: 'inherit',
    lineHeight: 1.5,
    minHeight: '42px',
    maxHeight: '160px',
    overflowY: 'hidden',
    transition: 'height 0.1s ease',
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
    fontSize: '1rem',
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
    fontSize: '1rem',
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
  senderTypeToggle: {
    display: 'flex',
    gap: 0,
    marginBottom: 14,
    borderRadius: 8,
    overflow: 'hidden',
    border: '1px solid #334155',
  },
  toggleBtn: {
    flex: 1,
    padding: '10px 16px',
    border: 'none',
    cursor: 'pointer',
    fontSize: '0.9rem',
    fontWeight: 600,
    transition: 'background 0.15s, color 0.15s',
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
  adminKeyBox: {
    display: 'flex',
    gap: 8,
    alignItems: 'center',
    background: '#0f172a',
    border: '1px solid #334155',
    borderRadius: 8,
    padding: 6,
  },
  adminKeyInput: {
    flex: 1,
    background: 'transparent',
    border: 'none',
    color: '#60a5fa',
    fontSize: '0.85rem',
    fontFamily: 'monospace',
    padding: '6px 8px',
    outline: 'none',
    minWidth: 0,
  },
  adminKeyCopyBtn: {
    background: '#334155',
    color: '#e2e8f0',
    border: 'none',
    borderRadius: 6,
    padding: '6px 14px',
    fontSize: '0.8rem',
    fontWeight: 600,
    cursor: 'pointer',
    whiteSpace: 'nowrap',
    transition: 'background 0.15s',
  },
  fileAttachBtn: {
    background: 'none',
    border: '1px solid #334155',
    borderRadius: 8,
    color: '#e2e8f0',
    padding: '8px 12px',
    cursor: 'pointer',
    fontSize: '1.1rem',
    lineHeight: 1,
    flexShrink: 0,
    transition: 'background 0.15s',
  },
  fileBubble: {
    padding: '10px 14px',
    lineHeight: 1.5,
    fontSize: '0.9rem',
  },
  fileImagePreview: {
    maxWidth: '100%',
    maxHeight: 200,
    borderRadius: 8,
    display: 'block',
    objectFit: 'contain',
  },
  fileInfo: {
    display: 'flex',
    alignItems: 'center',
    gap: 10,
  },
  fileIcon: {
    fontSize: '1.5rem',
    flexShrink: 0,
    lineHeight: 1,
  },
  fileDownloadBtn: {
    background: '#334155',
    color: '#e2e8f0',
    border: 'none',
    borderRadius: 6,
    padding: '4px 10px',
    cursor: 'pointer',
    fontSize: '0.85rem',
    textDecoration: 'none',
    flexShrink: 0,
    display: 'flex',
    alignItems: 'center',
  },
  participantPanel: {
    width: 240,
    minWidth: 240,
    borderLeft: '1px solid #1e293b',
    background: '#0f172a',
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  participantHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '12px',
    borderBottom: '1px solid #1e293b',
  },
  participantList: {
    flex: 1,
    overflowY: 'auto',
  },
  participantItem: {
    padding: '8px 12px',
    borderBottom: '1px solid rgba(30,41,59,0.5)',
  },
  fileDeleteBtn: {
    background: 'none',
    border: 'none',
    color: '#ef4444',
    cursor: 'pointer',
    fontSize: '0.85rem',
    padding: '4px 6px',
    flexShrink: 0,
    lineHeight: 1,
  },
};

// Mobile styles via media query workaround (inline)
if (typeof window !== 'undefined') {
  const style = document.createElement('style');
  style.textContent = `
    /* iOS Safari 100vh fix - fallback for browsers without dvh support */
    @supports not (height: 100dvh) {
      #root > div {
        height: 100vh !important;
        height: -webkit-fill-available !important;
        max-height: 100vh !important;
        max-height: -webkit-fill-available !important;
      }
    }
    @media (max-width: 768px) {
      .chat-mobile-header { display: flex !important; }
      .chat-sidebar {
        position: fixed !important;
        left: 0; top: 45px; bottom: 0;
        z-index: 50;
        width: 280px !important;
        min-width: 280px !important;
        background: #0f172a !important;
        box-shadow: 4px 0 24px rgba(0,0,0,0.5);
        animation: slideIn 0.2s ease-out;
      }
      .chat-sidebar-backdrop {
        display: block !important;
        position: fixed;
        inset: 0;
        top: 45px;
        background: rgba(0,0,0,0.5);
        z-index: 40;
      }
    }
    /* Prevent iOS bounce scroll on main layout */
    @media (max-width: 768px) {
      body { position: fixed; width: 100%; }
    }
    @media (min-width: 769px) {
      .chat-sidebar-backdrop { display: none !important; }
    }
    @media (max-width: 768px) {
      .participant-panel-wrapper {
        position: fixed !important;
        right: 0; top: 45px; bottom: 0;
        z-index: 50;
        width: 260px !important;
        min-width: 260px !important;
        box-shadow: -4px 0 24px rgba(0,0,0,0.5);
        animation: slideInRight 0.2s ease-out;
      }
    }
    @keyframes slideInRight {
      from { transform: translateX(100%); }
      to { transform: translateX(0); }
    }
    @keyframes slideIn {
      from { transform: translateX(-100%); }
      to { transform: translateX(0); }
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
