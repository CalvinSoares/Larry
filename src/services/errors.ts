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
    const technical = typeof record.technical === "string" ? record.technical : "";
    const diagnostic = record.diagnostic as Record<string, unknown> | undefined;
    const diagnosticSummary =
      diagnostic && typeof diagnostic.summary === "string"
        ? diagnostic.summary
        : "";
    const diagnosticLayer =
      diagnostic && typeof diagnostic.layer === "string" ? diagnostic.layer : "";
    const diagnosticTechnical =
      diagnostic && typeof diagnostic.technical === "string"
        ? diagnostic.technical
        : "";
    const diagnosticText = diagnosticSummary
      ? ` Camada: ${diagnosticLayer || "desconhecida"}. ${diagnosticSummary}${diagnosticTechnical ? ` (${diagnosticTechnical})` : ""}`
      : "";

    if (kind && message) {
      return `[${kind}] ${message}${technical ? ` (${technical})` : ""}${diagnosticText}`;
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
