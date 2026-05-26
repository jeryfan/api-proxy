import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold",
  {
    variants: {
      tone: {
        violet:
          "bg-violet-100 text-violet-700 dark:bg-violet-900/40 dark:text-violet-300",
        slate:
          "bg-slate-200 text-slate-700 dark:bg-slate-700/60 dark:text-slate-200",
        emerald:
          "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300",
        red:
          "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300",
        sky:
          "bg-sky-100 text-sky-700 dark:bg-sky-900/40 dark:text-sky-300",
        amber:
          "bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300",
      },
    },
    defaultVariants: { tone: "slate" },
  },
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof badgeVariants> {}

export function Badge({ className, tone, ...props }: BadgeProps) {
  return <span className={cn(badgeVariants({ tone }), className)} {...props} />;
}
