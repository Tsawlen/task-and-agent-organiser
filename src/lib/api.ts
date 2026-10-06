import { invoke } from '@tauri-apps/api/core';
import type { Account, AccountKind, AgentHookStatus, AgentSession, GheProjectMeta, WorkList } from './types';

export const listAccounts = (): Promise<Account[]> =>
  invoke('list_accounts');

export const addAccount = (
  kind: AccountKind,
  baseUrl: string,
  label: string,
  token: string,
): Promise<Account> =>
  invoke('add_account', { kind, baseUrl: baseUrl, label, token });

export const removeAccount = (id: string): Promise<void> =>
  invoke('remove_account', { id });

export const fetchWork = (accountId: string): Promise<WorkList> =>
  invoke('fetch_work', { accountId });

export const fetchAll = (): Promise<WorkList[]> =>
  invoke('fetch_all');

export const updateAccountOrgs = (id: string, blockedOrgs: string[]): Promise<void> =>
  invoke('update_account_orgs', { id, blockedOrgs });

export const updateAccountProjects = (id: string, selectedProjects: string[]): Promise<void> =>
  invoke('update_account_projects', { id, selectedProjects });

export const fetchOrgs = (accountId: string): Promise<string[]> =>
  invoke('fetch_orgs', { accountId });

export const fetchGheProjectsMeta = (accountId: string): Promise<GheProjectMeta[]> =>
  invoke('fetch_ghe_projects_meta', { accountId });

export const getAgentSessions = (): Promise<AgentSession[]> =>
  invoke('get_agent_sessions');

export const dismissDoneSessions = (): Promise<void> =>
  invoke('dismiss_done_sessions');

export const getAgentHookStatuses = (): Promise<AgentHookStatus[]> =>
  invoke('get_agent_hook_statuses');

export const setAgentHook = (agentId: string, install: boolean): Promise<void> =>
  invoke('set_agent_hook', { agentId, install });

export const assignToAgent = (
  accountId: string,
  repo: string,
  number: number,
  title: string,
  url: string,
  localPath: string,
): Promise<void> =>
  invoke('assign_to_agent', { accountId, repo, number, title, url, localPath });
