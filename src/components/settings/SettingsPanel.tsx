import * as React from "react";
import { Save } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api } from "@/lib/api";
import { globalConfigSchema } from "@/lib/schemas";
import type { GlobalConfig } from "@/types";
import { GlobalProxySettings } from "./GlobalProxySettings";

interface Props {
  open: boolean;
  global: GlobalConfig;
  onClose: () => void;
  onSaved: (g: GlobalConfig) => void;
}

const FORM_ID = "settings-form";

export function SettingsPanel({ open, global, onClose, onSaved }: Props) {
  const {
    register,
    control,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<GlobalConfig>({
    resolver: zodResolver(globalConfigSchema),
    defaultValues: global,
  });

  React.useEffect(() => {
    if (open) reset(global);
  }, [open, global, reset]);

  const submit = handleSubmit(async (values) => {
    try {
      const status = await api.applyGlobalConfig(values);
      toast.success(
        status.running
          ? `服务运行中：${status.listenAddress}:${status.listenPort}`
          : "已保存",
      );
      onSaved(values);
      onClose();
    } catch (e: unknown) {
      toast.error(
        typeof e === "string" ? e : (e as Error)?.message ?? "保存失败",
      );
    }
  });

  return (
    <FullScreenPanel
      open={open}
      onClose={onClose}
      title="设置"
      footer={
        <>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button type="submit" form={FORM_ID}>
            <Save className="h-4 w-4" />
            应用
          </Button>
        </>
      }
    >
      <form id={FORM_ID} onSubmit={submit} className="space-y-6">
        <section className="rounded-xl border p-4 space-y-4">
          <Label className="text-base font-semibold">代理监听</Label>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="listenAddress">监听地址</Label>
              <Input
                id="listenAddress"
                className="font-mono"
                placeholder="0.0.0.0"
                {...register("listenAddress")}
              />
              {errors.listenAddress && (
                <p className="text-sm text-destructive">
                  {errors.listenAddress.message}
                </p>
              )}
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="listenPort">监听端口</Label>
              <Input
                id="listenPort"
                type="number"
                min={1}
                max={65535}
                {...register("listenPort", { valueAsNumber: true })}
              />
              {errors.listenPort && (
                <p className="text-sm text-destructive">
                  {errors.listenPort.message}
                </p>
              )}
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="requestTimeoutSecs">请求超时（秒）</Label>
              <Input
                id="requestTimeoutSecs"
                type="number"
                min={1}
                max={600}
                {...register("requestTimeoutSecs", { valueAsNumber: true })}
              />
              {errors.requestTimeoutSecs && (
                <p className="text-sm text-destructive">
                  {errors.requestTimeoutSecs.message}
                </p>
              )}
            </div>
          </div>
        </section>

        <GlobalProxySettings />

        <section className="rounded-xl border p-4 space-y-4">
          <Label className="text-base font-semibold">行为</Label>
          <div className="flex items-center justify-between rounded-md border bg-muted/30 p-3">
            <div>
              <Label className="text-sm font-medium">
                关闭窗口最小化到托盘
              </Label>
              <p className="text-xs text-muted-foreground">
                关闭后只隐藏窗口，代理服务继续在后台运行
              </p>
            </div>
            <Controller
              control={control}
              name="closeToTray"
              render={({ field }) => (
                <Switch
                  checked={field.value}
                  onCheckedChange={field.onChange}
                />
              )}
            />
          </div>
          <div className="flex items-center justify-between rounded-md border bg-muted/30 p-3">
            <div>
              <Label className="text-sm font-medium">启动时自动启动代理</Label>
              <p className="text-xs text-muted-foreground">
                打开应用后立即启动代理服务
              </p>
            </div>
            <Controller
              control={control}
              name="autoStartServer"
              render={({ field }) => (
                <Switch
                  checked={field.value}
                  onCheckedChange={field.onChange}
                />
              )}
            />
          </div>
        </section>

        <section className="rounded-xl border p-4 space-y-4">
          <Label className="text-base font-semibold">外观</Label>
          <div className="space-y-1.5">
            <Label>主题</Label>
            <Controller
              control={control}
              name="theme"
              render={({ field }) => (
                <Select value={field.value} onValueChange={field.onChange}>
                  <SelectTrigger className="max-w-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="light">亮色</SelectItem>
                    <SelectItem value="dark">深色</SelectItem>
                    <SelectItem value="system">跟随系统</SelectItem>
                  </SelectContent>
                </Select>
              )}
            />
          </div>
        </section>
      </form>
    </FullScreenPanel>
  );
}
