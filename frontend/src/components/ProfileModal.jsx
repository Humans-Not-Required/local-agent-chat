import React, { useState, useEffect } from 'react';
import { styles } from '../styles';
import { API } from '../utils';

export default function ProfileModal({ sender, onClose, onSaved }) {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [form, setForm] = useState({
    display_name: '',
    avatar_url: '',
    bio: '',
    status_text: '',
  });

  useEffect(() => {
    fetch(`${API}/profiles/${encodeURIComponent(sender)}`)
      .then(r => r.ok ? r.json() : null)
      .then(data => {
        if (data) {
          setForm({
            display_name: data.display_name || '',
            avatar_url: data.avatar_url || '',
            bio: data.bio || '',
            status_text: data.status_text || '',
          });
        }
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, [sender]);

  const handleSave = async (e) => {
    e.preventDefault();
    setSaving(true);
    setError('');
    try {
      const body = {};
      if (form.display_name.trim()) body.display_name = form.display_name.trim();
      if (form.avatar_url.trim()) body.avatar_url = form.avatar_url.trim();
      if (form.bio.trim()) body.bio = form.bio.trim();
      if (form.status_text.trim()) body.status_text = form.status_text.trim();

      const res = await fetch(`${API}/profiles/${encodeURIComponent(sender)}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (!res.ok) throw new Error(`Failed to save profile (${res.status})`);
      const profile = await res.json();
      if (onSaved) onSaved(profile);
      onClose();
    } catch (err) {
      setError(err.message);
    } finally {
      setSaving(false);
    }
  };

  const inputStyle = {
    ...styles.input,
    marginBottom: 10,
    fontSize: '0.9rem',
  };

  const labelStyle = {
    display: 'block',
    fontSize: '0.75rem',
    color: '#94a3b8',
    marginBottom: 4,
    fontWeight: 500,
  };

  return (
    <div style={styles.modalOverlay} onClick={onClose}>
      <div style={{ ...styles.modal, maxWidth: 400 }} onClick={e => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
          <h3 style={{ fontSize: '1rem', fontWeight: 600, margin: 0 }}>✏️ Edit Profile</h3>
          <button onClick={onClose} style={styles.iconBtn}>✕</button>
        </div>

        {loading ? (
          <div style={{ padding: 20, textAlign: 'center', color: '#64748b' }}>Loading...</div>
        ) : (
          <form onSubmit={handleSave}>
            <div style={{ marginBottom: 12, textAlign: 'center' }}>
              {form.avatar_url && (
                <img
                  src={form.avatar_url}
                  alt="Avatar preview"
                  style={{
                    width: 64, height: 64, borderRadius: '50%',
                    objectFit: 'cover', border: '2px solid #334155',
                    marginBottom: 8,
                  }}
                  onError={e => { e.target.style.display = 'none'; }}
                />
              )}
              <div style={{ fontSize: '0.8rem', color: '#64748b' }}>@{sender}</div>
            </div>

            <label style={labelStyle}>Display Name</label>
            <input
              value={form.display_name}
              onChange={e => setForm(f => ({ ...f, display_name: e.target.value }))}
              placeholder={sender}
              style={inputStyle}
              maxLength={100}
            />

            <label style={labelStyle}>Avatar URL</label>
            <input
              value={form.avatar_url}
              onChange={e => setForm(f => ({ ...f, avatar_url: e.target.value }))}
              placeholder="https://example.com/avatar.png"
              style={inputStyle}
              type="url"
            />

            <label style={labelStyle}>Bio</label>
            <textarea
              value={form.bio}
              onChange={e => setForm(f => ({ ...f, bio: e.target.value }))}
              placeholder="A short description about yourself..."
              style={{ ...inputStyle, minHeight: 60, resize: 'vertical', fontFamily: 'inherit' }}
              maxLength={500}
            />

            <label style={labelStyle}>Status</label>
            <input
              value={form.status_text}
              onChange={e => setForm(f => ({ ...f, status_text: e.target.value }))}
              placeholder="online, busy, exploring..."
              style={inputStyle}
              maxLength={100}
            />

            {error && (
              <div style={{ color: '#ef4444', fontSize: '0.8rem', marginBottom: 8 }}>{error}</div>
            )}

            <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
              <button type="button" onClick={onClose} style={{
                ...styles.btnPrimary,
                flex: 1,
                background: '#334155',
                color: '#94a3b8',
              }}>
                Cancel
              </button>
              <button type="submit" disabled={saving} style={{
                ...styles.btnPrimary,
                flex: 1,
                opacity: saving ? 0.6 : 1,
              }}>
                {saving ? 'Saving...' : 'Save Profile'}
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}
