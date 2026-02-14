import React, { useState, useEffect, useRef } from 'react';
import { styles } from '../styles';
import { API, renderContent, formatTime, formatFullTimestamp, senderColor, timeAgo } from '../utils';
import ReactionChips from './ReactionChips';

export default function ThreadPanel({ roomId, messageId, sender, onReply, onClose }) {
  const [thread, setThread] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [replyText, setReplyText] = useState('');
  const [sending, setSending] = useState(false);
  const scrollRef = useRef(null);

  useEffect(() => {
    if (!roomId || !messageId) return;
    let cancelled = false;
    const fetchThread = async () => {
      setLoading(true);
      setError(null);
      try {
        const res = await fetch(`${API}/rooms/${roomId}/messages/${messageId}/thread`);
        if (res.ok && !cancelled) {
          setThread(await res.json());
        } else if (!cancelled) {
          setError('Failed to load thread');
        }
      } catch {
        if (!cancelled) setError('Failed to load thread');
      }
      if (!cancelled) setLoading(false);
    };
    fetchThread();
    return () => { cancelled = true; };
  }, [roomId, messageId]);

  // Scroll to bottom when thread loads
  useEffect(() => {
    if (thread && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [thread]);

  const handleSendReply = async () => {
    const trimmed = replyText.trim();
    if (!trimmed || !sender || sending) return;
    setSending(true);
    try {
      // Reply to the last message in the thread (continue the conversation)
      const replyToId = thread.replies.length > 0
        ? thread.replies[thread.replies.length - 1].id
        : thread.root.id;
      const res = await fetch(`${API}/rooms/${roomId}/messages`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          sender,
          content: trimmed,
          reply_to: replyToId,
          sender_type: localStorage.getItem('senderType') || 'agent',
        }),
      });
      if (res.ok) {
        setReplyText('');
        // Refresh thread
        const threadRes = await fetch(`${API}/rooms/${roomId}/messages/${messageId}/thread`);
        if (threadRes.ok) setThread(await threadRes.json());
      }
    } catch { /* ignore */ }
    setSending(false);
  };

  const handleKeyDown = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendReply();
    }
  };

  const renderMessage = (msg, isRoot) => {
    const depth = msg.depth || 0;
    return (
      <div key={msg.id || msg.message?.id} style={{
        background: isRoot ? '#1e3a5f' : '#1e293b',
        borderRadius: 8,
        padding: '10px 14px',
        marginBottom: 8,
        borderLeft: `3px solid ${senderColor(isRoot ? msg.sender : (msg.sender || msg.message?.sender))}`,
        marginLeft: isRoot ? 0 : Math.min(depth - 1, 3) * 12,
      }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ fontWeight: 600, fontSize: '0.8rem', color: senderColor(msg.sender) }}>
              {msg.sender}
            </span>
            {isRoot && (
              <span style={{
                fontSize: '0.6rem', background: 'rgba(96,165,250,0.2)', color: '#60a5fa',
                padding: '1px 6px', borderRadius: 10, fontWeight: 500,
              }}>ROOT</span>
            )}
            {!isRoot && depth > 1 && (
              <span style={{
                fontSize: '0.6rem', color: '#64748b',
              }}>depth {depth}</span>
            )}
          </div>
          <span style={{ fontSize: '0.65rem', color: '#64748b' }} title={formatFullTimestamp(msg.created_at)}>
            {timeAgo(msg.created_at)}
          </span>
        </div>
        <div style={{ fontSize: '0.85rem', whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
          {renderContent(msg.content)}
        </div>
        {msg.edited_at && (
          <span style={{
            fontSize: '0.65rem', color: '#64748b', fontStyle: 'italic',
          }} title={`Edited: ${formatFullTimestamp(msg.edited_at)}`}>(edited)</span>
        )}
      </div>
    );
  };

  return (
    <div style={{
      position: 'absolute', top: 0, left: 0, right: 0, bottom: 0,
      background: '#0f172a', zIndex: 40,
      display: 'flex', flexDirection: 'column',
    }}>
      {/* Header */}
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        padding: '12px 16px', borderBottom: '1px solid #1e293b',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 600, fontSize: '0.95rem' }}>🧵 Thread</span>
          {thread && (
            <span style={{ fontSize: '0.75rem', color: '#64748b' }}>
              {thread.total_replies} {thread.total_replies === 1 ? 'reply' : 'replies'}
            </span>
          )}
        </div>
        <button onClick={onClose} style={{
          background: 'none', border: 'none', color: '#94a3b8',
          fontSize: '1.1rem', cursor: 'pointer', padding: '4px 8px',
        }}>✕</button>
      </div>

      {/* Thread messages */}
      <div ref={scrollRef} style={{ flex: 1, overflowY: 'auto', padding: '12px 16px' }}>
        {loading && (
          <div style={{ textAlign: 'center', color: '#64748b', padding: 20 }}>Loading thread...</div>
        )}
        {error && (
          <div style={{ textAlign: 'center', color: '#ef4444', padding: 20 }}>{error}</div>
        )}
        {thread && (
          <>
            {renderMessage(thread.root, true)}
            {thread.replies.length > 0 && (
              <div style={{
                fontSize: '0.7rem', color: '#475569', padding: '4px 0 8px',
                borderBottom: '1px solid rgba(255,255,255,0.05)', marginBottom: 8,
              }}>
                {thread.total_replies} {thread.total_replies === 1 ? 'reply' : 'replies'}
              </div>
            )}
            {thread.replies.map(reply => renderMessage(reply, false))}
          </>
        )}
      </div>

      {/* Reply input */}
      {thread && sender && (
        <div style={{
          padding: '10px 16px', borderTop: '1px solid #1e293b',
          display: 'flex', gap: 8, alignItems: 'flex-end',
        }}>
          <textarea
            value={replyText}
            onChange={(e) => setReplyText(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Reply in thread..."
            rows={1}
            style={{
              flex: 1, background: '#1e293b', border: '1px solid #334155',
              borderRadius: 8, padding: '8px 12px', color: '#e2e8f0',
              fontSize: '0.9rem', resize: 'none', outline: 'none',
              fontFamily: 'inherit', minHeight: 38, maxHeight: 120,
              boxSizing: 'border-box',
            }}
          />
          <button
            onClick={handleSendReply}
            disabled={!replyText.trim() || sending}
            style={{
              background: replyText.trim() ? '#3b82f6' : '#334155',
              color: '#fff', border: 'none', borderRadius: 8,
              padding: '8px 16px', cursor: replyText.trim() ? 'pointer' : 'default',
              fontSize: '0.85rem', fontWeight: 500,
              opacity: replyText.trim() ? 1 : 0.5,
              height: 38, boxSizing: 'border-box',
            }}
          >
            {sending ? '...' : 'Reply'}
          </button>
        </div>
      )}
    </div>
  );
}
