import * as React from "react";
import { listen } from "@tauri-apps/api/event";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  AlertCircle,
  ArrowRight,
  Clock,
  Copy,
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
type BodyView = "raw" | "transformed";

function headersEqual(a: HeaderEntry[], b: HeaderEntry[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i].key !== b[i].key || a[i].value !== b[i].value) return false;
  }
  return true;
}

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

function detectFormat(
  text: string,
  contentType?: string,
): "json" | "form" | "sse" | "xml" | "text" {
  const ct = (contentType ?? "").toLowerCase();
  if (ct.includes("application/json") || ct.includes("+json")) return "json";
  if (ct.includes("application/x-www-form-urlencoded")) return "form";
  if (ct.includes("text/event-stream")) return "sse";
  if (ct.includes("xml")) return "xml";
  const trimmed = text.trim();
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) return "json";
  if (trimmed.startsWith("<")) return "xml";
  return "text";
}

function indentXml(input: string): string {
  let out = "";
  let depth = 0;
  const tokens = input.replace(/>\s*</g, ">\n<").split("\n");
  for (const tok of tokens) {
    const line = tok.trim();
    if (!line) continue;
    if (line.startsWith("</")) depth = Math.max(depth - 1, 0);
    out += "  ".repeat(depth) + line + "\n";
    if (
      line.startsWith("<") &&
      !line.startsWith("</") &&
      !line.startsWith("<?") &&
      !line.endsWith("/>") &&
      !line.includes("</")
    ) {
      depth += 1;
    }
  }
  return out.trimEnd();
}

