export const CONTRACT_SCHEMA_VERSION = 1 as const;

export type PartnerMode = "company" | "work" | "focus";
export type InferenceProfile = "private_local" | "hybrid" | "cloud_voice";
export type CapabilityKind = "tool" | "workflow" | "agent";
export type Effect =
  | "read"
  | "write"
  | "send"
  | "delete"
  | "execute"
  | "schedule"
  | "unknown";
export type RunState =
  | "queued"
  | "awaiting_approval"
  | "running"
  | "succeeded"
  | "failed"
  | "cancellation_requested"
  | "cancelled"
  | "outcome_unknown";

export interface EventEnvelope<T = unknown> {
  schema_version: typeof CONTRACT_SCHEMA_VERSION;
  event_id: string;
  sequence: number;
  occurred_at: number;
  session_id?: string | null;
  turn_id?: string | null;
  run_id?: string | null;
  kind: string;
  payload: T;
}

export interface ProviderCapabilities {
  provider_id: string;
  model_id: string;
  tested_revision?: string | null;
  text: boolean;
  audio_input: boolean;
  audio_output: boolean;
  vision: boolean;
  tools: boolean;
  streaming: boolean;
  cancellation: boolean;
}

export interface IntegrationConnection {
  id: string;
  adapter_id: string;
  display_name: string;
  endpoint?: string | null;
  credential_ref?: string | null;
  enabled: boolean;
  locality: "local_process" | "local_network" | "remote" | "unknown";
  downstream_data_policy: "local_verified" | "may_use_cloud" | "cloud" | "unknown";
  health: "unknown" | "healthy" | "degraded" | "unavailable" | "authentication_required";
  supported_version?: string | null;
  tested_version?: string | null;
}

export interface ExecutionRequest {
  id: string;
  capability_id: string;
  project_id?: string | null;
  session_id?: string | null;
  arguments: unknown;
  objective?: string | null;
  context_refs: string[];
  grant_id?: string | null;
  deadline_at?: number | null;
  budget_micros?: number | null;
  ancestry: string[];
}

export interface ExecutionRun {
  id: string;
  request_id: string;
  external_id?: string | null;
  executor_id: string;
  project_id?: string | null;
  approved_request_hash?: string | null;
  state: RunState;
  sequence: number;
  created_at: number;
  updated_at: number;
  verification_state: string;
  artifact_refs: string[];
}
