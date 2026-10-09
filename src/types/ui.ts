export type CustomSelectTone = "default" | "success" | "warning" | "info";

export interface CustomSelectOption {
  value: string;
  label: string;
  tone?: CustomSelectTone;
  disabled?: boolean;
}

export type NewRequestProtocol = "http" | "websocket" | "grpc" | "sse" | "curl";

export interface NewRequestDraft {
  protocol: NewRequestProtocol;
  name: string;
  method: string;
  url: string;
}