function parseSSE(text: string): { event?: string; data: string; id?: string; raw: string }[] {
  const blocks = text.split(/\r?\n\r?\n/);
  const events: { event?: string; data: string; id?: string; raw: string }[] = [];
  for (const block of blocks) {
    const trimmed = block.trim();
    if (!trimmed) continue;
    let event: string | undefined;
    let id: string | undefined;
    const dataLines: string[] = [];
    for (const line of trimmed.split(/\r?\n/)) {
      if (line.startsWith("event:")) event = line.slice(6).trim();
      else if (line.startsWith("id:")) id = line.slice(3).trim();
      else if (line.startsWith("data:")) dataLines.push(line.slice(5).trim());
    }
    events.push({
      event,
      id,
      data: dataLines.join("\n"),
      raw: trimmed,
    });
  }
  return events;
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

async function copyBodyToClipboard(b64: string, binary: boolean) {
  if (binary) {
    toast.info("二进制内容无法复制为文本");
    return;
  }
  const text = base64ToText(b64);
  try {
    await writeText(text);
    toast.success("已复制到剪贴板");
  } catch (e) {
    toast.error(`复制失败：${(e as Error)?.message ?? e}`);
  }
}

function BodyViewToggle({
  value,
  onChange,
  rawLabel,
  transformedLabel,
}: {
  value: BodyView;
  onChange: (v: BodyView) => void;
  rawLabel: string;
  transformedLabel: string;
}) {
  return (
    <div className="inline-flex items-center rounded-md bg-muted p-0.5 text-xs">
      {(
        [
          ["raw", rawLabel],
          ["transformed", transformedLabel],
        ] as [BodyView, string][]
      ).map(([key, label]) => (
        <button
          key={key}
          type="button"
          onClick={() => onChange(key)}
          className={cn(
            "px-3 py-1 rounded-sm transition-colors",
            value === key
              ? "bg-background text-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

function RequestHeadersTab({
  log,
  view,
  onView,
}: {
  log: RequestLog;
  view: BodyView;
  onView: (v: BodyView) => void;
}) {
  const showToggle = !headersEqual(log.reqHeaders, log.upstreamHeaders);
  return (
    <div className="space-y-2">
      {showToggle && (
        <BodyViewToggle
          value={view}
          onChange={onView}
          rawLabel="原始（客户端发的）"
          transformedLabel="转换后（发往上游的）"
        />
      )}
      <HeaderTable
        headers={view === "raw" ? log.reqHeaders : log.upstreamHeaders}
      />
    </div>
  );
}

function ResponseHeadersTab({
  log,
  view,
  onView,
}: {
  log: RequestLog;
  view: BodyView;
  onView: (v: BodyView) => void;
}) {
  const showToggle = !headersEqual(log.upstreamRespHeaders, log.respHeaders);
  return (
    <div className="space-y-2">
      {showToggle && (
        <BodyViewToggle
          value={view}
          onChange={onView}
          rawLabel="原始（上游返回的）"
          transformedLabel="转换后（返回给客户端的）"
        />
      )}
      <HeaderTable
        headers={view === "raw" ? log.upstreamRespHeaders : log.respHeaders}
      />
    </div>
  );
}

function RequestBodyTab({
  log,
  view,
  onView,
}: {
  log: RequestLog;
  view: BodyView;
  onView: (v: BodyView) => void;
}) {
  const reqContentType = log.reqHeaders.find(
    (h) => h.key.toLowerCase() === "content-type",
  )?.value;
  const upstreamContentType = log.upstreamHeaders.find(
    (h) => h.key.toLowerCase() === "content-type",
  )?.value;
  const showToggle =
    log.reqBodyB64 !== log.upstreamBodyB64 || log.reqBodyLen !== log.upstreamBodyLen;
  return (
    <div className="space-y-2">
      {showToggle && (
        <BodyViewToggle
          value={view}
          onChange={onView}
          rawLabel="原始（客户端发的）"
          transformedLabel="转换后（发往上游的）"
        />
      )}
      <BodyView
        b64={view === "raw" ? log.reqBodyB64 : log.upstreamBodyB64}
        binary={log.reqBodyBinary}
        len={view === "raw" ? log.reqBodyLen : log.upstreamBodyLen}
        contentType={view === "raw" ? reqContentType : upstreamContentType}
      />
    </div>
  );
}

function ResponseBodyTab({
  log,
  view,
  onView,
}: {
  log: RequestLog;
  view: BodyView;
  onView: (v: BodyView) => void;
}) {
  const showToggle =
    log.upstreamRespBodyB64 !== log.respBodyB64 ||
    log.upstreamRespBodyLen !== log.respBodyLen;
  return (
    <div className="space-y-2">
      {showToggle && (
        <BodyViewToggle
          value={view}
          onChange={onView}
          rawLabel="原始（上游返回的）"
          transformedLabel="转换后（返回给客户端的）"
        />
      )}
      <BodyView
        b64={view === "raw" ? log.upstreamRespBodyB64 : log.respBodyB64}
        binary={log.respBodyBinary}
        len={view === "raw" ? log.upstreamRespBodyLen : log.respBodyLen}
        contentType={log.respContentType}
      />
    </div>
  );
}

function BodyView({
  b64,
  binary,
  len,
  contentType,
}: {
  b64: string;
  binary: boolean;
  len: number;
  contentType?: string;
}) {
  if (binary) {
    return (
      <p className="text-sm text-muted-foreground">
        二进制内容，未记录 body（{formatSize(len)}）。
      </p>
    );
  }
  if (len === 0) {
    return <p className="text-sm text-muted-foreground">空</p>;
  }
  const text = base64ToText(b64);
  const fmt = detectFormat(text, contentType);

  const headBar = (
    <div className="flex items-center gap-2 text-xs text-muted-foreground">
      <span>{formatSize(len)}</span>
      <Badge tone="sky">{fmt.toUpperCase()}</Badge>
      {contentType && <span className="font-mono">{contentType}</span>}
      <Button
        variant="ghost"
        size="sm"
        className="ml-auto h-7 px-2 text-xs"
        onClick={() => copyBodyToClipboard(b64, binary)}
        title="复制全部内容"
      >
        <Copy className="h-3 w-3" />
        复制
      </Button>
    </div>
  );

  if (fmt === "form") {
    const pairs: [string, string][] = [];
    try {
      new URLSearchParams(text).forEach((v, k) => pairs.push([k, v]));
    } catch {
      // fall through to text
    }
    if (pairs.length > 0) {
      return (
        <div className="space-y-2">
          {headBar}
          <div className="rounded-md border overflow-hidden">
            <table className="w-full text-sm font-mono">
              <tbody>
                {pairs.map(([k, v], idx) => (
                  <tr
                    key={`${k}-${idx}`}
                    className="border-b last:border-b-0 hover:bg-muted/40"
                  >
                    <td className="px-3 py-1.5 text-muted-foreground align-top w-1/3 break-all">
                      {k}
                    </td>
                    <td className="px-3 py-1.5 break-all whitespace-pre-wrap">
                      {v}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      );
    }
  }

  if (fmt === "sse") {
    const events = parseSSE(text);
    if (events.length > 0) {
      return (
        <div className="space-y-2">
          {headBar}
          <div className="space-y-2 max-h-[60vh] overflow-y-auto">
            {events.map((ev, idx) => {
              const pretty = tryFormatJson(ev.data);
              return (
                <div key={idx} className="rounded-md border bg-muted/30 p-3 space-y-1">
                  <div className="flex items-center gap-2 text-xs text-muted-foreground">
                    <span>#{idx + 1}</span>
                    {ev.event && <Badge tone="violet">event: {ev.event}</Badge>}
                    {ev.id && <span className="font-mono">id: {ev.id}</span>}
                  </div>
                  <pre className="text-xs font-mono whitespace-pre-wrap break-all">
                    {pretty}
                  </pre>
                </div>
              );
            })}
          </div>
        </div>
      );
    }
  }

  let display = text;
  if (fmt === "json") display = tryFormatJson(text);
  else if (fmt === "xml") display = indentXml(text);

  return (
    <div className="space-y-2">
      {headBar}
      <pre className="rounded-md border bg-muted/30 px-3 py-2 text-xs font-mono whitespace-pre-wrap break-all max-h-[60vh] overflow-y-auto">
        {display}
      </pre>
    </div>
  );
}

export function RequestLogPanel({ endpoint, onClose }: Props) {
  const [logs, setLogs] = React.useState<RequestLog[]>([]);
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [tab, setTab] = React.useState<Tab>("reqHeaders");
  const [reqHeadersView, setReqHeadersView] = React.useState<BodyView>("raw");
  const [respHeadersView, setRespHeadersView] = React.useState<BodyView>("raw");
  const [reqBodyView, setReqBodyView] = React.useState<BodyView>("raw");
  const [respBodyView, setRespBodyView] = React.useState<BodyView>("raw");
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
            共 {logs.length} 条
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
                  <RequestHeadersTab
                    log={selected}
                    view={reqHeadersView}
                    onView={setReqHeadersView}
                  />
                )}
                {tab === "reqBody" && (
                  <RequestBodyTab
                    log={selected}
                    view={reqBodyView}
                    onView={setReqBodyView}
                  />
                )}
                {tab === "respHeaders" && (
                  <ResponseHeadersTab
                    log={selected}
                    view={respHeadersView}
                    onView={setRespHeadersView}
                  />
                )}
                {tab === "respBody" && (
                  <ResponseBodyTab
                    log={selected}
                    view={respBodyView}
                    onView={setRespBodyView}
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
