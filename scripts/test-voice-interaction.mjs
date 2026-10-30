import { readFileSync } from 'node:fs';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import ts from 'typescript';
const source = readFileSync(new URL('../src/lib/voiceInteraction.ts', import.meta.url), 'utf8');
const code = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext } }).outputText;
const { createVoiceInteraction } = await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`);
const deferred = () => { let resolve; const promise = new Promise(done => { resolve = done; }); return { promise, resolve }; };
function fixture() {
  const calls = [];
  const state = { isListening: false, isProcessing: false, isSpeaking: false };
  return { calls, state, options: { state: () => state, disabled: false, start: async () => { calls.push('start'); }, stop: async () => { calls.push('stop'); return 'hello'; }, cancel: async () => { calls.push('cancel'); }, send: async text => { calls.push(text); }, isCurrentProject: () => true } };
}
test('competing clicks share one capture transition; rejection releases admission', async () => {
  const f = fixture(), gate = deferred(), toggle = createVoiceInteraction();
  f.options.start = async () => { f.calls.push('start'); await gate.promise; throw Error('device'); };
  const first = toggle(f.options);
  await toggle(f.options);
  assert.deepEqual(f.calls, ['start']);
  gate.resolve(); await assert.rejects(first, /device/);
  f.options.start = async () => { f.calls.push('retry'); };
  await toggle(f.options); assert.deepEqual(f.calls, ['start', 'retry']);
});
test('stop stays available disconnected, serializes stop, and excludes stale project transcript', async () => {
  const f = fixture(), gate = deferred(), toggle = createVoiceInteraction();
  f.state.isListening = true; f.options.disabled = true;
  f.options.stop = async () => { f.calls.push('stop'); return gate.promise; };
  const first = toggle(f.options); await toggle(f.options);
  f.options.isCurrentProject = () => false;
  gate.resolve('private transcript'); await first;
  assert.deepEqual(f.calls, ['stop']);
});
test('cancel remains callable during pending transcription and generation; blank audio never sends', async () => {
  const f = fixture(), gate = deferred(), toggle = createVoiceInteraction();
  f.state.isListening = true;
  f.options.stop = async () => { f.calls.push('stop'); return gate.promise; };
  const first = toggle(f.options);
  f.state.isListening = false; f.state.isProcessing = true; f.options.disabled = true;
  await toggle(f.options); assert.deepEqual(f.calls, ['stop', 'cancel']);
  gate.resolve('   '); await first;
  f.state.isProcessing = false; f.state.isSpeaking = true;
  await toggle(f.options); assert.deepEqual(f.calls, ['stop', 'cancel', 'cancel']);
});
