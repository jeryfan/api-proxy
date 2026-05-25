import * as React from "react";
import { Eye, EyeOff, Loader2, Search, TestTube2, X } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api } from "@/lib/api";
import type { DetectedProxy } from "@/types";

function extractAuth(url: string): {
  baseUrl: string;
  username: string;
  password: string;
} {
  if (!url.trim()) return { baseUrl: "", username: "", password: "" };
  try {
    const parsed = new URL(url);
    const username = decodeURIComponent(parsed.username || "");
    const password = decodeURIComponent(parsed.password || "");
    parsed.username = "";
    parsed.password = "";
    return { baseUrl: parsed.toString(), username, password };
  } catch {
    return { baseUrl: url, username: "", password: "" };
  }
}

function mergeAuth(
  baseUrl: string,
  username: string,
  password: string,
): string {
  if (!baseUrl.trim()) return "";
  if (!username.trim()) return baseUrl;
  try {
    const parsed = new URL(baseUrl);
    parsed.username = username.trim();
    if (password) parsed.password = password;
    return parsed.toString();
  } catch {
    return baseUrl;
  }
}

export function GlobalProxySettings() {
  const [saved, setSaved] = React.useState<string | null>(null);
  const [url, setUrl] = React.useState("");
  const [username, setUsername] = React.useState("");
  const [password, setPassword] = React.useState("");
  const [showPassword, setShowPassword] = React.useState(false);
  const [dirty, setDirty] = React.useState(false);
  const [detected, setDetected] = React.useState<DetectedProxy[]>([]);
  const [saving, setSaving] = React.useState(false);
  const [scanning, setScanning] = React.useState(false);
  const [testing, setTesting] = React.useState(false);

  const fullUrl = React.useMemo(
    () => mergeAuth(url, username, password),
    [url, username, password],
  );

  React.useEffect(() => {
    api.getGlobalProxyUrl().then((u) => {
      setSaved(u);
      const { baseUrl, username: usr, password: pwd } = extractAuth(u);
      setUrl(baseUrl);
      setUsername(usr);
      setPassword(pwd);
      setDirty(false);
    });
  }, []);

  const handleSave = async () => {
    setSaving(true);
    try {
      await api.setGlobalProxyUrl(fullUrl);
      setSaved(fullUrl);
      setDirty(false);
      toast.success(fullUrl ? "出站代理已应用" : "已切换为直连");
    } catch (e: unknown) {
      toast.error(
        typeof e === "string" ? e : (e as Error)?.message ?? "保存失败",
      );
    } finally {
      setSaving(false);
    }
  };

  const handleTest = async () => {
    if (!fullUrl) return;
    setTesting(true);
    try {
      const res = await api.testProxyUrl(fullUrl);
      if (res.success) {
        toast.success(`代理可用 · ${res.latencyMs}ms`);
      } else {
        toast.error(`代理不可用：${res.error ?? "未知错误"}`);
      }
    } catch (e: unknown) {
      toast.error(
        typeof e === "string" ? e : (e as Error)?.message ?? "测试失败",
      );
    } finally {
      setTesting(false);
    }
  };

  const handleScan = async () => {
    setScanning(true);
    try {
      const result = await api.scanLocalProxies();
      setDetected(result);
      if (result.length === 0) toast.info("未发现本机代理端口");
    } finally {
      setScanning(false);
    }
  };

  const handleSelect = (proxyUrl: string) => {
    const { baseUrl, username: u, password: p } = extractAuth(proxyUrl);
    setUrl(baseUrl);
    setUsername(u);
    setPassword(p);
    setDirty(true);
    setDetected([]);
  };

  const handleClear = () => {
    setUrl("");
    setUsername("");
    setPassword("");
    setDirty(true);
  };

  const isUnchangedFromSaved = saved === fullUrl;

  return (
    <section className="rounded-xl border p-4 space-y-3">
      <div className="space-y-1">
        <Label className="text-base font-semibold">出站代理</Label>
        <p className="text-sm text-muted-foreground">
          让本工具发往上游 API 的请求经由 HTTP 或 SOCKS 代理。留空表示直连。
        </p>
      </div>

      <div className="flex gap-2">
        <Input
          placeholder="http://127.0.0.1:7890 / socks5://127.0.0.1:1080"
          value={url}
          onChange={(e) => {
            setUrl(e.target.value);
            setDirty(true);
          }}
          className="font-mono text-sm flex-1"
        />
        <Button
          variant="outline"
          size="icon"
          disabled={scanning}
          onClick={handleScan}
          title="扫描本机代理"
        >
          {scanning ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Search className="h-4 w-4" />
          )}
        </Button>
        <Button
          variant="outline"
          size="icon"
          disabled={!fullUrl || testing}
          onClick={handleTest}
          title="测试连通"
        >
          {testing ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <TestTube2 className="h-4 w-4" />
          )}
        </Button>
        <Button
          variant="outline"
          size="icon"
          disabled={!url && !username && !password}
          onClick={handleClear}
          title="清空"
        >
          <X className="h-4 w-4" />
        </Button>
        <Button
          onClick={handleSave}
          disabled={!dirty || saving || isUnchangedFromSaved}
          size="sm"
        >
          {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
          保存
        </Button>
      </div>

      <div className="flex gap-2">
        <Input
          placeholder="用户名（可选）"
          value={username}
          onChange={(e) => {
            setUsername(e.target.value);
            setDirty(true);
          }}
          className="font-mono text-sm flex-1"
        />
        <div className="relative flex-1">
          <Input
            type={showPassword ? "text" : "password"}
            placeholder="密码（可选）"
            value={password}
            onChange={(e) => {
              setPassword(e.target.value);
              setDirty(true);
            }}
            className="font-mono text-sm pr-10"
          />
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="absolute right-0 top-0 h-full px-3 hover:bg-transparent"
            onClick={() => setShowPassword(!showPassword)}
            tabIndex={-1}
          >
            {showPassword ? (
              <EyeOff className="h-4 w-4 text-muted-foreground" />
            ) : (
              <Eye className="h-4 w-4 text-muted-foreground" />
            )}
          </Button>
        </div>
      </div>

      {detected.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {detected.map((p) => (
            <Button
              key={p.url}
              variant="secondary"
              size="sm"
              onClick={() => handleSelect(p.url)}
              className="font-mono text-xs"
            >
              {p.url}
            </Button>
          ))}
        </div>
      )}
    </section>
  );
}
