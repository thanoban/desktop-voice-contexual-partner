import { readFileSync } from "node:fs";
import { strict as assert } from "node:assert";
import { test } from "node:test";
import ts from "typescript";

// Compile the production pure reducer gate; no browser, Tauri runtime or new test dependency.
const source = readFileSync(new URL("../src/store/conversationSnapshot.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext } });
const { acceptConversationSnapshot } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

const storeSource = readFileSync(new URL("../src/store/chatStore.ts", import.meta.url), "utf8");
const compiledStore = ts.transpileModule(storeSource, { compilerOptions: { module: ts.ModuleKind.ESNext } }).outputText;
const loadStore = async (name) => {
  const code = compiledStore.replace('"zustand"', JSON.stringify(import.meta.resolve("zustand"))) + `\n// ${name}`;
  return (await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`)).useChatStore;
};
const idleAudio = { listening: false, transcribing: false, speaking: false };

test("late hydration and duplicate token snapshots cannot replace newer conversation state", () => {
  assert.equal(acceptConversationSnapshot(12, "client-a", { project_id: "client-a", revision: 10 }), false);
  assert.equal(acceptConversationSnapshot(12, "client-a", { project_id: "client-a", revision: 12 }), false);
  assert.equal(acceptConversationSnapshot(12, "client-a", { project_id: "client-a", revision: 13 }), true);
});

test("late events from another workspace are excluded even with a higher revision", () => {
  assert.equal(acceptConversationSnapshot(-1, "client-b", { project_id: "client-a", revision: 100 }), false);
  assert.equal(acceptConversationSnapshot(-1, "client-b", { project_id: "client-b", revision: 101 }), true);
});

test("two window stores converge without duplicating messages and a reopened view hydrates", async () => {
  const main = await loadStore("main");
  const widget = await loadStore("widget");
  const user = { id: "u1", role: "user", content: "Hello", timestamp: 1 };
  const reply = { id: "a1", role: "assistant", content: "Hi", timestamp: 2 };
  const base = { project_id: "personal", session_id: "s1", messages: [user], streaming_content: "", generating: true, audio: idleAudio, error: null, cancellation_requested: false };
  const started = { ...base, revision: 1 };
  const streaming = { ...base, revision: 2, streaming_content: "Hi" };
  const completed = { ...base, revision: 3, messages: [user, reply], generating: false };
  const deliver = (store, snapshot) => {
    if (acceptConversationSnapshot(store.getState().snapshotRevision, "personal", snapshot)) store.getState().applySnapshot(snapshot);
  };
  for (const store of [main, widget]) {
    for (const snapshot of [started, streaming, completed, streaming, completed]) deliver(store, snapshot);
    assert.deepEqual(store.getState().messages, [user, reply]);
    assert.equal(store.getState().isProcessing, false);
    assert.equal(store.getState().streamingContent, "");
  }
  const reopened = await loadStore("reopened");
  deliver(reopened, completed);
  assert.deepEqual(reopened.getState().messages, main.getState().messages);
  deliver(reopened, { ...completed, revision: 4, error: "Connection lost", streaming_content: "Partial reply" });
  assert.equal(reopened.getState().streamingContent, "Partial reply");
  assert.equal(reopened.getState().isProcessing, false);
  assert.equal(reopened.getState().messages.filter((message) => message.content.startsWith("[Error:")).length, 1);
  reopened.getState().clearMessages();
  deliver(reopened, completed);
  assert.deepEqual(reopened.getState().messages, []);
  assert.equal(reopened.getState().snapshotRevision, 4);
  deliver(reopened, { ...completed, revision: 5, session_id: "s2", messages: [] });
  assert.equal(reopened.getState().currentSessionId, "s2");
});

test("independent reopened views recover capture, transcription and playback without edge events", async () => {
  const cases = [
    { audio: { ...idleAudio, listening: true }, generating: false, expected: [true, false, false] },
    { audio: { ...idleAudio, transcribing: true }, generating: false, expected: [false, true, false] },
    { audio: { ...idleAudio, speaking: true }, generating: false, expected: [false, false, true] },
    { audio: idleAudio, generating: true, expected: [false, true, false] },
    { audio: { ...idleAudio, transcribing: true, speaking: true }, generating: true, expected: [false, true, true] },
  ];
  for (const [index, scenario] of cases.entries()) {
    const main = await loadStore(`audio-main-${index}`);
    const reopened = await loadStore(`audio-reopened-${index}`);
    const snapshot = { revision: 20, project_id: "personal", session_id: "s1", messages: [], streaming_content: "", error: null, cancellation_requested: false, audio: scenario.audio, generating: scenario.generating };
    for (const store of [main, reopened]) {
      store.getState().applySnapshot(snapshot);
      const state = store.getState();
      assert.deepEqual([state.isListening, state.isProcessing, state.isSpeaking], scenario.expected);
      assert.equal(state.currentSessionId, "s1");
    }
    assert.deepEqual(reopened.getState().messages, main.getState().messages);
  }
});

test("stale completion and duplicate snapshots cannot clear a newer audio run", async () => {
  const store = await loadStore("audio-races");
  const base = { project_id: "personal", session_id: "s1", messages: [], streaming_content: "", generating: false, error: null, cancellation_requested: false };
  const check = (listening, processing, speaking) => {
    const state = store.getState();
    assert.deepEqual([state.isListening, state.isProcessing, state.isSpeaking], [listening, processing, speaking]);
  };
  store.getState().applySnapshot({ ...base, revision: 10, audio: { ...idleAudio, transcribing: true } });
  store.getState().applySnapshot({ ...base, revision: 9, audio: idleAudio });
  store.getState().applySnapshot({ ...base, revision: 10, audio: idleAudio });
  check(false, true, false);
  store.getState().applySnapshot({ ...base, revision: 11, generating: true, audio: idleAudio });
  store.getState().applySnapshot({ ...base, revision: 10, audio: idleAudio });
  check(false, true, false);
  store.getState().applySnapshot({ ...base, revision: 12, audio: { ...idleAudio, speaking: true } });
  store.getState().applySnapshot({ ...base, revision: 11, audio: idleAudio });
  check(false, false, true);
  store.getState().applySnapshot({ ...base, revision: 13, audio: { ...idleAudio, listening: true } });
  store.getState().applySnapshot({ ...base, revision: 12, audio: idleAudio });
  check(true, false, false);
  store.getState().applySnapshot({ ...base, revision: 14, audio: idleAudio });
  check(false, false, false);
});
