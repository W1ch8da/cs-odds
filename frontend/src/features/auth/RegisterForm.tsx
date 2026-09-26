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

import { useRegisterMutation } from "./authApi";
import { homeFor } from "./roles";

// Mirrors the backend rules in domain/user.rs so most mistakes are caught before submitting.
const schema = z.object({
  name: z.string().trim().min(1, "Enter your name.").max(100, "Keep the name under 100 characters."),
  email: z.string().trim().min(1, "Enter your email address.").pipe(z.email("Enter a valid email address, like name@company.com.")),
  password: z
    .string()
    .min(10, "Use at least 10 characters for your password.")
    .max(128, "Use at most 128 characters for your password."),
});

type Values = z.infer<typeof schema>;

const PASSWORD_HINT = "At least 10 characters. A short phrase is easier to remember.";

export function RegisterForm() {
  const router = useRouter();
  const [registerAccount, { isLoading }] = useRegisterMutation();
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { name: "", email: "", password: "" } });

  const onSubmit = handleSubmit(async (values) => {
    setFormError(null);
    try {
      const session = await registerAccount(values).unwrap();
      router.replace(homeFor(session.user.role));
    } catch (err) {
      setFormError(errorMessage(err as Parameters<typeof errorMessage>[0]));
    }
  });

  return (
    <form onSubmit={onSubmit} noValidate className="flex flex-col gap-5">
      {formError && (
        <Alert variant="destructive">
          <CircleAlertIcon />
          <AlertDescription>
            {formError}
            {formError.includes("already exists") && (
              <>
                {" "}
                <Link href="/login" className="font-medium underline">
                  Sign in
                </Link>
              </>
            )}
          </AlertDescription>
        </Alert>
      )}
      <FormField id="name" label="Full name" error={errors.name?.message}>
        <Input {...fieldA11y("name", errors.name?.message)} autoComplete="name" autoFocus {...register("name")} />
      </FormField>
      <FormField id="email" label="Work email" error={errors.email?.message}>
        <Input
          {...fieldA11y("email", errors.email?.message)}
          type="email"
          autoComplete="email"
          placeholder="you@company.com"
          {...register("email")}
        />
      </FormField>
      <FormField id="password" label="Password" hint={PASSWORD_HINT} error={errors.password?.message}>
        <Input
          {...fieldA11y("password", errors.password?.message, PASSWORD_HINT)}
          type="password"
          autoComplete="new-password"
          {...register("password")}
        />
      </FormField>
      <Button type="submit" size="lg" disabled={isLoading}>
        {isLoading && <Loader2Icon className="animate-spin" />}
        {isLoading ? "Creating your account…" : "Create account"}
      </Button>
      <p className="text-center text-sm text-muted-foreground">
        Already have an account?{" "}
        <Link href="/login" className="font-medium text-accent-foreground hover:underline">
          Sign in
        </Link>
      </p>
    </form>
  );
}
