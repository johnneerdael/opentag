import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/document-state.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
const { documentSelection, refreshDocumentAccess, copyRequest, copyOutcome, sourcePresentation, publicDocumentMessage, reconcileDrafts } = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
const path = '/data/user/0/app/files/documents/uuid/Mémoire 猫.wav';
const ready = { uri:'content://audio.provider/document/37', name:'Mémoire 猫.wav', writable:true, persisted:true, state:'ready', error:null };
const access = (state=ready) => ({ states:{ [path]:state }, refreshError:null });

test('known ready writable sources allow immediate publication and a single copy', () => {
  const policy = documentSelection([path], access(), true);
  assert.equal(policy.mutationReason, '');
  assert.equal(policy.copyReason, '');
  assert.ok(policy.unsupportedReason);
});
test('read-only documents retain draft and copy access while blocking immediate mutations', () => {
  const policy = documentSelection([path], access({...ready,writable:false}), true);
  assert.match(policy.mutationReason, /read.only/i);
  assert.equal(policy.copyReason, '');
});
test('recovery sources cannot publish or reload and remain available for copy recovery', () => {
  const policy = documentSelection([path], access({...ready,state:'recovery_required',error:'Source changed'}), true);
  assert.match(policy.mutationReason, /recovery/i);
  assert.ok(policy.reloadReason);
  assert.equal(policy.copyReason, '');
});
test('unknown Android source state fails closed without guessing ordinary filesystem access', () => {
  const policy = documentSelection([path], {states:{},refreshError:null}, true);
  assert.ok(policy.mutationReason);
  assert.ok(policy.copyReason);
  assert.ok(policy.reloadReason);
  assert.ok(policy.unsupportedReason);
});
test('mixed source selections block publication and multi-file Save a copy', () => {
  const policy = documentSelection([path,'/music/local.wav'], access(), false);
  assert.ok(policy.mutationReason);
  assert.ok(policy.copyReason);
  assert.ok(policy.unsupportedReason);
});
test('ordinary desktop selections retain existing operation policy', () => {
  const policy = documentSelection(['/music/local.wav'], {states:{},refreshError:null}, false);
  assert.equal(policy.mutationReason, '');
  assert.equal(policy.reloadReason, '');
  assert.equal(policy.unsupportedReason, '');
  assert.ok(policy.copyReason);
});
test('refresh failure retains the last source error and blocks mutation until a successful refresh', async () => {
  const original = access({...ready,state:'recovery_required',error:'Provider write was interrupted'});
  const result = await refreshDocumentAccess(original, async()=>{throw new Error('state service unavailable');});
  assert.equal(result.states, original.states);
  assert.equal(result.states[path].error, 'Provider write was interrupted');
  assert.match(result.refreshError, /unavailable/);
  assert.ok(documentSelection([path], result, true).mutationReason);
  const recovered = await refreshDocumentAccess(result, async()=>({[path]:ready}));
  assert.equal(recovered.refreshError, null);
  assert.equal(documentSelection([path], recovered, true).mutationReason, '');
});
test('edited copy includes a snapshot of pending fields and original backup ignores them', () => {
  const draft = { TITLE:['Edited title'], COMMENT:[] };
  const edited = copyRequest(path,'edited',draft);
  assert.deepEqual(edited,{path,version:'edited',updates:{TITLE:['Edited title'],COMMENT:[]}});
  draft.TITLE[0]='Changed later';
  assert.equal(edited.updates.TITLE[0], 'Edited title');
  assert.deepEqual(copyRequest(path,'original',draft),{path,version:'original',updates:{}});
});
test('edited copy snapshots pending artwork, including explicit removal; original ignores it', () => {
  const draft={TITLE:['Pending']}, covers=[{digest:'cover-1',picture_type:'CoverBack',description:'Pending cover'}];
  const request=copyRequest(path,'edited',draft,covers);
  assert.deepEqual(request,{path,version:'edited',updates:draft,covers});
  covers[0].description='Changed later';covers.push({digest:'cover-2',picture_type:'CoverFront',description:''});
  assert.equal(request.covers.length,1);
  assert.equal(request.covers[0].description,'Pending cover');
  assert.deepEqual(copyRequest(path,'edited',{},[]),{path,version:'edited',updates:{},covers:[]});
  assert.deepEqual(copyRequest(path,'edited',{}),{path,version:'edited',updates:{}});
  assert.deepEqual(copyRequest(path,'original',draft,covers),{path,version:'original',updates:{}});
  assert.deepEqual(copyRequest(path,'original',draft,[]),{path,version:'original',updates:{}});
});
test('copy cancellation never reports a save and copy failures remain failures', () => {
  assert.equal(copyOutcome({entries:[],errors:[]}), 'cancelled');
  assert.equal(copyOutcome({entries:[],errors:[{path,error:'Destination failed'}]}), 'failed');
  assert.equal(copyOutcome({entries:[{path:'copy'}],errors:[]}), 'saved');
});
test('source presentation uses provider identity and distinguishes grant, recovery and unknown access', () => {
  const source=sourcePresentation(ready);
  assert.equal(source.name,'Mémoire 猫.wav');
  assert.equal(source.location,'content://audio.provider/document/37');
  assert.equal(source.detail,'Access retained · Save a copy available');
  assert.equal(sourcePresentation({...ready,persisted:false}).detail,'Access for this session · Save a copy available');
  assert.match(sourcePresentation({...ready,writable:false}).label,/read.only/i);
  for(const persisted of [true,false]) {
    const recovery=sourcePresentation({...ready,persisted,state:'recovery_required'});
    assert.match(recovery.label,/recovery/i);
    assert.match(recovery.detail,/^Previous source details: /);
    assert.match(recovery.detail,/Edited and original copies retained$/);
    assert.equal(recovery.name,source.name);
    assert.equal(recovery.location,source.location);
    assert.match(sourcePresentation({...ready,persisted},true).detail,/^Previous source details: /);
  }
  assert.match(sourcePresentation(undefined).label,/unknown/i);
});
test('source errors redact working paths and private storage details before display', () => {
  assert.equal(publicDocumentMessage(`Could not write ${path}`, access().states), 'Could not write Mémoire 猫.wav');
  assert.equal(publicDocumentMessage('failed /data/user/0/app/files/documents/other/track.wav', {}), 'The document operation failed. Private working files were retained.');
  assert.equal(publicDocumentMessage('Provider access was revoked', {}), 'Provider access was revoked');
});

