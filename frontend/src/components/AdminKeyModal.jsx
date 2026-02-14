import React, { useState } from 'react';
import { styles } from '../styles';

export default function AdminKeyModal({ roomName, adminKey, onDismiss }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(adminKey);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
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
