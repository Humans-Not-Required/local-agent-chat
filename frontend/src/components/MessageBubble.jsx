import React, { useState } from 'react';
import { styles } from '../styles';
import { renderContent, formatTime, formatFullTimestamp } from '../utils';
import ReplyPreview from './ReplyPreview';
import ReactionChips from './ReactionChips';
import EmojiPicker from './EmojiPicker';

export default function MessageBubble({ msg, isOwn, onEdit, onDelete, onReply, onReact, reactions, sender, allMessages }) {
  const [showActions, setShowActions] = useState(false);
  const [editing, setEditing] = useState(false);
  const [editText, setEditText] = useState(msg.content);
  const [showEmojiPicker, setShowEmojiPicker] = useState(false);

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

  const handleBubbleClick = (e) => {
    if (!editing) {
      if (e.target.closest('[data-actions]') || e.target.closest('[data-reactions]') || e.target.closest('[data-emoji-picker]')) return;
      setShowActions(prev => !prev);
    }
  };

  const handleEmojiSelect = (emoji) => {
    onReact(msg.id, emoji);
    setShowEmojiPicker(false);
    setShowActions(false);
  };

  return (
    <div
      style={{
        position: 'relative',
        alignSelf: isOwn ? 'flex-end' : 'flex-start',
        maxWidth: '75%',
      }}
      onMouseEnter={() => setShowActions(true)}
      onMouseLeave={() => { setShowActions(false); setShowEmojiPicker(false); }}
    >
      {showActions && !editing && (
        <div style={styles.msgActions} data-actions>
          <button
            onClick={(e) => { e.stopPropagation(); onReply(msg); setShowActions(false); }}
            style={styles.msgActionBtn}
            title="Reply"
          >↩</button>
          <button
            onClick={(e) => { e.stopPropagation(); setShowEmojiPicker(prev => !prev); }}
            style={styles.msgActionBtn}
            title="React"
          >😀</button>
          {isOwn && (
            <>
              <button
                onClick={(e) => { e.stopPropagation(); setEditText(msg.content); setEditing(true); setShowActions(false); }}
                style={styles.msgActionBtn}
                title="Edit"
              >✎</button>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  if (window.confirm('Delete this message?')) onDelete(msg.id);
                }}
                style={{ ...styles.msgActionBtn, color: '#ef4444' }}
                title="Delete"
              >✕</button>
            </>
          )}
          {showEmojiPicker && (
            <div data-emoji-picker>
              <EmojiPicker onSelect={handleEmojiSelect} onClose={() => setShowEmojiPicker(false)} />
            </div>
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
            <div style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>{renderContent(msg.content)}</div>
            <div style={{ fontSize: '0.7rem', color: '#64748b', marginTop: 4, textAlign: 'right', display: 'flex', justifyContent: 'flex-end', gap: 6, alignItems: 'center' }}>
              {msg.edited_at && <span style={{ fontStyle: 'italic' }} title={`Edited: ${formatFullTimestamp(msg.edited_at)}`}>(edited)</span>}
              <span title={formatFullTimestamp(msg.created_at)}>{formatTime(msg.created_at)}</span>
            </div>
          </>
        )}
      </div>
      <div data-reactions>
        <ReactionChips
          reactions={reactions}
          sender={sender}
          onToggle={(emoji) => onReact(msg.id, emoji)}
        />
      </div>
    </div>
  );
}
