import React, { useState, useEffect } from 'react';
import { styles } from '../styles';
import { API, renderContent, formatTime, formatFullTimestamp, senderColor } from '../utils';

export default function PinnedPanel({ roomId, adminKey, onUnpin, onClose }) {
  const [pins, setPins] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    const fetchPins = async () => {
      setLoading(true);
      try {
        const res = await fetch(`${API}/rooms/${roomId}/pins`);
        if (res.ok && !cancelled) {
          setPins(await res.json());
        }
      } catch { /* ignore */ }
      if (!cancelled) setLoading(false);
    };
    fetchPins();
    return () => { cancelled = true; };
  }, [roomId]);

  const handleUnpin = async (msgId) => {
    if (!adminKey) return;
    if (!window.confirm('Unpin this message?')) return;
    try {
      const res = await fetch(`${API}/rooms/${roomId}/messages/${msgId}/pin`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${adminKey}` },
      });
      if (res.ok) {
        setPins(prev => prev.filter(p => p.id !== msgId));
        onUnpin?.(msgId);
      }
    } catch { /* ignore */ }
  };

  return (
    <div style={{
      position: 'absolute', top: 0, left: 0, right: 0, bottom: 0,
      background: '#0f172a', zIndex: 40,
      display: 'flex', flexDirection: 'column',
    }}>
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        padding: '12px 16px', borderBottom: '1px solid #1e293b',
      }}>
        <span style={{ fontWeight: 600, fontSize: '0.95rem' }}>📌 Pinned Messages</span>
        <button onClick={onClose} style={{
          background: 'none', border: 'none', color: '#94a3b8',
          fontSize: '1.1rem', cursor: 'pointer', padding: '4px 8px',
        }}>✕</button>
      </div>
      <div style={{ flex: 1, overflowY: 'auto', padding: '12px 16px' }}>
        {loading && (
          <div style={{ textAlign: 'center', color: '#64748b', padding: 20 }}>Loading...</div>
        )}
        {!loading && pins.length === 0 && (
          <div style={{ textAlign: 'center', color: '#64748b', padding: 40 }}>
            <div style={{ fontSize: '1.5rem', marginBottom: 8 }}>📌</div>
            <div>No pinned messages yet</div>
            <div style={{ fontSize: '0.8rem', marginTop: 4 }}>
              Pin important messages with the 📌 action
            </div>
          </div>
        )}
        {pins.map(pin => (
          <div key={pin.id} style={{
            background: '#1e293b', borderRadius: 8, padding: '10px 14px',
            marginBottom: 8, borderLeft: `3px solid ${senderColor(pin.sender)}`,
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
              <span style={{ fontWeight: 600, fontSize: '0.8rem', color: senderColor(pin.sender) }}>
                {pin.sender}
              </span>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <span style={{ fontSize: '0.65rem', color: '#64748b' }} title={formatFullTimestamp(pin.created_at)}>
                  {formatTime(pin.created_at)}
                </span>
                {adminKey && (
                  <button
                    onClick={() => handleUnpin(pin.id)}
                    style={{
                      background: 'none', border: 'none', color: '#64748b',
                      cursor: 'pointer', fontSize: '0.75rem', padding: '2px 4px',
                    }}
                    title="Unpin"
                  >✕</button>
                )}
              </div>
            </div>
            <div style={{ fontSize: '0.85rem', whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
              {renderContent(pin.content)}
            </div>
            <div style={{ fontSize: '0.6rem', color: '#475569', marginTop: 6 }}>
              Pinned {pin.pinned_at ? formatTime(pin.pinned_at) : ''} by {pin.pinned_by}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
