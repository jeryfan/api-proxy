import { Controller, FormProvider, useFieldArray, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { AlertTriangle, Plus, RotateCcw, ShieldAlert, Trash2 } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { endpointFormSchema, type EndpointForm as TForm } from "@/lib/schemas";
import type { Endpoint, UpstreamHealthState } from "@/types";
import { RulesField } from "./RulesField";

interface Props {
  initial?: Endpoint;
  formId: string;
  healthStates?: Record<string, UpstreamHealthState>;
  onResetHealth?: (upstreamId: string) => Promise<void>;
  onSubmit: (values: TForm) => Promise<void>;
}

function makeDefault(initial?: Endpoint): TForm {
  return {
    id: initial?.id ?? "",
    name: initial?.name ?? "",
    description: initial?.description ?? "",
    enabled: initial?.enabled ?? true,
    path: initial?.path ?? "/",
    upstreams:
      initial?.upstreams.map((u) => ({
        id: u.id,
        name: u.name,
        url: u.url,
        enabled: u.enabled,
        weight: u.weight,
        headerRules: u.headerRules ?? [],
        health: {
          enabled: u.health?.enabled ?? false,
          failureThreshold: u.health?.failureThreshold ?? 3,
          countConnectError: u.health?.countConnectError ?? true,
          statusCodes: u.health?.statusCodes ?? [429, 500, 502, 503, 504],
          bodyMatch: {
            mode: u.health?.bodyMatch?.mode ?? "contains",
            pattern: u.health?.bodyMatch?.pattern ?? "",
          },
        },
      })) ?? [],
    stripPrefix: initial?.stripPrefix ?? true,
    fixedUpstream: initial?.fixedUpstream ?? false,
    headerRules: initial?.headerRules ?? [],
    queryRules: initial?.queryRules ?? [],
  };
}

function newUpstream(): TForm["upstreams"][number] {
  return {
    id: crypto.randomUUID(),
    name: "",
    url: "",
    enabled: true,
    weight: 1,
    headerRules: [],
    health: {
      enabled: false,
      failureThreshold: 3,
      countConnectError: true,
      statusCodes: [429, 500, 502, 503, 504],
      bodyMatch: {
        mode: "contains",
        pattern: "",
      },
    },
  };
}

export function EndpointForm({ initial, formId, healthStates, onResetHealth, onSubmit }: Props) {
  const methods = useForm<TForm>({
    resolver: zodResolver(endpointFormSchema),
    defaultValues: makeDefault(initial),
    mode: "onSubmit",
  });
  const {
    register,
    control,
    handleSubmit,
    watch,
    formState: { errors },
  } = methods;

  const { fields, append, remove } = useFieldArray({
    control,
    name: "upstreams",
  });

  return (
    <FormProvider {...methods}>
      <form id={formId} onSubmit={handleSubmit(onSubmit)} className="space-y-6">
        <section className="rounded-xl border p-4 space-y-4">
          <Label className="text-base font-semibold">基本信息</Label>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="name">名称</Label>
              <Input
                id="name"
                placeholder="端点名称"
                {...register("name")}
              />
              {errors.name && (
                <p className="text-sm text-destructive">
                  {errors.name.message}
                </p>
              )}
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="description">描述（可选）</Label>
              <Input
                id="description"
                placeholder="可选说明"
                {...register("description")}
              />
            </div>
          </div>
        </section>

        <section className="rounded-xl border p-4 space-y-4">
          <Label className="text-base font-semibold">路由</Label>
          <div className="space-y-1.5">
            <Label htmlFor="path">端点路径</Label>
            <Input
              id="path"
              placeholder="/api"
              className="font-mono"
              {...register("path")}
            />
            {errors.path && (
              <p className="text-sm text-destructive">{errors.path.message}</p>
            )}
          </div>
          <Controller
            control={control}
            name="fixedUpstream"
            render={({ field }) => (
              <div className="flex items-center justify-between rounded-md border bg-muted/30 p-3">
                <Label className="text-sm font-medium">固定上游地址</Label>
                <Switch
                  checked={field.value}
                  onCheckedChange={field.onChange}
                />
              </div>
            )}
          />
          <Controller
            control={control}
            name="stripPrefix"
            render={({ field: stripField }) => (
              <Controller
                control={control}
                name="fixedUpstream"
                render={({ field: fixedField }) => (
                  <div className="flex items-center justify-between rounded-md border bg-muted/30 p-3">
                    <Label className="text-sm font-medium">剥离路径前缀</Label>
                    <Switch
                      checked={stripField.value}
                      onCheckedChange={stripField.onChange}
                      disabled={fixedField.value}
                    />
                  </div>
                )}
              />
            )}
          />
        </section>

        <section className="rounded-xl border p-4 space-y-4">
          <Label className="text-base font-semibold">上游与负载均衡</Label>
          {errors.upstreams?.root?.message && (
            <p className="text-sm text-destructive">
              {errors.upstreams.root.message}
            </p>
          )}
          <div className="space-y-4">
            {fields.map((field, index) => {
              const hsKey = initial?.id ? `${initial.id}:${field.id}` : "";
              const currentHealth = hsKey && healthStates ? healthStates[hsKey] : undefined;
              const isTripped = currentHealth?.isTripped;
              const isHealthEnabled = watch(`upstreams.${index}.health.enabled`);

              return (
                <div
                  key={field.id}
                  className="space-y-3 rounded-md border bg-muted/20 p-4 transition-all"
                >
                  {isTripped && (
                    <div className="flex items-center justify-between rounded-md border border-rose-500/30 bg-rose-500/10 p-2.5 text-xs text-rose-600 dark:text-rose-400">
                      <div className="flex items-center gap-1.5">
                        <AlertTriangle className="h-4 w-4 flex-shrink-0" />
                        <span>
                          <strong>该上游已熔断失效</strong>：连续失败 {currentHealth?.consecutiveFailures} 次
                          {currentHealth?.lastFailureReason ? ` (${currentHealth.lastFailureReason})` : ""}，已停止分发流量。
                        </span>
                      </div>
                      {onResetHealth && (
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          className="h-6 px-2 text-xs border-rose-500/30 hover:bg-rose-500/20"
                          onClick={() => onResetHealth(field.id)}
                        >
                          <RotateCcw className="h-3 w-3 mr-1" />
                          手动重置并开启
                        </Button>
                      )}
                    </div>
                  )}

                  <div className="flex items-center gap-3">
                    <div className="flex-1 space-y-1.5">
                      <Input
                        placeholder="名称（可选，用于日志与标识）"
                        {...register(`upstreams.${index}.name`)}
                      />
                    </div>
                    <div className="w-28 space-y-1.5">
                      <Input
                        type="number"
                        min={1}
                        max={100}
                        title="权重 1-100"
                        placeholder="权重 1-100"
                        {...register(`upstreams.${index}.weight`, {
                          valueAsNumber: true,
                        })}
                      />
                      {errors.upstreams?.[index]?.weight && (
                        <p className="text-xs text-destructive">
                          {errors.upstreams[index]?.weight?.message}
                        </p>
                      )}
                    </div>
                    <Controller
                      control={control}
                      name={`upstreams.${index}.enabled`}
                      render={({ field: f }) => (
                        <div className="flex items-center gap-1.5" title={f.value ? "上游启用中" : "上游已停用"}>
                          <Switch
                            checked={f.value}
                            onCheckedChange={f.onChange}
                          />
                        </div>
                      )}
                    />
                  </div>

                  <div className="flex items-start gap-3">
                    <div className="flex-1 space-y-1.5">
                      <Input
                        placeholder="https://example.com"
                        className="font-mono"
                        {...register(`upstreams.${index}.url`)}
                      />
                      {errors.upstreams?.[index]?.url && (
                        <p className="text-sm text-destructive">
                          {errors.upstreams[index]?.url?.message}
                        </p>
                      )}
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 p-1 flex-shrink-0 hover:text-red-500"
                      onClick={() => remove(index)}
                      disabled={fields.length <= 1}
                      title="删除上游"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>

                  <RulesField
                    name={`upstreams.${index}.headerRules`}
                    title="本上游请求头规则（覆盖端点级同名规则）"
                    keyPlaceholder="Header 名称"
                    compact
                  />

                  {/* 熔断与故障判定规则配置 */}
                  <div className="rounded-lg border bg-background/70 p-3 space-y-3 text-xs">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <ShieldAlert className="h-4 w-4 text-amber-500 flex-shrink-0" />
                        <Label className="text-xs font-semibold">熔断与失败规则配置</Label>
                      </div>
                      <Controller
                        control={control}
                        name={`upstreams.${index}.health.enabled`}
                        render={({ field: f }) => (
                          <div className="flex items-center gap-2">
                            <span className="text-[11px] text-muted-foreground">
                              {f.value ? "已开启" : "未开启"}
                            </span>
                            <Switch
                              checked={f.value}
                              onCheckedChange={f.onChange}
                            />
                          </div>
                        )}
                      />
                    </div>

                    {isHealthEnabled && (
                      <div className="space-y-3 pt-2.5 border-t border-dashed">
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                          <div className="space-y-1">
                            <Label className="text-xs font-medium">连续失败阈值 (次)</Label>
                            <Input
                              type="number"
                              min={1}
                              max={50}
                              className="h-8 text-xs font-mono"
                              placeholder="3"
                              {...register(`upstreams.${index}.health.failureThreshold`, {
                                valueAsNumber: true,
                              })}
                            />
                          </div>

                          <div className="space-y-1">
                            <Label className="text-xs font-medium">网络异常处理</Label>
                            <div className="flex items-center justify-between h-8 px-2.5 rounded-md border bg-background/50">
                              <span className="text-xs text-foreground/80">计入连接/超时错误</span>
                              <Controller
                                control={control}
                                name={`upstreams.${index}.health.countConnectError`}
                                render={({ field: f }) => (
                                  <Switch
                                    checked={f.value}
                                    onCheckedChange={f.onChange}
                                  />
                                )}
                              />
                            </div>
                          </div>
                        </div>

                        <div className="space-y-1">
                          <Label className="text-xs font-medium">触发失败的 HTTP 状态码</Label>
                          <Controller
                            control={control}
                            name={`upstreams.${index}.health.statusCodes`}
                            render={({ field: f }) => (
                              <Input
                                className="h-8 text-xs font-mono"
                                placeholder="429, 500, 502, 503, 504"
                                value={(f.value || []).join(", ")}
                                onChange={(e) => {
                                  const nums = e.target.value
                                    .split(/[\s,]+/)
                                    .map((s) => parseInt(s.trim(), 10))
                                    .filter((n) => !isNaN(n) && n >= 100 && n <= 599);
                                  f.onChange(nums);
                                }}
                              />
                            )}
                          />
                        </div>

                        <div className="space-y-1.5 pt-1">
                          <Label className="text-xs font-medium">响应内容特征匹配（失败判定）</Label>
                          <div className="flex items-center gap-2">
                            <div className="w-32 flex-shrink-0">
                              <Controller
                                control={control}
                                name={`upstreams.${index}.health.bodyMatch.mode`}
                                render={({ field: f }) => (
                                  <Select
                                    value={f.value || "contains"}
                                    onValueChange={f.onChange}
                                  >
                                    <SelectTrigger className="h-8 text-xs">
                                      <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                      <SelectItem value="contains">包含文本</SelectItem>
                                      <SelectItem value="regex">正则表达式</SelectItem>
                                    </SelectContent>
                                  </Select>
                                )}
                              />
                            </div>
                            <div className="flex-1">
                              <Input
                                className="h-8 text-xs font-mono"
                                placeholder="匹配文本或正则表达式"
                                {...register(`upstreams.${index}.health.bodyMatch.pattern`)}
                              />
                            </div>
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => append(newUpstream())}
          >
            <Plus className="h-4 w-4 mr-1" />
            添加上游
          </Button>
        </section>

        <RulesField
          name="headerRules"
          title="请求头规则（端点级，对所有上游生效）"
          keyPlaceholder="Header 名称"
        />
        <RulesField
          name="queryRules"
          title="查询参数规则"
          keyPlaceholder="参数名"
        />
      </form>
    </FormProvider>
  );
}
