export type RuleAction = "set" | "add" | "remove";

export type ApiFormat =
  | "passthrough"
  | "anthropicToOpenaiChat"
  | "anthropicToOpenaiResponses"
  | "anthropicToGemini"
  | "responsesToOpenaiChat";

export interface Rule {
  action: RuleAction;
  key: string;
  value: string;
}

export interface Endpoint {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  path: string;
  upstreamUrl: string;
  stripPrefix: boolean;
  fixedUpstream: boolean;
  headerRules: Rule[];
  queryRules: Rule[];
  bodyMerge: string;
  sortIndex: number;
  apiFormat: ApiFormat;
  createdAt: number;
  updatedAt: number;
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
  startedAt?: number;
  lastError?: string;
}

export interface InitPayload {
  global: GlobalConfig;
  endpoints: Endpoint[];
  status: ServerStatus;
}

export interface ProxyTestResult {
  success: boolean;
  latencyMs: number;
  error?: string;
}

export interface DetectedProxy {
  url: string;
  proxyType: string;
  port: number;
}

export interface HeaderEntry {
  key: string;
  value: string;
}

export interface RequestLog {
  id: string;
  endpointId: string;
  endpointName: string;
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
  upstreamBodyB64: string;
  upstreamBodyLen: number;
  statusCode?: number;
  respHeaders: HeaderEntry[];
  respBodyB64: string;
  respBodyLen: number;
  respBodyBinary: boolean;
  respContentType?: string;
  durationMs: number;
  error?: string;
}
