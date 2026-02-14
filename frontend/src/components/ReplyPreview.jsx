import React from 'react';
import { styles } from '../styles';
import { senderColor } from '../utils';

export default function ReplyPreview({ replyToId, messages, style: extraStyle }) {
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
