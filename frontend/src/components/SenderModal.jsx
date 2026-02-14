import React, { useState } from 'react';
import { styles } from '../styles';
import ChatLogo from './ChatLogo';

export default function SenderModal({ onSet }) {
  const [name, setName] = useState('');
  const [senderType, setSenderType] = useState('agent');

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
