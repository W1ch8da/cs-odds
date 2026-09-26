import { cn } from "@/lib/utils";

import { PRIORITY_LABEL } from "../labels";
import type { Priority } from "../types";

const BARS: Record<Priority, number> = { low: 1, normal: 2, high: 3, urgent: 3 };
const TONE: Record<Priority, string> = {
  urgent: "text-destructive",
  high: "text-warning",
  normal: "text-muted-foreground",
  low: "text-muted-foreground",
};

/** Signal-strength bars plus the priority name. */
export function PriorityLabel({ priority, className }: { priority: Priority; className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-1.5 text-sm whitespace-nowrap", TONE[priority], className)}>
      <span aria-hidden className="inline-flex h-2.5 items-end gap-0.5">
        {[1, 2, 3].map((bar) => (
          <span
            key={bar}
            className={cn("w-[3px] rounded-[1px]", bar <= BARS[priority] ? "bg-current" : "bg-border")}
            style={{ height: `${bar * 3 + 2}px` }}
          />
        ))}
      </span>
      {PRIORITY_LABEL[priority]}
    </span>
  );
}
