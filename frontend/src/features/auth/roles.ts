import type { Role } from "./types";

export const ROLE_LABELS: Record<Role, string> = {
  admin: "Admin",
  agent: "Agent",
  customer: "Customer",
};

/** Where each role lands after signing in. */
export function homeFor(role: Role): string {
  return role === "customer" ? "/portal" : "/agent/tickets";
}

export function isStaff(role: Role): boolean {
  return role === "admin" || role === "agent";
}

/** Only allow same-site relative paths, to prevent open redirects. */
export function safeNext(next: string | undefined): string | undefined {
  return next && next.startsWith("/") && !next.startsWith("//") ? next : undefined;
}

export function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]!.toUpperCase())
    .join("");
}
