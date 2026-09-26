"use client";

import { CheckCircle2Icon, InboxIcon, LayersIcon, UserIcon } from "lucide-react";
import Link from "next/link";
import { usePathname, useSearchParams } from "next/navigation";

import { cn } from "@/lib/utils";

import { QUEUE_VIEWS, type QueueView, isQueueView } from "../labels";
import { useTicketCountsQuery } from "../ticketsApi";

const ICONS: Record<QueueView, typeof InboxIcon> = { mine: UserIcon, unassigned: InboxIcon, open: LayersIcon, solved: CheckCircle2Icon };
const ORDER: QueueView[] = ["mine", "unassigned", "open", "solved"];

/** Sidebar queue views with live counts. Counts refresh every minute and after any change. */
export function QueueViewsNav({ variant = "sidebar" }: { variant?: "sidebar" | "tabs" }) {
  const pathname = usePathname();
  const params = useSearchParams();
  const { data: counts } = useTicketCountsQuery(undefined, { pollingInterval: 60_000 });
  const viewParam = params.get("view");
  const current: QueueView | null = pathname === "/agent/tickets" ? (isQueueView(viewParam) ? viewParam : "mine") : null;

  return (
    <nav aria-label="Ticket views" className={cn(variant === "sidebar" ? "flex flex-col gap-0.5" : "flex gap-1 overflow-x-auto")}>
      {ORDER.map((view) => {
        const Icon = ICONS[view];
        const active = current === view;
        const count = counts?.[view];
        return (
          <Link
            key={view}
            href={`/agent/tickets?view=${view}`}
            aria-current={active ? "page" : undefined}
            className={cn(
              "flex items-center gap-2.5 rounded-lg text-sm whitespace-nowrap transition-colors",
              variant === "sidebar" ? "px-2.5 py-1.5" : "px-3 py-1.5 font-medium",
              active
                ? "bg-accent font-medium text-accent-foreground"
                : "text-muted-foreground hover:bg-muted hover:text-foreground",
            )}
          >
            {variant === "sidebar" && <Icon className="size-4 shrink-0" />}
            {QUEUE_VIEWS[view].label}
            {count !== undefined && (
              <span className={cn("ml-auto text-xs tabular-nums", active ? "text-accent-foreground" : "text-faint")}>{count}</span>
            )}
          </Link>
        );
      })}
    </nav>
  );
}
