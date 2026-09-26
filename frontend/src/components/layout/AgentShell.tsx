"use client";

import { UsersIcon } from "lucide-react";
import { Suspense } from "react";

import { ThemeToggle } from "@/components/theme/ThemeToggle";
import { selectCurrentUser } from "@/features/auth/authSlice";
import { UserMenu } from "@/features/auth/UserMenu";
import { QueueViewsNav } from "@/features/tickets/components/QueueViewsNav";
import { useAppSelector } from "@/lib/store/hooks";

import { Brand } from "./Brand";
import { NavLink } from "./NavLink";

function SectionLabel({ children }: { children: React.ReactNode }) {
  return <p className="px-2.5 pb-1.5 text-[11px] font-medium tracking-wider text-faint uppercase">{children}</p>;
}

/** Workspace for agents and admins: sidebar on desktop, top bar on small screens. */
export function AgentShell({ children }: { children: React.ReactNode }) {
  const user = useAppSelector(selectCurrentUser);
  const isAdmin = user?.role === "admin";

  return (
    <div className="min-h-svh md:grid md:grid-cols-[15rem_minmax(0,1fr)]">
      <aside className="sticky top-0 hidden h-svh flex-col gap-6 overflow-y-auto border-r bg-sidebar px-2.5 py-4 md:flex">
        <Brand href="/agent/tickets" subtitle="Support workspace" className="px-2" />
        <div className="flex flex-col gap-5">
          <div>
            <SectionLabel>Tickets</SectionLabel>
            {/* Reads the ?view= search param, which needs a Suspense boundary. */}
            <Suspense>
              <QueueViewsNav />
            </Suspense>
          </div>
          {isAdmin && (
            <nav aria-label="Admin" className="flex flex-col gap-0.5">
              <SectionLabel>Admin</SectionLabel>
              <NavLink href="/admin/users" icon={UsersIcon}>
                Team &amp; customers
              </NavLink>
            </nav>
          )}
        </div>
        <div className="mt-auto flex flex-col gap-1">
          <ThemeToggle withLabel />
          <UserMenu side="top" />
        </div>
      </aside>

      <header className="sticky top-0 z-20 flex flex-col gap-2 border-b bg-sidebar px-4 py-2 md:hidden">
        <div className="flex items-center gap-2">
          <Brand href="/agent/tickets" />
          <div className="ml-auto flex items-center gap-1">
            {isAdmin && (
              <NavLink href="/admin/users" variant="tab">
                Team
              </NavLink>
            )}
            <ThemeToggle />
            <UserMenu compact />
          </div>
        </div>
        <Suspense>
          <QueueViewsNav variant="tabs" />
        </Suspense>
      </header>

      <main className="min-w-0 px-4 py-6 md:px-8 md:py-8">{children}</main>
    </div>
  );
}
