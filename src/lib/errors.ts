import type { AppError } from "../types/server";

export function isAppError(value: unknown): value is AppError {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<AppError>;
  return typeof candidate.code === "string" && typeof candidate.message === "string";
}

export function errorMessage(value: unknown): string {
  if (isAppError(value)) return `${value.message} · ${value.code}`;
  if (value instanceof Error) return value.message;
  return typeof value === "string" ? value : "发生未知错误";
}
