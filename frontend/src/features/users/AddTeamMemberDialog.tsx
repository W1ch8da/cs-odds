"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { CircleAlertIcon, Loader2Icon, UserPlusIcon } from "lucide-react";
import { useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { toast } from "sonner";
import { z } from "zod";

import { FormField, fieldA11y } from "@/components/forms/FormField";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { errorMessage } from "@/lib/api-error";

import { useCreateTeamMemberMutation } from "./usersApi";

const ROLE_ITEMS = [
  { value: "agent", label: "Agent: works tickets" },
  { value: "admin", label: "Admin: also manages the team and settings" },
] as const;

const schema = z.object({
  name: z.string().trim().min(1, "Enter their name.").max(100, "Keep the name under 100 characters."),
  email: z.string().trim().min(1, "Enter their work email.").pipe(z.email("Enter a valid email address, like name@company.com.")),
  password: z.string().min(10, "Use at least 10 characters.").max(128, "Use at most 128 characters."),
  role: z.enum(["agent", "admin"]),
});

type Values = z.infer<typeof schema>;

const DEFAULTS: Values = { name: "", email: "", password: "", role: "agent" };
const PASSWORD_HINT = "Share it with them securely. They can sign in right away.";

export function AddTeamMemberDialog() {
  const [open, setOpen] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [create, { isLoading }] = useCreateTeamMemberMutation();
  const {
    register,
    control,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: DEFAULTS });

  const onOpenChange = (next: boolean) => {
    setOpen(next);
    if (!next) {
      reset(DEFAULTS);
      setFormError(null);
    }
  };

  const onSubmit = handleSubmit(async (values) => {
    setFormError(null);
    try {
      const user = await create(values).unwrap();
      toast.success(`${user.name} was added as ${user.role === "admin" ? "an admin" : "an agent"}`);
      onOpenChange(false);
    } catch (err) {
      setFormError(errorMessage(err as Parameters<typeof errorMessage>[0]));
    }
  });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogTrigger render={<Button />}>
        <UserPlusIcon />
        Add team member
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <form onSubmit={onSubmit} noValidate className="flex flex-col gap-5">
          <DialogHeader>
            <DialogTitle>Add a team member</DialogTitle>
            <DialogDescription>Agents work tickets. Admins can also manage the team.</DialogDescription>
          </DialogHeader>
          {formError && (
            <Alert variant="destructive">
              <CircleAlertIcon />
              <AlertDescription>{formError}</AlertDescription>
            </Alert>
          )}
          <FormField id="member-name" label="Full name" error={errors.name?.message}>
            <Input {...fieldA11y("member-name", errors.name?.message)} autoFocus {...register("name")} />
          </FormField>
          <FormField id="member-email" label="Work email" error={errors.email?.message}>
            <Input {...fieldA11y("member-email", errors.email?.message)} type="email" {...register("email")} />
          </FormField>
          <FormField id="member-role" label="Role" error={errors.role?.message}>
            <Controller
              control={control}
              name="role"
              render={({ field }) => (
                <Select items={ROLE_ITEMS} value={field.value} onValueChange={(v) => v && field.onChange(v)}>
                  <SelectTrigger id="member-role" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {ROLE_ITEMS.map((item) => (
                      <SelectItem key={item.value} value={item.value}>
                        {item.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            />
          </FormField>
          <FormField id="member-password" label="Temporary password" hint={PASSWORD_HINT} error={errors.password?.message}>
            <Input
              {...fieldA11y("member-password", errors.password?.message, PASSWORD_HINT)}
              type="password"
              autoComplete="new-password"
              {...register("password")}
            />
          </FormField>
          <DialogFooter>
            <DialogClose render={<Button variant="outline" type="button" />}>Cancel</DialogClose>
            <Button type="submit" disabled={isLoading}>
              {isLoading && <Loader2Icon className="animate-spin" />}
              {isLoading ? "Adding…" : "Add team member"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
