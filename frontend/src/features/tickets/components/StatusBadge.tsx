import { cn } from "@/lib/utils";

import { CUSTOMER_STATUS_LABEL, STATUS_HINT, STATUS_LABEL, STATUS_TONE } from "../labels";
import type { TicketStatus } from "../types";

/** Status pill. `audience="customer"` uses the portal wording. */
export function StatusBadge({ status, audience = "team" }: { status: TicketStatus; audience?: "team" | "customer" }) {
  return (
    <span
      title={audience === "team" ? STATUS_HINT[status] : undefined}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium whitespace-nowrap",
        "before:size-1.5 before:rounded-full before:bg-current before:content-['']",
        STATUS_TONE[status],
      )}
    >
      {audience === "team" ? STATUS_LABEL[status] : CUSTOMER_STATUS_LABEL[status]}
    </span>
  );
}
