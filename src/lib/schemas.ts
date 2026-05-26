import { z } from "zod";

export const ruleActionSchema = z.enum(["set", "add", "remove"]);

export const ruleSchema = z.object({
  action: ruleActionSchema,
  key: z.string().min(1, "请填写 Key"),
  value: z.string(),
});

export const apiFormatSchema = z.enum([
  "passthrough",
  "anthropicToOpenaiChat",
  "anthropicToOpenaiResponses",
  "anthropicToGemini",
  "responsesToOpenaiChat",
]);

export const endpointFormSchema = z.object({
  id: z.string(),
  name: z.string().min(1, "名称必填").max(60, "名称不超过 60 字符"),
  description: z.string().max(200, "描述不超过 200 字符"),
  enabled: z.boolean(),
  path: z
    .string()
    .min(2, "路径不能为空")
    .regex(/^\/[A-Za-z0-9._\-/]+$/, "路径只能包含字母、数字、_、-、. 和 /")
    .refine((s) => s !== "/", "路径不能为根 /"),
  upstreamUrl: z
    .string()
    .url("请输入合法 URL")
    .refine(
      (s) => s.startsWith("http://") || s.startsWith("https://"),
      "仅支持 http/https",
    ),
  stripPrefix: z.boolean(),
  fixedUpstream: z.boolean(),
  headerRules: z.array(ruleSchema),
  queryRules: z.array(ruleSchema),
  bodyMerge: z
    .string()
    .refine((s) => {
      if (!s.trim()) return true;
      try {
        const v = JSON.parse(s);
        return typeof v === "object" && v !== null && !Array.isArray(v);
      } catch {
        return false;
      }
    }, "必须为合法的 JSON 对象"),
  apiFormat: apiFormatSchema,
});

export type EndpointForm = z.infer<typeof endpointFormSchema>;

export const globalConfigSchema = z.object({
  listenAddress: z.string().min(1, "请填写监听地址"),
  listenPort: z.number().int().min(1).max(65535),
  requestTimeoutSecs: z.number().int().min(1).max(600),
  theme: z.enum(["light", "dark", "system"]),
  closeToTray: z.boolean(),
  autoStartServer: z.boolean(),
  proxyUrl: z.string(),
  logBufferCapacity: z.number().int().min(1).max(2000),
});
