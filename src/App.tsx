import { useState, useEffect, useCallback, useRef } from 'react';
import './App.css';
import type { Account, AgentSession, WorkItem, WorkList as WorkListType } from './lib/types';
import { listAccounts, fetchAll, getAgentSessions } from './lib/api';
import { diffPrStates } from './lib/notifications';
import { WorkList } from './components/WorkList';
import { SettingsPage } from './components/SettingsPage';
import { AgentsPage } from './components/AgentsPage';
import { AssignDialog } from './components/AssignDialog';
import { RefreshIcon, SettingsIcon, IssueIcon, PrIcon, AgentIcon } from './components/icons';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
import clsx from 'clsx';

const POLL_INTERVAL = 5 * 60 * 1000;

type Filter = 'all' | 'github' | 'github_enterprise' | 'gitlab';
type Page = 'issues' | 'prs' | 'reviews' | 'agents' | 'settings';

const FILTERS: { value: Filter; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'github', label: 'GitHub' },
  { value: 'github_enterprise', label: 'GHE' },
  { value: 'gitlab', label: 'GitLab' },
];

const STATE_LABEL: Record<string, string> = {
  approved: 'Approved',
  changes_requested: 'Changes Requested',
};

function applyOrgFilter(lists: WorkListType[], accounts: Account[]): WorkListType[] {
  const blockedByAccount = Object.fromEntries(
    accounts.map(a => [a.id, new Set(a.blockedOrgs.map(o => o.toLowerCase()))])
  );
  return lists.map(wl => {
    const blocked = blockedByAccount[wl.accountId];
    if (!blocked || blocked.size === 0) return wl;
    return {
      ...wl,
      items: wl.items.filter(item => {
        if (!item.repo) return true;
        const org = item.repo.split('/')[0].toLowerCase();
        return !blocked.has(org);
      }),
    };
  });
}

async function ensureNotificationPermission(): Promise<boolean> {
  if (await isPermissionGranted()) return true;
  const result = await requestPermission();
  return result === 'granted';
}

async function notifyStateChanges(
  prev: WorkItem[],
  next: WorkItem[],
) {
  const changes = diffPrStates(prev, next);
  if (changes.length === 0) return;
  if (!await ensureNotificationPermission()) return;

  for (const { item, to } of changes) {
    const repo = item.repo ? `${item.repo}` : 'a project';
    sendNotification({
      title: `PR ${STATE_LABEL[to] ?? to}`,
      body: `${item.title} (${repo}${item.number ? ` #${item.number}` : ''})`,
    });
  }
}

