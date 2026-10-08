export function formatIpcError(value: unknown): string {
  if (typeof value === "string") {
    return value;
  }

  if (value instanceof Error) {
    return value.message;
  }

  if (typeof value === "object" && value !== null) {
    const record = value as Record<string, unknown>;
    const kind = typeof record.kind === "string" ? record.kind : "";
    const message = typeof record.message === "string" ? record.message : "";

    if (kind && message) {
      return `[${kind}] ${message}`;
    }

    if (message) {
      return message;
    }
  }

  try {
    return JSON.stringify(value) ?? String(value);
  } catch {
    return String(value);
  }
}
