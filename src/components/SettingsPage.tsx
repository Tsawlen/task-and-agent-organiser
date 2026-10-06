import { useState, useEffect } from 'react';
import type { Account, AgentHookStatus, GheProjectMeta } from '../lib/types';
import { removeAccount, updateAccountOrgs, updateAccountProjects, fetchOrgs, fetchGheProjectsMeta, getAgentHookStatuses, setAgentHook } from '../lib/api';
import { AddAccountDialog } from './AddAccountDialog';
import { PlusIcon, TrashIcon, RefreshIcon } from './icons';

interface Props {
  accounts: Account[];
  onChanged: () => void;
}

type Tab = 'accounts' | 'filters' | 'projects' | 'onboarding';

const KIND_LABEL: Record<string, string> = {
  github: 'GitHub.com',
  github_enterprise: 'GitHub Enterprise',
  gitlab: 'GitLab',
};

function AccountsTab({ accounts, onChanged }: Props) {
  const [showAdd, setShowAdd] = useState(false);
  const [removing, setRemoving] = useState<string | null>(null);

  async function handleRemove(id: string) {
    setRemoving(id);
    await removeAccount(id);
    onChanged();
    setRemoving(null);
  }

  return (
    <div className="settings-section">
      <div className="settings-section-header">
        <span>Connected accounts</span>
        <button className="btn btn-primary btn-sm" onClick={() => setShowAdd(true)}>
          <PlusIcon /> Add account
        </button>
      </div>

      {accounts.length === 0 ? (
        <div className="settings-empty">No accounts yet. Add one to get started.</div>
      ) : (
        <div className="settings-account-list">
          {accounts.map(a => (
            <div key={a.id} className="settings-account-row">
              <div className={`provider-dot ${a.kind}`} />
              <div className="settings-account-info">
                <div className="settings-account-label">{a.label}</div>
                <div className="settings-account-meta">
                  {KIND_LABEL[a.kind]}
                  {a.baseUrl ? ` · ${a.baseUrl}` : ''}
                </div>
              </div>
              <button
                className="btn btn-sm btn-danger"
                onClick={() => handleRemove(a.id)}
                disabled={removing === a.id}
                title="Remove account"
              >
                <TrashIcon />
              </button>
            </div>
          ))}
        </div>
      )}

      {showAdd && (
        <AddAccountDialog onClose={() => setShowAdd(false)} onAdded={onChanged} />
      )}
    </div>
  );
}

