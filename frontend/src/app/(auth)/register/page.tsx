import type { Metadata } from "next";

import { RegisterForm } from "@/features/auth/RegisterForm";

export const metadata: Metadata = { title: "Create an account" };

export default function RegisterPage() {
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold tracking-tight">Create your account</h1>
        <p className="text-sm text-muted-foreground">Send us requests and follow every reply in one place.</p>
      </div>
      <RegisterForm />
    </div>
  );
}
