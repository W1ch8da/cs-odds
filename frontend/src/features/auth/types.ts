export type Role = "admin" | "agent" | "customer";

export type User = {
  id: string;
  email: string;
  name: string;
  role: Role;
  createdAt: string;
};

export type Session = {
  user: User;
  accessExpiresAt: string;
};
