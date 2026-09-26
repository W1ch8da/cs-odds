import Link from "next/link";

import { cn } from "@/lib/utils";

export function Brand({ href = "/", subtitle, className }: { href?: string; subtitle?: string; className?: string }) {
  return (
    <Link href={href} className={cn("flex items-center gap-2.5 rounded-md", className)}>
      <span className="grid size-7 place-items-center rounded-lg bg-primary text-xs font-semibold text-primary-foreground">
        CS
      </span>
      <span className="leading-tight">
        <span className="block font-semibold">CS-ODDS</span>
        {subtitle && <span className="block text-xs text-faint">{subtitle}</span>}
      </span>
    </Link>
  );
}