test('failed publication and state refresh preserve drafts while successful files clear independently', async () => {
  const drafts=new Map([[path,{TITLE:['Pending']}],['/music/other.wav',{ARTIST:['Other']}] ]);
  reconcileDrafts(drafts, [], true);
  await refreshDocumentAccess(access(), async()=>{throw new Error('offline');});
  assert.deepEqual([...drafts],[[path,{TITLE:['Pending']}],['/music/other.wav',{ARTIST:['Other']}] ]);
  reconcileDrafts(drafts,['/music/other.wav'],true);
  assert.deepEqual([...drafts],[[path,{TITLE:['Pending']}] ]);
});
test('cancelled, failed and successful copies retain source drafts including original-backup copies', () => {
  const drafts=new Map([[path,{TITLE:['Keep this draft']}] ]);
  for(const batch of [{entries:[],errors:[]},{entries:[],errors:[{error:'failed'}]},{entries:[{path:'new-copy'}],errors:[]}]) {
    reconcileDrafts(drafts,batch.entries.map(entry=>entry.path),false);
    assert.deepEqual([...drafts],[[path,{TITLE:['Keep this draft']}] ]);
  }
});

test('failed refresh presents access as unknown while retaining previous source identity', () => {
  const previous=sourcePresentation(ready,true);
  assert.match(previous.label,/unknown/i);
  assert.equal(previous.name,'Mémoire 猫.wav');
  assert.match(previous.detail,/previous/i);
});
