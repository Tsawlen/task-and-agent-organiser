import { useState } from 'react';
import type { AccountKind } from '../lib/types';
import { addAccount } from '../lib/api';
import { XIcon } from './icons';

interface Props {
  onClose: () => void;
  onAdded: () => void;
}

const KIND_LABELS: Record<AccountKind, string> = {
  github: 'GitHub.com',
  github_enterprise: 'GitHub Enterprise',
  gitlab: 'GitLab (self-hosted or .com)',
};

export function AddAccountDialog({ onClose, onAdded }: Props) {
  const [kind, setKind] = useState<AccountKind>('github');
  const [baseUrl, setBaseUrl] = useState('');
  const [label, setLabel] = useState('');
  const [token, setToken] = useState('');
  const [error, setError] = useState('');
  const [saving, setSaving] = useState(false);

  const needsBaseUrl = kind !== 'github';

  const defaultBaseUrl = kind === 'github_enterprise'
    ? 'https://github.example.com'
    : 'https://gitlab.example.com';

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError('');
    if (!label.trim()) { setError('Label is required'); return; }
    if (!token.trim()) { setError('Token is required'); return; }
    if (needsBaseUrl && !baseUrl.trim()) { setError('Base URL is required'); return; }

    const url = kind === 'github' ? '' : baseUrl.trim().replace(/\/$/, '');

    setSaving(true);
    try {
      await addAccount(kind, url, label.trim(), token.trim());
      onAdded();
      onClose();
    } catch (err: unknown) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={e => e.stopPropagation()}>
        <button className="dialog-close" onClick={onClose}><XIcon /></button>
        <h2>Add account</h2>

        <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
          <div className="form-group">
            <label className="form-label">Provider</label>
            <select
              className="form-select"
              value={kind}
              onChange={e => { setKind(e.target.value as AccountKind); setBaseUrl(''); }}
            >
              {(Object.keys(KIND_LABELS) as AccountKind[]).map(k => (
                <option key={k} value={k}>{KIND_LABELS[k]}</option>
              ))}
            </select>
          </div>

          {needsBaseUrl && (
            <div className="form-group">
              <label className="form-label">Base URL</label>
              <input
                className="form-input"
                type="url"
                placeholder={defaultBaseUrl}
                value={baseUrl}
                onChange={e => setBaseUrl(e.target.value)}
              />
            </div>
          )}

          <div className="form-group">
            <label className="form-label">Label</label>
            <input
              className="form-input"
              type="text"
              placeholder="e.g. Work GitHub"
              value={label}
              onChange={e => setLabel(e.target.value)}
            />
          </div>

          <div className="form-group">
            <label className="form-label">Personal Access Token</label>
            <input
              className="form-input"
              type="password"
              placeholder="ghp_… or glpat-…"
              value={token}
              onChange={e => setToken(e.target.value)}
            />
            <span className="form-hint">
              {kind === 'gitlab'
                ? 'Needs: read_api scope'
                : 'Needs: repo, read:org, read:project scopes'}
            </span>
          </div>

          {error && <div className="form-error">{error}</div>}

          <div className="dialog-actions">
            <button type="button" className="btn" onClick={onClose}>Cancel</button>
            <button type="submit" className="btn btn-primary" disabled={saving}>
              {saving ? 'Saving…' : 'Add account'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
