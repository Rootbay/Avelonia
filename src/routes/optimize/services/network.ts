import { invoke } from '@tauri-apps/api/core';

export type NetworkAdapterInfo = {
  name: string;
  status?: string | null;
  linkSpeed?: string | null;
  mac?: string | null;
  media?: string | null;
  linkState?: string | null;
};

export type NetworkSummary = {
  primaryAdapter?: string | null;
  ipv4?: string | null;
  ipv6?: string | null;
  dnsServers: string[];
  gateways: string[];
  adapters: NetworkAdapterInfo[];
};

export const NETWORK_PRESETS = [
  {
    id: 'refresh',
    label: 'Quick refresh',
    description: 'Flush DNS and renew DHCP to recover connectivity quickly.',
    actions: ['flush_dns', 'renew_ip'] as const,
  },
  {
    id: 'full',
    label: 'Full reset',
    description: 'Flush DNS, reset Winsock, and renew IP to clear stubborn issues.',
    actions: ['flush_dns', 'reset_winsock', 'renew_ip'] as const,
  },
  {
    id: 'winsock',
    label: 'Winsock focus',
    description: 'Reset Winsock and renew IP without flushing DNS for driver resets.',
    actions: ['reset_winsock', 'renew_ip'] as const,
  },
] as const;

export type NetworkPresetId = (typeof NETWORK_PRESETS)[number]['id'];
export type NetworkPreset = (typeof NETWORK_PRESETS)[number];

export async function flushDns(): Promise<string> {
  return (await invoke('flush_dns')) as string;
}

export async function resetWinsock(): Promise<string> {
  return (await invoke('reset_winsock')) as string;
}

export async function renewIp(): Promise<string> {
  return (await invoke('renew_ip')) as string;
}

export async function getNetworkSummary(): Promise<NetworkSummary> {
  return await invoke<NetworkSummary>('get_network_summary');
}

export async function runPing(host: string, count = 4): Promise<string> {
  return (await invoke('run_ping', { host, count })) as string;
}

export async function runTraceroute(host: string): Promise<string> {
  return (await invoke('run_traceroute', { host })) as string;
}

export async function runDnsLookup(host: string): Promise<string> {
  return (await invoke('run_dns_lookup', { host })) as string;
}
