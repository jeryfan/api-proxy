import * as React from "react";
import { listen } from "@tauri-apps/api/event";
import {
  AlertCircle,
  ArrowRight,
  Clock,
  Loader2,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { api, EVENTS } from "@/lib/api";
import { cn } from "@/lib/utils";
import type { Endpoint, HeaderEntry, RequestLog } from "@/types";

interface Props {
  endpoint: Endpoint | null;
  onClose: () => void;
}

type Tab = "reqHeaders" | "reqBody" | "respHeaders" | "respBody";

function base64ToText(b64: string): string {
  if (!b64) return "";
  try {
    const bin = atob(b64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
  } catch {
    return "";
  }
}

function tryFormatJson(text: string): string {
  const trimmed = text.trim();
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) return text;
  try {
    return JSON.stringify(JSON.parse(trimmed), null, 2);
  } catch {
    return text;
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function statusTone(
  code?: number,
): "emerald" | "violet" | "amber" | "red" | "slate" {
  if (!code) return "slate";
  if (code < 200) return "violet";
  if (code < 300) return "emerald";
  if (code < 400) return "violet";
  if (code < 500) return "amber";
  return "red";
}

function HeaderTable({ headers }: { headers: HeaderEntry[] }) {
  if (headers.length === 0) {
    return <p className="text-sm text-muted-foreground">无</p>;
  }
  return (
    <div className="rounded-md border overflow-hidden">
      <table className="w-full text-sm font-mono">
        <tbody>
          {headers.map((h, idx) => (
            <tr
              key={`${h.key}-${idx}`}
              className="border-b last:border-b-0 hover:bg-muted/40"
            >
              <td className="px-3 py-1.5 text-muted-foreground align-top w-1/3 break-all">
                {h.key}
              </td>
              <td className="px-3 py-1.5 break-all whitespace-pre-wrap">
                {h.value}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function BodyView({
  b64,
  binary,
  truncated,
  len,
}: {
  b64: string;
  binary: boolean;
  truncated: boolean;
  len: number;
}) {
  if (binary) {
    return (
      <p className="text-sm text-muted-foreground">
        二进制响应，未记录 body（{formatSize(len)}）。
      </p>
    );
  }
  if (len === 0) {
    return <p className="text-sm text-muted-foreground">空</p>;
  }
  const text = base64ToText(b64);
  const formatted = tryFormatJson(text);
  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <span>{formatSize(len)}</span>
        {truncated && (
          <Badge tone="amber">
            已截断（仅前 {formatSize(b64.length)} 显示）
          </Badge>
        )}
      </div>
      <pre className="rounded-md border bg-muted/30 px-3 py-2 text-xs font-mono whitespace-pre-wrap break-all max-h-[60vh] overflow-y-auto">
        {formatted}
      </pre>
    </div>
  );
}

export function RequestLogPanel({ endpoint, onClose }: Props) {
  const [logs, setLogs] = React.useState<RequestLog[]>([]);
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [tab, setTab] = React.useState<Tab>("reqHeaders");
  const [loading, setLoading] = React.useState(false);

  const refresh = React.useCallback(async () => {
    if (!endpoint) return;
    setLoading(true);
    try {
      const data = await api.listRequestLogs(endpoint.id);
      setLogs(data);
      if (data.length > 0 && !selectedId) {
        setSelectedId(data[0].id);
      }
    } finally {
      setLoading(false);
    }
  }, [endpoint, selectedId]);

  React.useEffect(() => {
    if (endpoint) {
      setSelectedId(null);
      refresh();
    } else {
      setLogs([]);
      setSelectedId(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [endpoint?.id]);

  React.useEffect(() => {
    if (!endpoint) return;
    const unlisten = listen<RequestLog>(EVENTS.LOG_RECORDED, (e) => {
      if (e.payload.endpointId !== endpoint.id) return;
      setLogs((prev) => [e.payload, ...prev].slice(0, 200));
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [endpoint]);

  const handleClear = async () => {
    if (!endpoint) return;
    try {
      await api.clearRequestLogs(endpoint.id);
      setLogs([]);
      setSelectedId(null);
      toast.success("已清空日志");
    } catch (e: unknown) {
      toast.error(typeof e === "string" ? e : (e as Error)?.message ?? "失败");
    }
  };

  const selected = logs.find((l) => l.id === selectedId);

  return (
    <FullScreenPanel
      open={endpoint != null}
      onClose={onClose}
      title={endpoint ? `请求日志：${endpoint.name}` : ""}
      footer={
        <>
          <span className="text-xs text-muted-foreground mr-auto">
            共 {logs.length} 条（最多保留 200 条，重启清空）
          </span>
          <Button variant="outline" size="sm" onClick={refresh} disabled={loading}>
            {loading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
            刷新
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleClear}
            disabled={logs.length === 0}
            className="hover:text-red-500"
          >
            <Trash2 className="h-4 w-4" />
            清空
          </Button>
        </>
      }
    >
      <div className="flex h-full -mx-6 -my-6 min-h-0">
        <aside className="w-[26rem] flex-shrink-0 border-r overflow-y-auto">
          {logs.length === 0 ? (
            <div className="p-6 text-sm text-muted-foreground">
              暂无请求记录
            </div>
          ) : (
            <ul className="divide-y">
              {logs.map((log) => (
                <li
                  key={log.id}
                  onClick={() => setSelectedId(log.id)}
                  className={cn(
                    "px-4 py-3 cursor-pointer hover:bg-muted/40",
                    selectedId === log.id && "bg-muted/60",
                  )}
                >
                  <div className="flex items-center gap-2">
                    <Badge tone={statusTone(log.statusCode)}>
                      {log.statusCode ?? "ERR"}
                    </Badge>
                    <span className="font-mono text-xs text-muted-foreground">
                      {log.reqMethod}
                    </span>
                    <span className="font-mono text-sm truncate flex-1">
                      {log.reqPath}
                    </span>
                  </div>
                  <div className="mt-1 flex items-center gap-3 text-xs text-muted-foreground">
                    <span className="inline-flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      {new Date(log.startedAt).toLocaleTimeString()}
                    </span>
                    <span>{log.durationMs} ms</span>
                    {log.error && (
                      <span className="inline-flex items-center gap-1 text-red-500">
                        <AlertCircle className="h-3 w-3" />
                        错误
                      </span>
                    )}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </aside>

        <section className="flex-1 min-w-0 overflow-y-auto px-6 py-6 space-y-4">
          {!selected ? (
            <p className="text-sm text-muted-foreground">从左侧选择一条请求查看详情</p>
          ) : (
            <>
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <Badge tone={statusTone(selected.statusCode)}>
                    {selected.statusCode ?? "ERR"}
                  </Badge>
                  <span className="font-mono text-base font-semibold">
                    {selected.reqMethod}
                  </span>
                  <span className="font-mono text-sm truncate text-muted-foreground">
                    {selected.reqPath}
                    {selected.reqQuery ? `?${selected.reqQuery}` : ""}
                  </span>
                  <ArrowRight className="h-4 w-4 text-muted-foreground" />
                  <span className="font-mono text-sm truncate text-muted-foreground flex-1">
                    {selected.upstreamUrl}
                  </span>
                </div>
                <div className="flex flex-wrap items-center gap-4 text-xs text-muted-foreground">
                  <span>{new Date(selected.startedAt).toLocaleString()}</span>
                  <span>{selected.durationMs} ms</span>
                  {selected.clientAddr && <span>来源 {selected.clientAddr}</span>}
                  <span>请求体 {formatSize(selected.reqBodyLen)}</span>
                  <span>响应体 {formatSize(selected.respBodyLen)}</span>
                  {selected.respContentType && (
                    <span className="font-mono">{selected.respContentType}</span>
                  )}
                </div>
                {selected.error && (
                  <div className="rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm text-red-600 dark:text-red-400">
                    {selected.error}
                  </div>
                )}
              </div>

              <div className="flex gap-2 border-b">
                {(
                  [
                    ["reqHeaders", "请求头"],
                    ["reqBody", "请求体"],
                    ["respHeaders", "响应头"],
                    ["respBody", "响应体"],
                  ] as [Tab, string][]
                ).map(([key, label]) => (
                  <button
                    key={key}
                    onClick={() => setTab(key)}
                    className={cn(
                      "px-3 py-2 text-sm transition-colors -mb-[1px] border-b-2",
                      tab === key
                        ? "border-blue-500 text-foreground"
                        : "border-transparent text-muted-foreground hover:text-foreground",
                    )}
                  >
                    {label}
                  </button>
                ))}
              </div>

              <div className="space-y-2">
                {tab === "reqHeaders" && (
                  <HeaderTable headers={selected.reqHeaders} />
                )}
                {tab === "reqBody" && (
                  <BodyView
                    b64={selected.reqBodyB64}
                    binary={selected.reqBodyBinary}
                    truncated={selected.reqBodyTruncated}
                    len={selected.reqBodyLen}
                  />
                )}
                {tab === "respHeaders" && (
                  <HeaderTable headers={selected.respHeaders} />
                )}
                {tab === "respBody" && (
                  <BodyView
                    b64={selected.respBodyB64}
                    binary={selected.respBodyBinary}
                    truncated={selected.respBodyTruncated}
                    len={selected.respBodyLen}
                  />
                )}
              </div>
            </>
          )}
        </section>
      </div>
    </FullScreenPanel>
  );
}
