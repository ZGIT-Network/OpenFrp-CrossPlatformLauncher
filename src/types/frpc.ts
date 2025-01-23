export interface FrpcConfig {
  id: string;
  token: string;
  tunnelId: string;
  status: 'running' | 'stopped';
}

export interface ProxyInfo {
  name: string;
  id: number;
  type: string;
  remote: string;
  local: string;
}

export interface NodeInfo {
  node: string;
  proxies: ProxyInfo[];
}

export interface ProxyListResponse {
  status: number;
  success: boolean;
  message: string;
  data: NodeInfo[];
} 