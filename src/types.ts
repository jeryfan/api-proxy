export type RuleAction = "set" | "add" | "remove";

export interface Rule {
  action: RuleAction;
  key: string;
  value: string;
}

export type BodyMatchMode = "contains" | "regex";

export interface BodyMatch {
  mode: BodyMatchMode;
  pattern: string;
}

export interface HealthConfig {
  enabled: boolean;
  failureThreshold: number;
  countConnectError: boolean;
  statusCodes: number[];
  bodyMatch?: BodyMatch;
}

export interface UpstreamHealthState {
  endpointId: string;
  upstreamId: string;
  consecutiveFailures: number;
  isTripped: boolean;
  lastFailureAt?: number;
  lastFailureReason?: string;
}

export interface Upstream {
  id: string;
  name: string;
  url: string;
  enabled: boolean;
  weight: number;
  headerRules: Rule[];
  health: HealthConfig;
}

export interface Endpoint {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  path: string;
  upstreams: Upstream[];
  stripPrefix: boolean;
  fixedUpstream: boolean;
  headerRules: Rule[];
  queryRules: Rule[];
  sortIndex: number;
  createdAt: number;
}

export interface GlobalConfig {
  listenAddress: string;
  listenPort: number;
  requestTimeoutSecs: number;
  theme: "light" | "dark" | "system";
  closeToTray: boolean;
  autoStartServer: boolean;
  proxyUrl: string;
  logBufferCapacity: number;
}

export interface ServerStatus {
  running: boolean;
  listenAddress?: string;
  listenPort?: number;
}

export interface InitPayload {
  global: GlobalConfig;
  endpoints: Endpoint[];
  status: ServerStatus;
  health: Record<string, UpstreamHealthState>;
}

export interface ProxyTestResult {
  success: boolean;
  latencyMs: number;
  error?: string;
}

export interface DetectedProxy {
  url: string;
}

export interface HeaderEntry {
  key: string;
  value: string;
}

export interface RequestLog {
  id: string;
  endpointId: string;
  startedAt: number;
  clientAddr?: string;
  reqMethod: string;
  reqPath: string;
  reqQuery?: string;
  reqHeaders: HeaderEntry[];
  reqBodyB64: string;
  reqBodyLen: number;
  reqBodyBinary: boolean;
  upstreamUrl: string;
  upstreamHeaders: HeaderEntry[];
  failoverLog: string[];
  statusCode?: number;
  respHeaders: HeaderEntry[];
  upstreamRespHeaders: HeaderEntry[];
  respBodyB64: string;
  respBodyLen: number;
  respBodyBinary: boolean;
  respContentType?: string;
  durationMs: number;
  error?: string;
}
