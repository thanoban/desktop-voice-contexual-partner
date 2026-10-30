import { useEffect } from "react";
import { getConversationSnapshot, onConversationSnapshot, type ConversationSnapshot } from "@/lib/tauri";
import { useChatStore } from "@/store/chatStore";
import { acceptConversationSnapshot } from "@/store/conversationSnapshot";

/** Subscribe before fetching: revision checks prevent delayed hydration replacing newer events. */
export function useConversationSync(projectId: string) {
  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const apply = (snapshot: ConversationSnapshot) => {
      if (!disposed && acceptConversationSnapshot(useChatStore.getState().snapshotRevision, projectId, snapshot)) {
        useChatStore.getState().applySnapshot(snapshot);
      }
    };
    const hydrate = () => { void getConversationSnapshot().then(apply).catch(() => undefined); };
    const register = async () => {
      // One revision covers transcript and audio state. Unversioned audio edge
      // events cannot overwrite a newer capture, transcription or playback run.
      try {
        const unlisten = await onConversationSnapshot(apply);
        if (disposed) unlisten(); else unlisteners.push(unlisten);
      } catch {
        // Focus hydration remains available if event registration fails.
      }
      if (!disposed) hydrate();
    };
    void register();
    window.addEventListener("focus", hydrate);
    return () => {
      disposed = true;
      window.removeEventListener("focus", hydrate);
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [projectId]);
}
