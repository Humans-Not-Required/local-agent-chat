import React from 'react';
import { styles } from '../styles';
import { senderColor, formatFileSize, formatTime, formatFullTimestamp, API } from '../utils';

export default function FileCard({ file, isOwn, sender, onDelete }) {
  const color = senderColor(file.sender);
  const isImage = file.content_type && file.content_type.startsWith('image/');
  const downloadUrl = `${API}/files/${file.id}`;

  return (
    <div style={{
      marginBottom: 16,
      display: 'flex',
      flexDirection: 'column',
      alignItems: isOwn ? 'flex-end' : 'flex-start',
    }}>
      <div style={{ fontSize: '0.8rem', fontWeight: 600, color, marginBottom: 4, paddingLeft: isOwn ? 0 : 4, paddingRight: isOwn ? 4 : 0 }}>
        📎 {file.sender}
      </div>
      <div style={{
        ...styles.fileBubble,
        background: isOwn ? '#1e3a5f' : '#1e293b',
        borderRadius: isOwn ? '12px 12px 4px 12px' : '12px 12px 12px 4px',
        maxWidth: '75%',
        position: 'relative',
      }}>
        {isImage && (
          <a href={downloadUrl} target="_blank" rel="noopener noreferrer" style={{ display: 'block', marginBottom: 8 }}>
            <img
              src={downloadUrl}
              alt={file.filename}
              style={styles.fileImagePreview}
              onError={(e) => { e.target.style.display = 'none'; }}
            />
          </a>
        )}
        <div style={styles.fileInfo}>
          <div style={styles.fileIcon}>
            {isImage ? '🖼️' : file.content_type?.includes('pdf') ? '📕' : file.content_type?.includes('json') ? '📋' : file.content_type?.includes('text') ? '📄' : '📦'}
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: '0.85rem', color: '#e2e8f0', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {file.filename}
            </div>
            <div style={{ fontSize: '0.7rem', color: '#64748b' }}>
              {formatFileSize(file.size)}
            </div>
          </div>
          <a
            href={downloadUrl}
            download={file.filename}
            style={styles.fileDownloadBtn}
            title="Download"
          >
            ⬇
          </a>
          {isOwn && (
            <button
              onClick={() => { if (window.confirm('Delete this file?')) onDelete(file.id); }}
              style={styles.fileDeleteBtn}
              title="Delete file"
            >
              ✕
            </button>
          )}
        </div>
        <div style={{ fontSize: '0.7rem', color: '#64748b', marginTop: 6, textAlign: 'right' }}>
          <span title={formatFullTimestamp(file.created_at)}>{formatTime(file.created_at)}</span>
        </div>
      </div>
    </div>
  );
}
