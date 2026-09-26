import { AgentShell } from "@/components/layout/AgentShell";
import { RequireRole } from "@/features/auth/RequireRole";

export default function AdminLayout({ children }: LayoutProps<"/admin">) {
  return (
    <RequireRole roles={["admin"]}>
      <AgentShell>{children}</AgentShell>
    </RequireRole>
  );
}
