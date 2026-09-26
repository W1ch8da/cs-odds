import { AgentShell } from "@/components/layout/AgentShell";
import { RequireRole } from "@/features/auth/RequireRole";

export default function AgentLayout({ children }: LayoutProps<"/agent">) {
  return (
    <RequireRole roles={["agent", "admin"]}>
      <AgentShell>{children}</AgentShell>
    </RequireRole>
  );
}
