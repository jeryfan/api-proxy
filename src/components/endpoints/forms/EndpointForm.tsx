import { Controller, FormProvider, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { endpointFormSchema, type EndpointForm as TForm } from "@/lib/schemas";
import type { Endpoint } from "@/types";
import { RulesField } from "./RulesField";

interface Props {
  initial?: Endpoint;
  formId: string;
  onSubmit: (values: TForm) => Promise<void>;
}

function makeDefault(initial?: Endpoint): TForm {
  return {
    id: initial?.id ?? "",
    name: initial?.name ?? "",
    description: initial?.description ?? "",
    enabled: initial?.enabled ?? true,
    path: initial?.path ?? "/",
    upstreamUrl: initial?.upstreamUrl ?? "",
    stripPrefix: initial?.stripPrefix ?? true,
    headerRules: initial?.headerRules ?? [],
    queryRules: initial?.queryRules ?? [],
    bodyMerge: initial?.bodyMerge ?? "",
    apiFormat: initial?.apiFormat ?? "passthrough",
  };
}

export function EndpointForm({ initial, formId, onSubmit }: Props) {
  const methods = useForm<TForm>({
    resolver: zodResolver(endpointFormSchema),
    defaultValues: makeDefault(initial),
    mode: "onSubmit",
  });
  const {
    register,
    control,
    handleSubmit,
    formState: { errors },
  } = methods;

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
                placeholder="例如 Moonshot 转发"
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
          <div className="flex items-center justify-between rounded-md border bg-muted/30 p-3">
            <div>
              <Label className="text-sm font-medium">启用端点</Label>
              <p className="text-xs text-muted-foreground">
                关闭后此端点收到的请求会返回 404
              </p>
            </div>
            <Controller
              control={control}
              name="enabled"
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
          <Label className="text-base font-semibold">路由</Label>
          <div className="space-y-1.5">
            <Label htmlFor="path">端点路径</Label>
            <Input
              id="path"
              placeholder="/cc"
              className="font-mono"
              {...register("path")}
            />
            {errors.path && (
              <p className="text-sm text-destructive">{errors.path.message}</p>
            )}
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="upstreamUrl">上游地址</Label>
            <Input
              id="upstreamUrl"
              placeholder="https://api.moonshot.com/v1"
              className="font-mono"
              {...register("upstreamUrl")}
            />
            {errors.upstreamUrl && (
              <p className="text-sm text-destructive">
                {errors.upstreamUrl.message}
              </p>
            )}
          </div>
          <div className="flex items-center justify-between rounded-md border bg-muted/30 p-3">
            <div>
              <Label className="text-sm font-medium">剥离路径前缀</Label>
              <p className="text-xs text-muted-foreground">
                开启：/cc/x → 上游 /x；关闭：/cc/x → 上游 /cc/x
              </p>
            </div>
            <Controller
              control={control}
              name="stripPrefix"
              render={({ field }) => (
                <Switch
                  checked={field.value}
                  onCheckedChange={field.onChange}
                />
              )}
            />
          </div>
        </section>

        <section className="rounded-xl border p-4 space-y-3">
          <Label className="text-base font-semibold">格式转换</Label>
          <p className="text-sm text-muted-foreground">
            将客户端请求体与上游响应在不同 LLM 协议之间相互转换。流式响应（SSE）会用对应的状态机做流式转换。
          </p>
          <Controller
            control={control}
            name="apiFormat"
            render={({ field }) => (
              <Select value={field.value} onValueChange={field.onChange}>
                <SelectTrigger className="max-w-md">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="passthrough">不转换（原样转发）</SelectItem>
                  <SelectItem value="anthropicToOpenaiChat">
                    Anthropic Messages → OpenAI Chat Completions
                  </SelectItem>
                  <SelectItem value="anthropicToOpenaiResponses">
                    Anthropic Messages → OpenAI Responses
                  </SelectItem>
                  <SelectItem value="anthropicToGemini">
                    Anthropic Messages → Google Gemini
                  </SelectItem>
                  <SelectItem value="responsesToOpenaiChat">
                    OpenAI Responses → OpenAI Chat Completions
                  </SelectItem>
                </SelectContent>
              </Select>
            )}
          />
        </section>

        <RulesField
          name="headerRules"
          title="请求头规则"
          keyPlaceholder="Header 名（如 Authorization）"
        />
        <RulesField
          name="queryRules"
          title="查询参数规则"
          keyPlaceholder="参数名（如 model）"
        />

        <section className="rounded-xl border p-4 space-y-3">
          <Label htmlFor="bodyMerge" className="text-base font-semibold">
            请求体 JSON 合并（可选）
          </Label>
          <p className="text-sm text-muted-foreground">
            仅当请求体为 JSON 时生效。这里填的对象会深合并到原 body，键冲突时此处覆盖。
          </p>
          <Textarea
            id="bodyMerge"
            placeholder='{"model":"kimi-k2"}'
            className="font-mono"
            {...register("bodyMerge")}
          />
          {errors.bodyMerge && (
            <p className="text-sm text-destructive">
              {errors.bodyMerge.message}
            </p>
          )}
        </section>
      </form>
    </FormProvider>
  );
}
