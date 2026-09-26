"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { CircleAlertIcon, Loader2Icon } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { FormField, fieldA11y } from "@/components/forms/FormField";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { errorMessage } from "@/lib/api-error";

import { useLoginMutation } from "./authApi";
import { homeFor, safeNext } from "./roles";

const schema = z.object({
  email: z.string().trim().min(1, "Enter your email address.").pipe(z.email("Enter a valid email address, like name@company.com.")),
  password: z.string().min(1, "Enter your password."),
});

type Values = z.infer<typeof schema>;

export function LoginForm({ next }: { next?: string }) {
  const router = useRouter();
  const [login, { isLoading }] = useLoginMutation();
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { email: "", password: "" } });

  const onSubmit = handleSubmit(async (values) => {
    setFormError(null);
    try {
      const session = await login(values).unwrap();
      router.replace(safeNext(next) ?? homeFor(session.user.role));
    } catch (err) {
      setFormError(errorMessage(err as Parameters<typeof errorMessage>[0]));
    }
  });

  return (
    <form onSubmit={onSubmit} noValidate className="flex flex-col gap-5">
      {formError && (
        <Alert variant="destructive">
          <CircleAlertIcon />
          <AlertDescription>{formError}</AlertDescription>
        </Alert>
      )}
      <FormField id="email" label="Email" error={errors.email?.message}>
        <Input
          {...fieldA11y("email", errors.email?.message)}
          type="email"
          autoComplete="email"
          autoFocus
          placeholder="you@company.com"
          {...register("email")}
        />
      </FormField>
      <FormField id="password" label="Password" error={errors.password?.message}>
        <Input
          {...fieldA11y("password", errors.password?.message)}
          type="password"
          autoComplete="current-password"
          {...register("password")}
        />
      </FormField>
      <Button type="submit" size="lg" disabled={isLoading}>
        {isLoading && <Loader2Icon className="animate-spin" />}
        {isLoading ? "Signing in…" : "Sign in"}
      </Button>
      <p className="text-center text-sm text-muted-foreground">
        New here?{" "}
        <Link href="/register" className="font-medium text-accent-foreground hover:underline">
          Create an account
        </Link>
      </p>
    </form>
  );
}
