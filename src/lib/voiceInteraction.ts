/** Serialize capture transitions while allowing cancellation during transcription or generation. */
export function createVoiceInteraction() {
  let capturePending = false;
  return async (options: {
    state: () => { isListening: boolean; isProcessing: boolean; isSpeaking: boolean };
    disabled: boolean;
    start: () => Promise<void>;
    stop: () => Promise<string>;
    cancel: () => Promise<unknown>;
    send: (text: string) => Promise<void>;
    isCurrentProject: () => boolean;
  }): Promise<void> => {
    const state = options.state();
    if (state.isSpeaking || state.isProcessing) { await options.cancel(); return; }
    if (capturePending || (options.disabled && !state.isListening)) return;
    capturePending = true;
    let transcript = "";
    try {
      if (state.isListening) transcript = await options.stop();
      else await options.start();
    } finally { capturePending = false; }
    if (transcript.trim() && options.isCurrentProject()) await options.send(transcript.trim());
  };
}
