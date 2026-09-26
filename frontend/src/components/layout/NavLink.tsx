"use client";

import type { LucideIcon } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";

import { cn } from "@/lib/utils";

type Props = {
  href: string;
  icon?: LucideIcon;
  children: React.ReactNode;
  variant?: "sidebar" | "tab";
  /** Match only this exact path, not its sub-pages. */
  exact?: boolean;
  /** Extra path prefix that also counts as this link, e.g. detail pages. */
  alsoActiveUnder?: string;
};

export function NavLink({ href, icon: Icon, children, variant = "sidebar", exact = false, alsoActiveUnder }: Props) {
  const pathname = usePathname();
  const active =
    pathname === href ||
    (!exact && pathname.startsWith(`${href}/`)) ||
    (alsoActiveUnder !== undefined && pathname.startsWith(alsoActiveUnder));
  return (
    <Link
      href={href}
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex items-center gap-2.5 rounded-lg text-sm transition-colors [&_svg]:size-4 [&_svg]:shrink-0",
        variant === "sidebar" && "px-2.5 py-1.5 text-muted-foreground hover:bg-muted hover:text-foreground",
        variant === "sidebar" && active && "bg-accent font-medium text-accent-foreground hover:bg-accent hover:text-accent-foreground",
        variant === "tab" && "px-3 py-1.5 font-medium text-muted-foreground hover:bg-muted hover:text-foreground",
        variant === "tab" && active && "bg-muted text-foreground",
      )}
    >
      {Icon && <Icon />}
      {children}
    </Link>
  );
}
