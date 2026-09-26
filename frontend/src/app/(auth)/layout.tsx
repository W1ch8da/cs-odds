import { Brand } from "@/components/layout/Brand";
import { ThemeToggle } from "@/components/theme/ThemeToggle";

export default function AuthLayout({ children }: LayoutProps<"/">) {
  return (
    <div className="relative flex min-h-svh flex-col items-center justify-center px-4 py-12">
      <div className="absolute top-3 right-3">
        <ThemeToggle />
      </div>
      <Brand subtitle="Support" className="mb-8" />
      <div className="w-full max-w-sm rounded-xl border bg-card p-6 shadow-sm sm:p-7">{children}</div>
    </div>
  );
}
