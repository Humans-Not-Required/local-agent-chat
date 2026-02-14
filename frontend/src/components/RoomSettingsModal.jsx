import React, { useState } from 'react';
import { styles } from '../styles';
import { API } from '../utils';

export default function RoomSettingsModal({ room, onClose, onUpdated, onRoomArchived }) {
  const [name, setName] = useState(room.name);
  const [description, setDescription] = useState(room.description || '');
  const [adminKey, setAdminKey] = useState('');
  const [saving, setSaving] = useState(false);
  const [archiving, setArchiving] = useState(false);
  const [error, setError] = useState('');

  const handleSave = async (e) => {
    e.preventDefault();
    if (!adminKey.trim()) { setError('Admin key is required to edit'); return; }
    if (!name.trim()) { setError('Room name cannot be empty'); return; }
    setSaving(true);
    setError('');
    try {
      const body = {};
      if (name.trim() !== room.name) body.name = name.trim();
      if (description.trim() !== (room.description || '')) body.description = description.trim();
      if (Object.keys(body).length === 0) { onClose(); return; }
      const res = await fetch(`${API}/rooms/${room.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${adminKey.trim()}` },
        body: JSON.stringify(body),
      });
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        if (res.status === 401) setError('Invalid admin key');
        else if (res.status === 409) setError('A room with that name already exists');
        else setError(data.error || `Failed (${res.status})`);
        setSaving(false);
        return;
      }
      const updated = await res.json();
      onUpdated(updated);
      onClose();
    } catch {
      setError('Network error');
      setSaving(false);
    }
  };

  const handleArchiveToggle = async () => {
    if (!adminKey.trim()) { setError('Admin key is required'); return; }
    const action = room.archived_at ? 'unarchive' : 'archive';
    if (!window.confirm(`${action === 'archive' ? 'Archive' : 'Unarchive'} #${room.name}? ${action === 'archive' ? 'It will be hidden from the room list.' : 'It will appear in the room list again.'}`)) return;
    setArchiving(true);
    setError('');
    try {
      const res = await fetch(`${API}/rooms/${room.id}/${action}`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${adminKey.trim()}` },
      });
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        if (res.status === 403) setError('Invalid admin key');
        else setError(data.error || `Failed (${res.status})`);
        setArchiving(false);
        return;
      }
      const updated = await res.json();
      if (onRoomArchived) onRoomArchived(updated);
      onClose();
    } catch {
      setError('Network error');
      setArchiving(false);
    }
  };

  return (
    <div style={styles.modalOverlay} onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={styles.modal}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
          <h2 style={{ fontSize: '1.1rem', fontWeight: 600, margin: 0 }}>⚙️ Room Settings</h2>
          <button onClick={onClose} style={{ background: 'none', border: 'none', color: '#94a3b8', fontSize: '1.2rem', cursor: 'pointer', padding: '4px 8px' }}>✕</button>
        </div>
        <div style={{ color: '#64748b', fontSize: '0.75rem', marginBottom: 16 }}>
          Created by <strong style={{ color: '#94a3b8' }}>{room.created_by || 'anonymous'}</strong>
          {room.created_at && <span> · {new Date(room.created_at).toLocaleDateString()}</span>}
          {room.archived_at && <span style={{ color: '#f59e0b', marginLeft: 8 }}>📦 Archived</span>}
        </div>
        <form onSubmit={handleSave}>
          <label style={{ display: 'block', color: '#94a3b8', fontSize: '0.8rem', marginBottom: 4, fontWeight: 500 }}>Name</label>
          <input
            value={name}
            onChange={e => setName(e.target.value)}
            placeholder="Room name"
            style={{ ...styles.input, marginBottom: 12 }}
            autoFocus
          />
          <label style={{ display: 'block', color: '#94a3b8', fontSize: '0.8rem', marginBottom: 4, fontWeight: 500 }}>Description</label>
          <textarea
            value={description}
            onChange={e => setDescription(e.target.value)}
            placeholder="What's this room about?"
            rows={3}
            style={{ ...styles.input, marginBottom: 12, resize: 'vertical', fontFamily: 'inherit' }}
          />
          <label style={{ display: 'block', color: '#94a3b8', fontSize: '0.8rem', marginBottom: 4, fontWeight: 500 }}>Admin Key</label>
          <input
            type="password"
            value={adminKey}
            onChange={e => setAdminKey(e.target.value)}
            placeholder="Required to save changes"
            style={{ ...styles.input, marginBottom: 6 }}
          />
          <p style={{ color: '#64748b', fontSize: '0.7rem', marginBottom: 16 }}>
            The admin key was shown when this room was created.
          </p>
          {error && <p style={{ color: '#ef4444', fontSize: '0.8rem', marginBottom: 12 }}>{error}</p>}
          <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', alignItems: 'center' }}>
            <button
              type="button"
              onClick={handleArchiveToggle}
              disabled={archiving}
              style={{
                ...styles.btnSecondary,
                marginRight: 'auto',
                color: room.archived_at ? '#22c55e' : '#f59e0b',
                borderColor: room.archived_at ? '#22c55e33' : '#f59e0b33',
                fontSize: '0.8rem',
                opacity: archiving ? 0.6 : 1,
              }}
              title={room.archived_at ? 'Restore this room to active' : 'Hide this room from the room list'}
            >
              {archiving ? '...' : room.archived_at ? '📤 Unarchive' : '📦 Archive'}
            </button>
            <button type="button" onClick={onClose} style={styles.btnSecondary}>Cancel</button>
            <button type="submit" disabled={saving} style={{ ...styles.btnPrimary, opacity: saving ? 0.6 : 1 }}>
              {saving ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
