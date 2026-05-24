import { Plus, Trash2 } from "lucide-react";
import { Controller, useFieldArray, useFormContext } from "react-hook-form";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { EndpointForm } from "@/lib/schemas";

interface Props {
  name: "headerRules" | "queryRules";
  title: string;
  keyPlaceholder: string;
}

export function RulesField({ name, title, keyPlaceholder }: Props) {
  const { control, register } = useFormContext<EndpointForm>();
  const { fields, append, remove } = useFieldArray({ control, name });
  return (
    <section className="rounded-xl border p-4 space-y-3">
      <div className="flex items-center justify-between">
        <Label className="text-base font-semibold">{title}</Label>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => append({ action: "set", key: "", value: "" })}
        >
          <Plus className="h-4 w-4" />
          添加规则
        </Button>
      </div>
      {fields.length === 0 && (
        <p className="text-sm text-muted-foreground">暂无规则</p>
      )}
      <div className="space-y-2">
        {fields.map((field, idx) => (
          <div
            key={field.id}
            className="grid grid-cols-[7rem_1fr_1fr_2.5rem] gap-2"
          >
            <Controller
              control={control}
              name={`${name}.${idx}.action` as const}
              render={({ field: f }) => (
                <Select value={f.value} onValueChange={f.onChange}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="set">覆盖 / 新增</SelectItem>
                    <SelectItem value="add">追加</SelectItem>
                    <SelectItem value="remove">删除</SelectItem>
                  </SelectContent>
                </Select>
              )}
            />
            <Input
              placeholder={keyPlaceholder}
              {...register(`${name}.${idx}.key` as const)}
            />
            <Controller
              control={control}
              name={`${name}.${idx}.action` as const}
              render={({ field: a }) => (
                <Input
                  placeholder="Value"
                  disabled={a.value === "remove"}
                  {...register(`${name}.${idx}.value` as const)}
                />
              )}
            />
            <Button
              type="button"
              variant="ghost"
              size="icon"
              onClick={() => remove(idx)}
              className="hover:text-red-500"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        ))}
      </div>
    </section>
  );
}