function FiltersTab({ accounts, onChanged }: Props) {
  // orgs[accountId] = list of org names fetched from the provider
  const [orgs, setOrgs] = useState<Record<string, string[]>>({});
  const [loading, setLoading] = useState<Record<string, boolean>>({});
  const [saving, setSaving] = useState<string | null>(null);

  async function loadOrgs(account: Account) {
    setLoading(l => ({ ...l, [account.id]: true }));
    try {
      const result = await fetchOrgs(account.id);
      setOrgs(o => ({ ...o, [account.id]: result }));
    } finally {
      setLoading(l => ({ ...l, [account.id]: false }));
    }
  }

  // Load orgs for all accounts on mount
  useEffect(() => {
    accounts.forEach(a => loadOrgs(a));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  async function toggleOrg(account: Account, org: string, block: boolean) {
    setSaving(account.id);
    const next = block
      ? [...account.blockedOrgs, org]
      : account.blockedOrgs.filter(o => o !== org);
    await updateAccountOrgs(account.id, next);
    onChanged();
    setSaving(null);
  }

  if (accounts.length === 0) {
    return <div className="settings-section"><div className="settings-empty">Add an account first.</div></div>;
  }

  return (
    <div className="settings-section">
      <div className="settings-section-header"><span>Blocked organisations</span></div>
      <p className="settings-hint">
        Checked organisations are hidden from the work list. Changes are saved immediately.
      </p>

      {accounts.map(account => {
        const available = orgs[account.id] ?? [];
        const isLoading = loading[account.id] ?? false;
        const isSaving = saving === account.id;

        return (
          <div key={account.id} className="settings-org-group">
            <div className="settings-org-group-label">
              <div className={`provider-dot ${account.kind}`} />
              <span>{account.label}</span>
              <button
                className={`btn btn-sm btn-icon ${isLoading ? 'spinning' : ''}`}
                style={{ marginLeft: 'auto' }}
                onClick={() => loadOrgs(account)}
                disabled={isLoading}
                title="Reload organisations"
              >
                <RefreshIcon />
              </button>
            </div>

            {isLoading ? (
              <div className="org-loading">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className="shimmer" style={{ height: 28, borderRadius: 6, marginBottom: 4 }} />
                ))}
              </div>
            ) : available.length === 0 ? (
              <div className="settings-empty" style={{ padding: '8px 0', fontSize: 12 }}>
                No organisations found — check token scopes.
              </div>
            ) : (
              <div className="org-checklist">
                {available.map(org => {
                  const blocked = account.blockedOrgs.includes(org);
                  return (
                    <label key={org} className={`org-check-row ${blocked ? 'blocked' : ''}`}>
                      <input
                        type="checkbox"
                        className="org-checkbox"
                        checked={blocked}
                        disabled={isSaving}
                        onChange={e => toggleOrg(account, org, e.target.checked)}
                      />
                      <span className="org-check-name">{org}</span>
                      {blocked && <span className="org-check-badge">hidden</span>}
                    </label>
                  );
                })}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

function ProjectsTab({ accounts, onChanged }: Props) {
  const [projects, setProjects] = useState<Record<string, GheProjectMeta[]>>({});
  const [loading, setLoading] = useState<Record<string, boolean>>({});

  async function loadProjects(account: Account) {
    setLoading(l => ({ ...l, [account.id]: true }));
    try {
      const result = await fetchGheProjectsMeta(account.id);
      setProjects(p => ({ ...p, [account.id]: result }));
    } finally {
      setLoading(l => ({ ...l, [account.id]: false }));
    }
  }

  useEffect(() => {
    accounts.forEach(a => loadProjects(a));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  async function toggleProject(account: Account, projectId: string, selected: boolean) {
    const next = selected
      ? [...account.selectedProjects, projectId]
      : account.selectedProjects.filter(id => id !== projectId);
    await updateAccountProjects(account.id, next);
    onChanged();
  }

  if (accounts.length === 0) {
    return <div className="settings-section"><div className="settings-empty">No GitHub Enterprise accounts.</div></div>;
  }

  return (
    <div className="settings-section">
      <div className="settings-section-header"><span>GitHub Enterprise Projects</span></div>
      <p className="settings-hint">
        Select which project boards to fetch assigned issues from. Only items with status "In Progress", "Sprint Backlog", or "In Review" are shown.
      </p>

      {accounts.map(account => {
        const available = projects[account.id] ?? [];
        const isLoading = loading[account.id] ?? false;

        return (
          <div key={account.id} className="settings-org-group">
            <div className="settings-org-group-label">
              <div className={`provider-dot ${account.kind}`} />
              <span>{account.label}</span>
              <button
                className={`btn btn-sm btn-icon ${isLoading ? 'spinning' : ''}`}
                style={{ marginLeft: 'auto' }}
                onClick={() => loadProjects(account)}
                disabled={isLoading}
                title="Reload projects"
              >
                <RefreshIcon />
              </button>
            </div>

            {isLoading ? (
              <div className="org-loading">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className="shimmer" style={{ height: 28, borderRadius: 6, marginBottom: 4 }} />
                ))}
              </div>
            ) : available.length === 0 ? (
              <div className="settings-empty" style={{ padding: '8px 0', fontSize: 12 }}>
                No projects found.
              </div>
            ) : (
              <div className="org-checklist">
                {available.map(proj => {
                  const selected = account.selectedProjects.includes(proj.id);
                  return (
                    <label key={proj.id} className={`org-check-row ${selected ? 'blocked' : ''}`}>
                      <input
                        type="checkbox"
                        className="org-checkbox"
                        checked={selected}
                        onChange={e => toggleProject(account, proj.id, e.target.checked)}
                      />
                      <span className="org-check-name">{proj.title}</span>
                      <span className="org-check-badge" style={{ opacity: 0.5 }}>{proj.org}</span>
                      {selected && <span className="org-check-badge">active</span>}
                    </label>
                  );
                })}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

function OnboardingTab() {
  const [statuses, setStatuses] = useState<AgentHookStatus[]>([]);
  const [toggling, setToggling] = useState<string | null>(null);
  const [errors, setErrors] = useState<Record<string, string>>({});

  async function load() {
    try { setStatuses(await getAgentHookStatuses()); } catch { /* ignore */ }
  }

  useEffect(() => { load(); }, []);

  async function toggle(agent: AgentHookStatus) {
    setToggling(agent.id);
    setErrors(e => ({ ...e, [agent.id]: '' }));
    try {
      await setAgentHook(agent.id, !agent.installed);
      await load();
    } catch (e: any) {
      setErrors(err => ({ ...err, [agent.id]: String(e) }));
    } finally {
      setToggling(null);
    }
  }

  const SUPPORT_LABEL: Record<AgentHookStatus['support'], string> = {
    supported: 'Config file',
    file_required: 'Plugin file',
    unsupported: 'No hook support',
  };

  return (
    <div className="settings-section">
      <div className="settings-section-header"><span>Agent hooks</span></div>
      <p className="settings-hint">
        Connect coding agents so their sessions appear on the Agents page.
        The app writes a small hook into each agent's config that posts events
        to the local server at <code>127.0.0.1:27384</code>.
      </p>

      <div className="settings-account-list">
        {statuses.map(agent => (
          <div key={agent.id} className="settings-account-row">
            <div style={{
              width: 8, height: 8, borderRadius: '50%', flexShrink: 0,
              background: agent.support === 'unsupported' ? '#6e7681'
                : agent.installed ? '#3fb950' : '#e3b341',
            }} />
            <div className="settings-account-info">
              <div className="settings-account-label">{agent.name}</div>
              <div className="settings-account-meta">
                {SUPPORT_LABEL[agent.support]}
                {agent.configPath ? ` · ${agent.configPath}` : ''}
              </div>
              {errors[agent.id] && (
                <div className="form-error" style={{ marginTop: 2 }}>{errors[agent.id]}</div>
              )}
            </div>
            {agent.support !== 'unsupported' && (
              <button
                className={`btn btn-sm ${agent.installed ? 'btn-danger' : 'btn-primary'}`}
                disabled={toggling === agent.id}
                onClick={() => toggle(agent)}
              >
                {toggling === agent.id ? '…' : agent.installed ? 'Remove' : 'Install'}
              </button>
            )}
            {agent.support === 'unsupported' && (
              <span style={{ fontSize: 11, color: '#6e7681' }}>Not supported</span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

export function SettingsPage({ accounts, onChanged }: Props) {
  const [tab, setTab] = useState<Tab>('accounts');
  const gheAccounts = accounts.filter(a => a.kind === 'github_enterprise');

  return (
    <div className="settings-page">
      <div className="settings-sidebar">
        <div className="settings-title">Settings</div>
        <nav>
          {(['accounts', 'filters', 'onboarding'] as Tab[]).map(t => (
            <button
              key={t}
              className={`settings-nav-item ${tab === t ? 'active' : ''}`}
              onClick={() => setTab(t)}
            >
              {t === 'accounts' ? 'Accounts' : t === 'filters' ? 'Filters' : 'Onboarding'}
            </button>
          ))}
          {gheAccounts.length > 0 && (
            <button
              className={`settings-nav-item ${tab === 'projects' ? 'active' : ''}`}
              onClick={() => setTab('projects')}
            >
              Projects
            </button>
          )}
        </nav>
      </div>
      <div className="settings-content">
        {tab === 'accounts' && <AccountsTab accounts={accounts} onChanged={onChanged} />}
        {tab === 'filters' && <FiltersTab accounts={accounts} onChanged={onChanged} />}
        {tab === 'projects' && <ProjectsTab accounts={gheAccounts} onChanged={onChanged} />}
        {tab === 'onboarding' && <OnboardingTab />}
      </div>
    </div>
  );
}
