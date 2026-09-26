"use client";

import { ThemeToggle } from "@/components/theme/ThemeToggle";
import { UserMenu } from "@/features/auth/UserMenu";

import { Brand } from "./Brand";
import { NavLink } from "./NavLink";

/** Customer portal: calm top navigation and a centred reading column. */
export function PortalShell({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-svh">
      <header className="sticky top-0 z-20 border-b bg-card">
        <div className="mx-auto flex h-14 max-w-4xl items-center gap-5 px-4">
          <Brand href="/portal" />
          <nav aria-label="Portal" className="flex gap-1">
            <NavLink href="/portal" variant="tab" exact alsoActiveUnder="/portal/requests/">
              My requests
            </NavLink>
            <NavLink href="/portal/new" variant="tab">
              New request
            </NavLink>
          </nav>
          <div className="ml-auto flex items-center gap-1">
            <ThemeToggle />
            <UserMenu compact />
          </div>
        </div>
      </header>
      <main className="mx-auto flex max-w-4xl flex-col gap-7 px-4 py-8">{children}</main>
    </div>
  );
}
