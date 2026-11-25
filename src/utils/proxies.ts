export function hostAndPort(host: string, port: number): string {
  return `${host.includes(':') && !host.startsWith('[') ? `[${host}]` : host}:${port}`;
}

export function proxyRemoteAddress(proxy: Struct.UserProxy): string {
  const addresses = (proxy.connectAddress || '').trim();
  if (!addresses || !['tcp', 'udp'].includes(proxy.proxyType.toLowerCase()) || proxy.remotePort <= 0) {
    return addresses;
  }
  return addresses.split('\n').map(value => {
    const address = value.trim();
    if (!address || /^\[[^\]]+\]:\d+$/.test(address) || /^[^:]+:\d+$/.test(address)) return address;
    return hostAndPort(address, proxy.remotePort);
  }).join('\n');
}

export function proxyDomains(value: string): string[] {
  if (!value) return [];
  try {
    const parsed: unknown = JSON.parse(value);
    if (Array.isArray(parsed)) return parsed.filter((domain): domain is string => typeof domain === 'string');
    if (typeof parsed === 'string') return [parsed];
  } catch {
    return value.split(',').map(domain => domain.trim()).filter(Boolean);
  }
  return [];
}

export function groupAllowsNode(group: string, node: Struct.Node): boolean {
  return node.group.split(';').some(value => value.trim() === group);
}
