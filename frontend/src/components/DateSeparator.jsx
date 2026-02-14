import React from 'react';
import { styles } from '../styles';

export default function DateSeparator({ date }) {
  return (
    <div style={styles.dateSeparator}>
      <span style={styles.dateLine} />
      <span style={styles.dateLabel}>{date}</span>
      <span style={styles.dateLine} />
    </div>
  );
}
