import React, { useState } from 'react';
import { styles } from '../styles';
import { timeAgo, formatFullTimestamp, senderColor } from '../utils';

export default function DmSection({ conversations, activeRoom, onSelectDm, onStartDm, sender }) {
  const [composing, setComposing] = useState(false);
  const [recipient, setRecipient] = useState('');
  const [firstMessage, setFirstMessage] = useState('');
  const [sending, setSending] = useState(false);

  const handleStartDm = async (e) => {
    e.preventDefault();
    if (!recipient.trim() || !firstMessage.trim()) return;
    setSending(true);
    try {
      await onStartDm(recipient.trim(), firstMessage.trim());
      setRecipient('');
      setFirstMessage('');
      setComposing(false);
    } catch (err) {
      // ignore
    }
    setSending(false);
  };

  return (
    <div>
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '10px 14px 6px', borderTop: '1px solid #1e293b',
      }}>
        <h3 style={{ fontSize: '0.8rem', fontWeight: 600, color: '#94a3b8', textTransform: 'uppercase', letterSpacing: '0.05em', margin: 0 }}>
          Direct Messages
        </h3>
        <button
          onClick={() => setComposing(!composing)}
          style={{ ...styles.iconBtn, fontSize: '0.85rem' }}
          title="New DM"
        >
          ✉
        </button>
      </div>

      {composing && (
        <form onSubmit={handleStartDm} style={{ ...styles.createForm, padding: '6px 12px 10px' }}>
          <input
            value={recipient}
            onChange={e => setRecipient(e.target.value)}
            placeholder="Recipient name"
            style={styles.input}
            autoFocus
          />
          <textarea
            value={firstMessage}
            onChange={e => setFirstMessage(e.target.value)}
            placeholder="Message..."
            style={{ ...styles.input, resize: 'vertical', minHeight: 40, maxHeight: 100, fontFamily: 'inherit' }}
            rows={2}
          />
          <div style={{ display: 'flex', gap: 6 }}>
            <button type="submit" disabled={sending} style={styles.btnPrimary}>
              {sending ? '...' : 'Send'}
            </button>
            <button type="button" onClick={() => setComposing(false)} style={styles.btnSecondary}>Cancel</button>
          </div>
        </form>
      )}

      <div style={{ maxHeight: '30vh', overflowY: 'auto' }}>
        {conversations.map(conv => {
          const isActive = activeRoom?.id === conv.room_id;
          const hasUnread = conv.unread_count > 0;
          return (
            <div
              key={conv.room_id}
              onClick={() => onSelectDm(conv)}
              style={{
                ...styles.roomItem,
                background: isActive ? '#1e293b' : 'transparent',
                borderLeft: isActive ? '3px solid #a78bfa' : '3px solid transparent',
                cursor: 'pointer',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <div style={{
                  fontWeight: hasUnread ? 700 : 500,
                  color: isActive ? '#f1f5f9' : hasUnread ? '#f1f5f9' : '#cbd5e1',
                  fontSize: '0.9rem',
                }}>
                  <span style={{ color: senderColor(conv.other_participant), fontWeight: 600 }}>
                    {conv.other_participant}
                  </span>
                </div>
                {hasUnread && (
                  <span style={{
                    ...styles.unreadBadge,
                    background: '#a78bfa', // Purple for DMs to distinguish from rooms
                  }}>
                    {conv.unread_count > 99 ? '99+' : conv.unread_count}
                  </span>
                )}
              </div>
              {conv.last_message_content && (
                <div style={{ fontSize: '0.75rem', color: '#94a3b8', marginTop: 2, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  <span style={{ color: '#cbd5e1', fontWeight: 500 }}>{conv.last_message_sender}:</span>{' '}
                  {conv.last_message_content.length > 50 ? conv.last_message_content.slice(0, 50) + '…' : conv.last_message_content}
                </div>
              )}
              {conv.last_message_at && (
                <div style={{ fontSize: '0.7rem', color: '#64748b', marginTop: 2 }}>
                  {conv.message_count} msgs
                  <span title={formatFullTimestamp(conv.last_message_at)}>{` · ${timeAgo(conv.last_message_at)}`}</span>
                </div>
              )}
            </div>
          );
        })}
        {conversations.length === 0 && !composing && (
          <div style={{ padding: '8px 14px', color: '#64748b', fontSize: '0.8rem' }}>
            No conversations yet
          </div>
        )}
      </div>
    </div>
  );
}