export default function App() {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [rawLists, setRawLists] = useState<WorkListType[]>([]);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState<Page>('agents');
  const [filter, setFilter] = useState<Filter>('all');
  const [lastFetch, setLastFetch] = useState<Date | null>(null);
  const [agentSessions, setAgentSessions] = useState<AgentSession[]>([]);
  const [assignTarget, setAssignTarget] = useState<WorkItem | null>(null);

  // Keep a ref of previous PR items for diffing — not state, doesn't need to trigger renders
  const prevPrsRef = useRef<WorkItem[]>([]);

  const loadAccounts = useCallback(async () => {
    const accs = await listAccounts();
    setAccounts(accs);
    return accs;
  }, []);

  const refresh = useCallback(async (accs?: Account[]) => {
    const current = accs ?? accounts;
    if (current.length === 0) return;
    setLoading(true);
    try {
      const lists = await fetchAll();
      const nextPrs = lists.flatMap(wl => wl.items.filter(i => i.kind === 'pr'));
      await notifyStateChanges(prevPrsRef.current, nextPrs);
      prevPrsRef.current = nextPrs;
      setRawLists(lists);
      setLastFetch(new Date());
    } finally {
      setLoading(false);
    }
  }, [accounts]);

  useEffect(() => {
    loadAccounts().then(accs => {
      if (accs.length > 0) refresh(accs);
      else setPage('settings');
    });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (accounts.length === 0) return;
    const id = setInterval(() => refresh(), POLL_INTERVAL);
    return () => clearInterval(id);
  }, [accounts, refresh]);

  useEffect(() => {
    const poll = async () => { try { setAgentSessions(await getAgentSessions()); } catch {} };
    poll();
    const id = setInterval(poll, 5000);
    return () => clearInterval(id);
  }, []);

  async function handleAccountsChanged() {
    const accs = await loadAccounts();
    if (accs.length > 0) refresh(accs);
    else setRawLists([]);
  }

  const workLists = applyOrgFilter(rawLists, accounts);
  const activeKinds = new Set(accounts.map(a => a.kind));
  const visibleFilters = FILTERS.filter(
    f => f.value === 'all' || activeKinds.has(f.value as Account['kind'])
  );
  const allItems = workLists.flatMap(wl => wl.items);
  const issueCount = allItems.filter(i => i.kind === 'issue' || i.kind === 'project_item').length;
  const prCount = allItems.filter(i => i.kind === 'pr').length;
  const reviewCount = allItems.filter(i => i.kind === 'review_request').length;
  const activeAgents = agentSessions.filter(s => s.status !== 'done').length;

  return (
    <div className="app">
      <div className="sidebar">
        <div className="sidebar-logo">Organiser</div>
        <nav className="sidebar-nav">
          <button
            className={clsx('nav-item', page === 'agents' && 'active')}
            onClick={() => setPage('agents')}
          >
            <AgentIcon />
            Agents
          </button>
          <button
            className={clsx('nav-item', page === 'prs' && 'active')}
            onClick={() => setPage('prs')}
          >
            <PrIcon />
            Pull Requests
            {prCount > 0 && (
              <span className="section-count" style={{ marginLeft: 'auto' }}>{prCount}</span>
            )}
          </button>
          <button
            className={clsx('nav-item', page === 'issues' && 'active')}
            onClick={() => setPage('issues')}
          >
            <IssueIcon />
            Issues
            {issueCount > 0 && (
              <span className="section-count" style={{ marginLeft: 'auto' }}>{issueCount}</span>
            )}
          </button>
          <button
            className={clsx('nav-item', page === 'reviews' && 'active')}
            onClick={() => setPage('reviews')}
          >
            <PrIcon />
            Review Requests
            {reviewCount > 0 && (
              <span className="section-count" style={{ marginLeft: 'auto' }}>{reviewCount}</span>
            )}
          </button>
        </nav>
        <div className="sidebar-actions">
          <button
            className={clsx('btn', page === 'settings' && 'btn-primary')}
            style={{ width: '100%' }}
            onClick={() => setPage(p => p === 'settings' ? 'agents' : 'settings')}
          >
            <SettingsIcon /> Settings
          </button>
        </div>
      </div>

      <div className="main">
        {page === 'settings' ? (
          <SettingsPage accounts={accounts} onChanged={handleAccountsChanged} />
        ) : page === 'agents' ? (
          <>
            <div className="toolbar">
              <span className="toolbar-title">Agents</span>
            </div>
            <AgentsPage />
          </>
        ) : (
          <>
            <div className="toolbar">
              <span className="toolbar-title">{page === 'issues' ? 'Issues' : page === 'prs' ? 'Pull Requests' : 'Review Requests'}</span>

              {visibleFilters.length > 1 && (
                <div className="toolbar-filter">
                  {visibleFilters.map(f => (
                    <button
                      key={f.value}
                      className={clsx('filter-btn', filter === f.value && 'active')}
                      onClick={() => setFilter(f.value)}
                    >
                      {f.label}
                    </button>
                  ))}
                </div>
              )}

              <button
                className={clsx('refresh-btn', loading && 'spinning')}
                onClick={() => refresh()}
                disabled={loading || accounts.length === 0}
                title={lastFetch ? `Last updated ${lastFetch.toLocaleTimeString()}` : 'Refresh'}
              >
                <RefreshIcon />
                {loading ? 'Refreshing…' : 'Refresh'}
              </button>
            </div>

            {loading && rawLists.length === 0 ? (
              <div className="work-list">
                {[...Array(6)].map((_, i) => (
                  <div key={i} className="shimmer shimmer-row" style={{ marginTop: i === 0 ? 20 : 4 }} />
                ))}
              </div>
            ) : (
              <WorkList
                accounts={accounts}
                workLists={workLists}
                filter={filter}
                view={page as 'issues' | 'prs' | 'reviews'}
                activeAgents={activeAgents}
                onAssign={activeAgents > 0 ? item => setAssignTarget(item) : undefined}
              />
            )}
          </>
        )}
      </div>
      {assignTarget && (
        <AssignDialog item={assignTarget} onClose={() => setAssignTarget(null)} />
      )}
    </div>
  );
}
