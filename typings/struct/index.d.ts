declare namespace Struct {
  export interface ThirdParty {
    client_name: string;
    url: string;
    img: string;
    opensource: number;
    download_url: string;
    weibu_url: string;
    logo: string;
    verify: string;
    md5: string;
    tag: string;
    author: string;
    description: string;
  }
  export interface UserInfo {
    email: string;
    friendlyGroup: string;
    group: string;
    id: number;
    inLimit: number;
    outLimit: number;
    proxies: number;
    realname: boolean;
    regTime: number;
    token: string;
    traffic: number;
    used: number;
    username: string;
  }
  export interface SignInfo {
    sign_max: string;
    sign_min: string;
    signdate: number;
    todaysign: number;
    totalsign: string;
    totaltraffic: string;
  }
  // After Content by Yue-cn on 2023-8-13 ~ 2023-8-20
  export interface UserAllGroup {
    GExpire: number[];
    GFriendlyName: string;
    GID: number;
    GInLimit: number;
    GName: string;
    GOutLimit: number;
    GProxies: number;
  }
  export interface UserProxy {
    connectAddress: string;
    nodeHostname: string;
    extAddress: string[];
    custom: string;
    domain: string;
    friendlyNode: string;
    /// <summary>
    /// Host 重写
    /// </summary>
    // Disabled headerXFromWhere: string;
    /// <summary>
    /// 请求来源
    /// </summary>
    // Disabled hostHeaderRewrite: string;
    id: bigint;
    lastLogin: bigint;
    lastupdate: bigint;
    localIp: string;
    localPort: number;
    // Disabled locations: string;
    nid: number;
    online: boolean;
    proxyName: string;
    proxyType: string;
    remotePort: number;
    status: boolean;
    useCompression: boolean;
    useEncryption: boolean;
    autoTls: string;
    forceHttps: boolean;
    proxyProtocolVersion: boolean;
    // Disabled sk: string;
  }
  export interface RefreshProxyState {
    p_id: bigint;
    P_online: boolean;
  }
  export interface EditOrNewUserProxy {
    custom: string;
    dataEncrypt: boolean;
    dataGzip: boolean;
    domain_bind: string;
    // Disabled host_rewrite: string;
    local_addr: string;
    local_port: number | null;
    name: string;
    node_id: number;
    remote_port: number;
    // Disabled request_from: string;
    // Disabled request_pass: string;
    type: string;
    // Disabled url_route: string;
    // 创建时传入 -1;
    autoTls: string;
    forceHttps: boolean;
    proxyProtocolVersion: boolean;
    proxy_id: bigint | undefined;
  }
  export interface AppOrder {
    gid: number;
    name: string;
    package: OrderPackage[];
    description: string;
  }
  export interface OrderPackage {
    gid: number;
    id: number;
    name: string;
    originalPrice: 20;
    sellPrice: 20;
  }
  export interface Node {
    allowEc: boolean;
    allowPort: string | null;
    fullyLoaded: boolean;
    bandwidth: number;
    bandwidthMagnification: number;
    classify: number;
    comments: string;
    group: string;
    id: number;
    maxOnlineMagnification: number;
    name: string;
    needRealname: boolean;
    port: number;
    status: number;
    unitcostEc: number;
    description: string;
    protocolSupport: {
      tcp: boolean;
      udp: boolean;
      http: boolean;
      https: boolean;
    };
  }
  export interface NodeState {
    client_counts: number;
    cur_conns: number;
    last_update: number;
    load_1: number;
    load_5: number;
    load_15: number;
    m_used_p: number;
    maxonline: number;
    name: string;
    nid: number;
    proxy_type_count: string;
    status: number;
    total_traffic_in: bigint;
    total_traffic_out: bigint;
    uptime: bigint;
    version: string;
  }
  export interface AppUser {
    email: string;
    id: bigint;
    proxies: number;
    realname: boolean;
    regtime: bigint;
    status: number;
    traffic: number;
    username: string;
  }
  export interface SearchForProxy {
    n_name: string;
    p_domain: string;
    p_id: bigint;
    p_lastlogin: bigint;
    p_lastupdate: bigint;
    p_name: string;
    p_online: boolean;
    p_remoteport: number;
    p_status: number;
    p_type: string;
    u_id: number;
    u_username: string;
  }

  export interface nodeDataModel {
    id: number | null;
    name: string;
    comments: string;
    hostname: string;
    hostnameExt: string;
    port: number;
    adminPort: number;
    adminPass: string;
    token: string;
    group: string;
    status: number;
    forceHost: string;
    needRealname: boolean;
    classify: number;
    dashSsl: boolean;
    bandwidth: number;
    tcp: boolean;
    udp: boolean;
    http: boolean;
    https: boolean;
    allowEc: boolean;
    unitcostEc: number;
    bandwidthMagnification: number;
    maxOnlineMagnification: number;
    allowPort: string;
    enableDefaultTLS: boolean;
    tlsSNIOverwrite: string | null;
  }
  // End
  // After Content by Yue-cn on 2024-7-29 to be continue...
  interface Software {
    latest: string;
    latest_full: string;
    latest_ver: string;
    latest_msg: string;
    common_details: string;
    launcherUpdate: {
      latest: string;
      argument: string;
      msg: string;
      title: string;
    };
    source: Array<{
      label: string;
      value: string;
    }>;
    soft: Array<{
      os: string;
      label: string;
      arch: Array<{
        label: string;
        details: string;
        file: string;
      }>;
    }>;
  }
}
