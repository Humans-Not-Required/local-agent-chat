import React, { useState, useRef, useEffect } from 'react';
import { styles } from '../styles';
import { senderColor } from '../utils';
import MentionAutocomplete from './MentionAutocomplete';

export default function MessageInput({
  room, onSend, onTyping,
  replyTo, onCancelReply,
  uploading, fileInputRef, onFileSelect, onPaste,
  participants,
}) {
  const [text, setText] = useState('');
  const inputRef = useRef(null);
  const [mentionQuery, setMentionQuery] = useState(null);
  const [mentionIndex, setMentionIndex] = useState(0);
  const mentionStartRef = useRef(null);

  // Reset mention state on room change
  useEffect(() => {
    setMentionQuery(null);
    setMentionIndex(0);
    mentionStartRef.current = null;
  }, [room?.id]);

  // Focus input when replyTo changes
  useEffect(() => {
    if (replyTo) inputRef.current?.focus();
  }, [replyTo]);

  const filteredMentions = mentionQuery !== null
    ? participants.filter(p => p.sender.toLowerCase().includes(mentionQuery.toLowerCase()))
    : [];

  const handleMentionSelect = (name) => {
    if (!inputRef.current || mentionStartRef.current === null) return;
    const el = inputRef.current;
    const before = text.slice(0, mentionStartRef.current);
    const after = text.slice(mentionStartRef.current + 1 + (mentionQuery || '').length);
    const newText = before + '@' + name + ' ' + after;
    setText(newText);
    setMentionQuery(null);
    setMentionIndex(0);
    mentionStartRef.current = null;
    requestAnimationFrame(() => {
      const pos = before.length + 1 + name.length + 1;
      el.setSelectionRange(pos, pos);
      el.focus();
      autoResize(el);
    });
  };

  const detectMention = (value, cursorPos) => {
    const textBeforeCursor = value.slice(0, cursorPos);
    const atIndex = textBeforeCursor.lastIndexOf('@');
    if (atIndex === -1) {
      setMentionQuery(null);
      mentionStartRef.current = null;
      return;
    }
    if (atIndex > 0 && !/\s/.test(textBeforeCursor[atIndex - 1])) {
      setMentionQuery(null);
      mentionStartRef.current = null;
      return;
    }
    const query = textBeforeCursor.slice(atIndex + 1);
    if (/\s/.test(query)) {
      setMentionQuery(null);
      mentionStartRef.current = null;
      return;
    }
    if (query.length > 30) {
      setMentionQuery(null);
      mentionStartRef.current = null;
      return;
    }
    mentionStartRef.current = atIndex;
    setMentionQuery(query);
    setMentionIndex(0);
  };

  const autoResize = (el) => {
    if (!el) return;
    el.style.height = 'auto';
    const maxHeight = 160;
    const h = el.scrollHeight + 2;
    el.style.height = Math.min(h, maxHeight) + 'px';
    el.style.overflowY = h > maxHeight ? 'auto' : 'hidden';
  };

  const handleTextChange = (e) => {
    const value = e.target.value;
    setText(value);
    autoResize(e.target);
    if (value.trim()) onTyping();
    detectMention(value, e.target.selectionStart);
  };

  const handleSubmit = (e) => {
    e.preventDefault();
    if (!text.trim()) return;
    onSend(text.trim(), replyTo?.id || null);
    setText('');
    onCancelReply();
    if (inputRef.current) {
      inputRef.current.style.height = 'auto';
      inputRef.current.style.overflowY = 'hidden';
    }
  };

  const handleKeyDown = (e) => {
    if (mentionQuery !== null && filteredMentions.length > 0) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setMentionIndex(prev => (prev + 1) % filteredMentions.length);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setMentionIndex(prev => (prev - 1 + filteredMentions.length) % filteredMentions.length);
        return;
      }
      if (e.key === 'Enter' || e.key === 'Tab') {
        e.preventDefault();
        handleMentionSelect(filteredMentions[mentionIndex].sender);
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        setMentionQuery(null);
        mentionStartRef.current = null;
        return;
      }
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
    if (e.key === 'Escape' && replyTo) {
      onCancelReply();
    }
  };

  return (
    <>
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
          <button onClick={onCancelReply} style={styles.replyCloseBtn}>✕</button>
        </div>
      )}
      <form onSubmit={handleSubmit} style={{ ...styles.inputArea, position: 'relative' }}>
        {mentionQuery !== null && filteredMentions.length > 0 && (
          <MentionAutocomplete
            query={mentionQuery}
            participants={filteredMentions}
            activeIndex={mentionIndex}
            onSelect={handleMentionSelect}
            onClose={() => { setMentionQuery(null); mentionStartRef.current = null; }}
          />
        )}
        <input
          ref={fileInputRef}
          type="file"
          style={{ display: 'none' }}
          onChange={onFileSelect}
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
          onPaste={onPaste}
          onClick={(e) => detectMention(text, e.target.selectionStart)}
          onBlur={() => {
            setTimeout(() => { setMentionQuery(null); mentionStartRef.current = null; }, 200);
          }}
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
    </>
  );
}
