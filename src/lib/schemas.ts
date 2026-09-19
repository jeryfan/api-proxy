import { z } from "zod";

const ruleActionSchema = z.enum(["set", "add", "remove"]);

const ruleSchema = z.object({
  action: ruleActionSchema,
  key: z.string().min(1, "请填写 Key"),
  value: z.string(),
});

const bodyMatchSchema = z.object({
  mode: z.enum(["contains", "regex"]),
  pattern: z.string(),
});

const healthConfigSchema = z.object({
  enabled: z.boolean(),
  failureThreshold: z.number().int().min(1, "失败阈值至少为 1"),
  countConnectError: z.boolean(),
  statusCodes: z.array(z.number().int().min(100).max(599)),
  bodyMatch: bodyMatchSchema.optional(),
});

const upstreamSchema = z.object({
  id: z.string(),
  name: z.string().max(60, "名称不超过 60 字符"),
  url: z
    .url("请输入合法 URL")
    .refine(
      (s) => s.startsWith("http://") || s.startsWith("https://"),
      "仅支持 http/https",
    ),
  enabled: z.boolean(),
  weight: z
    .number()
    .int("权重必须为整数")
    .min(1, "权重 1-100")
    .max(100, "权重 1-100"),
  headerRules: z.array(ruleSchema),
  health: healthConfigSchema,
});

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
  upstreams: z
    .array(upstreamSchema)
    .min(1, "至少需要一个上游")
    .refine((list) => list.some((u) => u.enabled), "至少启用一个上游"),
  stripPrefix: z.boolean(),
  fixedUpstream: z.boolean(),
  headerRules: z.array(ruleSchema),
  queryRules: z.array(ruleSchema),
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
