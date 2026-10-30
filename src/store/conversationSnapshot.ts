import type { ConversationSnapshot } from "@/lib/tauri";

/** One monotonic backend revision spans sessions and projects for this process. */
export function acceptConversationSnapshot(currentRevision: number, projectId: string, snapshot: ConversationSnapshot): boolean {
  return snapshot.project_id === projectId && snapshot.revision > currentRevision;
}
