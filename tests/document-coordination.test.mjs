import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

// Exercise production coordination against a controlled provider and dialog boundary.
// Avoid importing main.ts's platform bootstrap or replacing its draft/save/refresh logic.
const main=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');
const helper=await readFile(new URL('../src/document-state.ts',import.meta.url),'utf8');
const moduleURL=async(file,replacements={})=>{
  let js=ts.transpileModule(await readFile(new URL(`../src/${file}`,import.meta.url),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  for(const [from,to] of Object.entries(replacements))js=js.replace(from,JSON.stringify(to));
  return `data:text/javascript;base64,${Buffer.from(js).toString('base64')}`;
};
const coreURL=await moduleURL('i18n/core.ts'),converterURL=await moduleURL('converter-errors.ts',{"'./i18n/core'":coreURL});
const batchURL=await moduleURL('batch-errors.ts',{"'./i18n/core'":coreURL,"'./converter-errors'":converterURL});
const actionURL=await moduleURL('action-ui.ts');
// The cover window markup comes from the real cover modules (without their stylesheet).
// Chapter rows (chapters.ts) supply the list splicing that accept() uses.
const coverModules=(await Promise.all(['cover-model','covers','chapters','tag-clipboard'].map(name=>readFile(new URL(`../src/${name}.ts`,import.meta.url),'utf8')))).map(source=>source.replace(/^import .*$/gm,'').replace(/\bexport /g,'')).join('\n');
const parsed=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
const names=new Set(['updateDraft','run','refreshHistory','saveDrafts','saveCopy','value','accept','operateImpl','refreshSourceAccess','providerEntry','copyBaseUnknown','editedCopyFields','discardDrafts','mutate','reconcileWorkingEntries','storeEntry','invalidateCovers','cacheCovers','coversFor','pendingCoverInfos','hydrateActiveCover','coverMarkup','sourceMarkup','workingMetadataNotice','flushNotices','spliceRows','sourcePolicy','operationReason','enabled','stageTagUpdates','clipTags']);
const mergedDeclaration=parsed.statements.filter(ts.isVariableStatement).flatMap(statement=>statement.declarationList.declarations).find(declaration=>declaration.name.getText(parsed)==='merged');
const functions=parsed.statements.filter(node=>ts.isFunctionDeclaration(node)&&names.has(node.name?.text)).map(node=>node.getText(parsed)).join('\n');
const declarationNames=new Set(['clipTargets','coverTargets','utilOperations','tagSourceOperations','listOperations']);
const declarations=parsed.statements.filter(ts.isVariableStatement).flatMap(statement=>statement.declarationList.declarations).filter(declaration=>declarationNames.has(declaration.name.getText(parsed))).map(declaration=>`const ${declaration.getText(parsed)};`).join('\n');
const factory=`import {BatchFailure} from ${JSON.stringify(batchURL)};
import {panelKey} from ${JSON.stringify(actionURL)};
${helper.replace(/\bexport /g,'')}
${coverModules}
export function createWorkspace(initial, options={}) {
  const path='retained.wav', original=structuredClone(initial), originalCovers=structuredClone(options.covers||[]);
  let working=structuredClone(initial), workingCovers=structuredClone(options.covers||[]), copied, copiedCovers, copyRequestSent, dialog, status='', busy=false, errors=[], location='', anchor;
  const nativeCalls=[],workingReadErrors=new Map();
  const coverEpochs=new Map(),coverCache=new Map(),coverRequests=new Map();
  const coverCacheBytesLimit=12*1024*1024,coverCacheEntryLimit=8;
  let coverCacheBytes=0,activeCover,coverError,coverIndex=0,progressJob;
  const desktop=true,active=()=>entries.get(path),timing=()=>{},refreshCoverPanel=()=>{},themedAsset=()=>'',button=()=>'';
  const coverUI=new CoverUI({selection:()=>paths().map(path=>({path,covers:workingReadErrors.has(path)?[]:entries.get(path).covers})),displayed:()=>({path:active()?.path,covers:coversFor(active()),index:coverIndex}),reason:()=>'',clipboard:()=>false,mac:false});
  const makeEntry=(path,fields)=>({path,filename:path,format:'WAV',duration:1,bitrate:1411,sample_rate:44100,channels:2,size:44,fields:structuredClone(fields),technical:{},covers:structuredClone(workingCovers)});
  const entries=new Map([[path,makeEntry(path,initial)]]),drafts=new Map(),copyOverrides=new Map(),selected=new Set([path]);
  // Clipboard images are controlled here; production pending-cover and copy coordination stay intact.
  const pendingCovers=new Map(),clipCovers=new Map(),clipImages=new Map(),folderImageFor=()=>undefined;
  const ready={uri:'content://provider/audio',name:path,writable:options.writable!==false,persisted:true,state:'ready',error:null};
  let nativeState={...ready,state:options.state||'ready'}, refreshFails=!!options.refreshFails;
  let documentAccess={states:options.provider===false?{}:{[path]:nativeState},refreshError:null};
  const runtimePlatform={document_access_supported:options.provider!==false,folder_picker_supported:options.provider===false,family:options.provider===false?'desktop':'android'};
  const paths=()=>[...selected],dirtySelected=()=>paths().some(path=>drafts.has(path));
  const render=()=>{},changedData=()=>{},esc=value=>String(value),publicMessage=value=>String(value),displayName=value=>value;
  const merged=${mergedDeclaration.initializer.getText(parsed)};
  const ask=async()=>true,modalError=()=>{};
  let tagClipboard;
  const navigator={clipboard:{writeText:async()=>{}}};
  if(options.clipboardSource){entries.set('clipboard.wav',{...makeEntry('clipboard.wav',options.clipboardSource.fields),covers:structuredClone(options.clipboardSource.covers)});documentAccess.states['clipboard.wav']={...ready,uri:'content://provider/clipboard',name:'clipboard.wav',writable:true};}
  let queuedNotices=[],autoPlaylistPending=false;const movedOut=new Set();const autoPlaylist=async()=>{autoPlaylistPending=false;},noticeLabels={},messages={enabled:()=>false,confirm:async()=>true,notify:async()=>{}};
  ${declarations}
  const applyFields=(base,updates)=>{const result=structuredClone(base);for(const [key,values] of Object.entries(updates)){if(values.length)result[key]=structuredClone(values);else delete result[key];}return result;};
  async function invoke(command,args) {
    nativeCalls.push(command);
    if(['remove_tags','filename_to_tags','add_cover','remove_cover'].includes(command)) {
      if(command==='remove_tags'){working={};workingCovers=[];}
      if(command==='filename_to_tags')working=applyFields(working,options.filenameFields||{TITLE:['Filename title']});
      if(command==='add_cover')workingCovers=[{index:0,mime:'image/png',description:'New cover',picture_type:'CoverFront',data_base64:'TkVX'}];
      if(command==='remove_cover')workingCovers.splice(args.index,1);
      nativeState={...ready,state:'recovery_required',error:'Source publication failed after local edit'};
      if(options.mutationThrows)throw new Error(nativeState.error);
      return {entries:[],errors:[{path,error:nativeState.error}]};
    }
    if(command==='document_working_entries') {
      if(options.workingReadFails)return {entries:[],errors:[{path,error:'Retained working bytes unreadable'}]};
      const entry=makeEntry(path,working);entry.covers.forEach(cover=>cover.data_base64='');
      return {entries:[entry],errors:[]};
    }
    if(command==='copy_tag_covers') {
      const images={},covers=args.paths.map(target=>(entries.get(target)?.covers||[]).map((cover,index)=>{
        const digest=cover.digest||'working-cover-'+index;
        images[digest]=cover.data_base64||'UE5H';
        return {digest,picture_type:cover.picture_type,description:cover.description,mime:cover.mime||'image/png',size:4,width:3,height:2};
      }));
      return {covers,images,errors:[]};
    }
    if(command==='get_covers')return structuredClone(workingCovers);
    if(command==='save_fields_batch') {
      for(const item of args.items)working=applyFields(working,item.updates);
      nativeState={...ready,state:'recovery_required',error:'Source publication failed after local edit'};
      return {entries:[],errors:[{path,error:'Source publication failed after local edit'}]};
    }
    if(command==='document_states') {if(refreshFails)throw new Error('State refresh unavailable');return {[path]:nativeState};}
    if(command==='document_save_copy') {
      copyRequestSent=structuredClone(args);
      if(options.copyThrows)throw new Error('Destination publication failed');
      if(options.copyCancels)return {entries:[],errors:[]};
      if(options.copyFails)return {entries:[],errors:[{path:'new-copy',error:'Destination publication failed'}]};
      copiedCovers=structuredClone(args.version==='original'?originalCovers:args.covers??workingCovers);
      copied=applyFields(args.version==='original'?original:working,args.updates);
      return {entries:[makeEntry('copied.wav',copied)],errors:[]};
    }
    throw new Error('Unexpected native boundary: '+command);
  }
  let dialogContent='';
  function modal(title,content) {
    dialogContent=content;
    const controls=new Map();
    for(const selector of ['[data-close]','#copy-version','#save-document-copy','.copy-status'])controls.set(selector,{value:'edited',addEventListener(event,handler){this[event]=handler;}});
    dialog={open:true,controls,querySelector:selector=>controls.get(selector),setAttribute(){},addEventListener(){},close(){this.open=false;}};
    return dialog;
  }
  ${functions}
  return {
    edit:(key,values)=>updateDraft(path,key,values),save:()=>saveDrafts(),
    operate:async(operation,target=path)=>{selected.clear();selected.add(target);await operateImpl(operation);},
    available:enabled,policy:sourcePolicy,
    mutate:(command,args={})=>mutate(command,args,'Applied'),
    hydrate:async()=>{hydrateActiveCover();await Promise.all([...coverRequests.values()]);},
    coverMarkup,sourceMarkup:()=>sourceMarkup('tags'),
    selectCover:index=>coverIndex=index,
    discard:()=>operateImpl('discard'),refresh:()=>operateImpl('refresh-source'),
    setRefreshFailure:failure=>refreshFails=failure,
    acceptImported:fields=>accept({entries:[makeEntry(path,fields)],errors:[]},'Loaded',false),
    acceptLoaded:fields=>{working=structuredClone(fields);nativeState=ready;accept({entries:[makeEntry(path,fields)],errors:[]},'Reloaded');},
    copy:async(version='edited')=>{selected.clear();selected.add(path);await operateImpl('save-copy');dialog.controls.get('#copy-version').value=version;await dialog.controls.get('#save-document-copy').click();return structuredClone(copied);},
    snapshot:()=>({fields:structuredClone(merged(entries.get(path))),drafts:structuredClone([...drafts]),copyOverrides:structuredClone([...copyOverrides]),errors:structuredClone(errors),status,busy,covers:structuredClone(coversFor(entries.get(path))),copiedCovers,copyRequestSent,pendingCovers:structuredClone([...pendingCovers]),coverIndex,nativeCalls:[...nativeCalls],workingReadErrors:[...workingReadErrors],dialogContent}),
  };
}`;
const javascript=ts.transpileModule(factory,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {createWorkspace}=await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

for(const refreshFails of [false,true]) {
  test(`failed publication then reverting to loaded tags exports the displayed intent (${refreshFails?'unknown':'recovery'})`,async()=>{
    const workspace=createWorkspace({TITLE:['A']},{refreshFails});
    workspace.edit('TITLE',['B']);await workspace.save();workspace.edit('TITLE',['A']);
    assert.deepEqual(workspace.snapshot().fields,{TITLE:['A']});
    assert.deepEqual(await workspace.copy(),{TITLE:['A']});
  });
}
test('restoring a removed tag after failed publication exports the restored value',async()=>{
  const workspace=createWorkspace({TITLE:['A']});
  workspace.edit('TITLE',[]);await workspace.save();workspace.edit('TITLE',['A']);
  assert.deepEqual(await workspace.copy(),{TITLE:['A']});
});
test('removing a newly added tag after failed publication exports an explicit deletion',async()=>{
  const workspace=createWorkspace({TITLE:['A']});
  workspace.edit('NEW',['B']);await workspace.save();workspace.edit('NEW',[]);
  const copied=await workspace.copy();assert.equal(Object.hasOwn(copied,'NEW'),false);assert.deepEqual(copied,{TITLE:['A']});
});
for(const field of ['TITLE','NEW']) {
  test(`Discard after failed publication clears pending drafts and exports displayed loaded tags (${field})`,async()=>{
    const workspace=createWorkspace({TITLE:['A']});
    workspace.edit(field,['B']);await workspace.save();await workspace.discard();
    const state=workspace.snapshot();assert.deepEqual(state.drafts,[]);assert.deepEqual(state.fields.TITLE,['A']);assert.equal((state.fields.NEW||[]).length,0);assert.match(state.status,/discarded/i);
    assert.deepEqual(await workspace.copy(),{TITLE:['A']});
  });
}
test('desktop and ready provider undo-to-loaded leave no pending draft',()=>{
  for(const provider of [false,true]){const workspace=createWorkspace({TITLE:['A']},{provider});workspace.edit('TITLE',['B']);workspace.edit('TITLE',['A']);assert.deepEqual(workspace.snapshot().drafts,[]);assert.deepEqual(workspace.snapshot().copyOverrides,[]);assert.deepEqual(workspace.snapshot().fields,{TITLE:['A']});}
});
test('original-backup copy ignores current recovery edits',async()=>{
  const workspace=createWorkspace({TITLE:['A']});workspace.edit('TITLE',['B']);await workspace.save();workspace.edit('TITLE',['A']);workspace.edit('COMMENT',['C']);
  assert.deepEqual(await workspace.copy('original'),{TITLE:['A']});
});
for(const refreshFails of [true,false]) {
  test(`manual ${refreshFails?'failed':'successful'} source refresh retains the original publication error`,async()=>{
    const workspace=createWorkspace({TITLE:['A']},{refreshFails:true});workspace.edit('TITLE',['B']);await workspace.save();
    const before=workspace.snapshot();assert.match(before.errors[0].error,/publication failed/);
    workspace.setRefreshFailure(refreshFails);await workspace.refresh();
    assert.deepEqual(workspace.snapshot().errors,before.errors);assert.deepEqual(workspace.snapshot().drafts,before.drafts);assert.equal(workspace.snapshot().busy,false);
  });
}

test('accepting a verified source baseline clears old recovery copy overrides',async()=>{
  const workspace=createWorkspace({TITLE:['A']});workspace.edit('TITLE',['B']);await workspace.save();workspace.edit('TITLE',['A']);
  assert.notDeepEqual(workspace.snapshot().copyOverrides,[]);
  workspace.acceptLoaded({TITLE:['Fresh source']});
  assert.deepEqual(workspace.snapshot().copyOverrides,[]);
  assert.deepEqual(workspace.snapshot().drafts,[]);
  assert.deepEqual(workspace.snapshot().fields,{TITLE:['Fresh source']});
  assert.deepEqual(await workspace.copy(),{TITLE:['Fresh source']});
});


test('failed Remove tags reconciles retained tags before a later field edit and edited copy',async()=>{
  const workspace=createWorkspace({TITLE:['Old title'],ARTIST:['Original artist']});
  await workspace.mutate('remove_tags');
  assert.deepEqual(workspace.snapshot().fields,{});
  assert.match(workspace.snapshot().errors[0].error,/publication failed/);
  workspace.edit('TITLE',['Replacement title']);
  assert.deepEqual(workspace.snapshot().fields,{TITLE:['Replacement title']});
  assert.deepEqual(await workspace.copy(),{TITLE:['Replacement title']});
});
test('failed filename conversion preserves pending fields and deletion intent over retained converted tags',async()=>{
  const workspace=createWorkspace({TITLE:['Old'],ARTIST:['Artist']},{filenameFields:{TITLE:['Filename title'],TRACK:['03']}});
  workspace.edit('COMMENT',['Pending']);workspace.edit('ARTIST',[]);
  await workspace.mutate('filename_to_tags',{pattern:'%title%'});
  assert.deepEqual(workspace.snapshot().fields,{TITLE:['Filename title'],TRACK:['03'],ARTIST:[],COMMENT:['Pending']});
  assert.deepEqual(await workspace.copy(),{TITLE:['Filename title'],TRACK:['03'],COMMENT:['Pending']});
});
test('Discard after failed immediate conversion reverts only pending edits, retaining the applied conversion',async()=>{
  const workspace=createWorkspace({TITLE:['Old'],ARTIST:['Artist']});
  await workspace.mutate('filename_to_tags',{pattern:'%title%'});
  workspace.edit('TITLE',['Pending']);workspace.edit('NEW',['Pending']);await workspace.discard();
  assert.deepEqual(workspace.snapshot().drafts,[]);
  assert.deepEqual(await workspace.copy(),{TITLE:['Filename title'],ARTIST:['Artist']});
});
test('immediate reconciliation preserves an explicit recovery reset override',async()=>{
  const workspace=createWorkspace({TITLE:['Old'],ARTIST:['Artist']});
  workspace.edit('TITLE',['Changed']);await workspace.save();workspace.edit('TITLE',['Old']);
  await workspace.mutate('remove_tags');
  assert.deepEqual(workspace.snapshot().fields,{TITLE:['Old']});
  assert.deepEqual(await workspace.copy(),{TITLE:['Old']});
});
const oldCover={index:0,mime:'image/png',description:'Old cover',picture_type:'CoverFront',data_base64:'T0xE'};
for(const scenario of ['first add','replace','remove','remove tags']) {
  test(`failed ${scenario} reconciles cover summaries and invalidates hydrated art`,async()=>{
    const workspace=createWorkspace({TITLE:['Old']},{covers:scenario==='first add'?[]:[oldCover]});
    await workspace.hydrate();
    const removing=scenario==='remove'||scenario==='remove tags';
    await workspace.mutate(scenario==='remove tags'?'remove_tags':removing?'remove_cover':'add_cover',{index:0,imagePath:'content://image'});
    await workspace.hydrate();
    assert.deepEqual(workspace.snapshot().covers.map(cover=>cover.data_base64),removing?[]:['TkVX']);
    assert.equal(workspace.snapshot().coverIndex,0);
    await workspace.copy();
    assert.deepEqual(workspace.snapshot().copiedCovers.map(cover=>cover.data_base64),removing?[]:['TkVX']);
  });
}
test('unreadable retained metadata is explicit in source, artwork and copy presentation without hiding publication error',async()=>{
  const workspace=createWorkspace({TITLE:['Old']},{covers:[oldCover],workingReadFails:true});
  await workspace.hydrate();await workspace.mutate('remove_tags');
  assert.match(workspace.sourceMarkup(),/retained.*unavailable/i);
  assert.match(workspace.coverMarkup(),/artwork unavailable/i);
  assert.deepEqual(workspace.snapshot().covers,[]);
  assert.match(workspace.snapshot().errors[0].error,/publication failed/);
  await workspace.copy();assert.match(workspace.snapshot().dialogContent,/retained.*unavailable/i);
});
test('a rejected immediate command also reconciles local bytes without provider reload',async()=>{
  const workspace=createWorkspace({TITLE:['Old'],ARTIST:['Artist']},{mutationThrows:true});
  await workspace.mutate('remove_tags');
  assert.deepEqual(workspace.snapshot().fields,{});
  assert.match(workspace.snapshot().errors[0].error,/publication failed/);
  assert.equal(workspace.snapshot().nativeCalls.includes('scan_files'),false);
});

test('failed removal clamps the selected artwork index to the remaining cover',async()=>{
  const workspace=createWorkspace({TITLE:['Old']},{covers:[oldCover,{...oldCover,index:1,data_base64:'U0VDT05E'}]});
  await workspace.hydrate();workspace.selectCover(1);await workspace.mutate('remove_cover',{index:1});await workspace.hydrate();
  assert.equal(workspace.snapshot().coverIndex,0);
  assert.deepEqual(workspace.snapshot().covers.map(cover=>cover.data_base64),['T0xE']);
});

test('reselecting a recovery backup does not clear a known unavailable edited-metadata notice',async()=>{
  const workspace=createWorkspace({TITLE:['Old']},{workingReadFails:true});
  await workspace.mutate('remove_tags');
  workspace.acceptImported({TITLE:['Old']});
  assert.match(workspace.sourceMarkup(),/retained.*unavailable/i);
  workspace.acceptLoaded({TITLE:['Verified source']});
  assert.equal(workspace.snapshot().workingReadErrors.length,0);
});

const pendingCover={digest:'pasted-cover',picture_type:'CoverBack',description:'Pending back cover'};
const clipboardSource=(covers,fields={TITLE:['Pending']})=>({fields,covers});
async function pasteCopiedTags(workspace){
  await workspace.operate('copy-tags','clipboard.wav');
  await workspace.operate('paste-tags');
}
for(const covers of [[pendingCover],[]]) {
  test(`Save a copy sends pasted ${covers.length?'replacement artwork':'cover removal'} with fields in one request`,async()=>{
    const workspace=createWorkspace({TITLE:['Old']},{covers:[oldCover],clipboardSource:clipboardSource(covers)});
    await pasteCopiedTags(workspace);
    const before=workspace.snapshot();await workspace.copy();const after=workspace.snapshot();
    assert.deepEqual(after.copyRequestSent,{path:'retained.wav',version:'edited',updates:{TITLE:['Pending']},covers});
    assert.deepEqual(after.copiedCovers,covers);
    assert.deepEqual(after.pendingCovers,before.pendingCovers);
    assert.deepEqual(after.drafts,before.drafts);
    assert.equal(after.nativeCalls.filter(command=>command==='document_save_copy').length,1);
    assert.equal(after.nativeCalls.includes('save_fields_batch'),false);
  });
  test(`original-backup copy ignores pasted ${covers.length?'artwork':'cover removal'}`,async()=>{
    const workspace=createWorkspace({TITLE:['Old']},{covers:[oldCover],clipboardSource:clipboardSource(covers)});
    await pasteCopiedTags(workspace);await workspace.copy('original');
    assert.deepEqual(workspace.snapshot().copyRequestSent,{path:'retained.wav',version:'original',updates:{}});
    assert.deepEqual(workspace.snapshot().copiedCovers,[oldCover]);
  });
}
for(const outcome of ['copyCancels','copyFails','copyThrows']) {
  test(`${outcome} retains pasted artwork and fields for another copy attempt`,async()=>{
    const workspace=createWorkspace({TITLE:['Old']},{covers:[oldCover],clipboardSource:clipboardSource([pendingCover]),[outcome]:true});
    await pasteCopiedTags(workspace);
    const before=workspace.snapshot();await workspace.copy();const after=workspace.snapshot();
    assert.deepEqual(after.pendingCovers,before.pendingCovers);
    assert.deepEqual(after.drafts,before.drafts);
    assert.equal(after.busy,false);
    assert.match(after.status,/cancelled|failed|retained/i);
  });
}
test('artwork-only Paste reaches Save a copy without manufacturing field updates',async()=>{
  const workspace=createWorkspace({TITLE:['Original']},{covers:[oldCover],clipboardSource:clipboardSource([pendingCover],{TITLE:['Original']})});
  await pasteCopiedTags(workspace);await workspace.copy();
  const state=workspace.snapshot();
  assert.deepEqual(state.copyRequestSent,{path:'retained.wav',version:'edited',updates:{},covers:[pendingCover]});
  assert.deepEqual(state.copiedCovers,[pendingCover]);
  assert.deepEqual(state.pendingCovers,[['retained.wav',[pendingCover]]]);
});

for(const [sourceState,sourceOptions,reason] of [
  ['read-only',{writable:false},/read-only/],
  ['recovery-required',{state:'recovery_required'},/Recovery required/],
]) {
  for(const operation of ['Paste','Cut']) {
    test(`${sourceState}: real ${operation} stages artwork for edited copy while source writes stay blocked`,async()=>{
      const workspace=createWorkspace({TITLE:['Original']},{...sourceOptions,covers:[oldCover],clipboardSource:clipboardSource([pendingCover])});
      assert.match(workspace.policy().mutationReason,reason);
      assert.equal(workspace.available(operation==='Paste'?'copy-tags':'cut-tags'),true);
      if(operation==='Paste')await pasteCopiedTags(workspace);else await workspace.operate('cut-tags');
      const covers=operation==='Paste'?[pendingCover]:[],updates={TITLE:operation==='Paste'?['Pending']:[]};
      const staged=workspace.snapshot();
      assert.deepEqual(staged.pendingCovers,[['retained.wav',covers]]);
      assert.match(staged.status,/Use Save a copy/);
      for(const blocked of ['save','add-cover','remove-cover','cover-manage','remove-tags']){
        assert.equal(workspace.available(blocked),false,blocked);
        await workspace.operate(blocked);
      }
      assert.equal(workspace.available('save-copy'),true);
      await workspace.copy();
      const copied=workspace.snapshot();
      assert.deepEqual(copied.copyRequestSent,{path:'retained.wav',version:'edited',updates,covers});
      assert.deepEqual(copied.copiedCovers,covers);
      assert.deepEqual(copied.pendingCovers,staged.pendingCovers);
      assert.deepEqual(copied.drafts,staged.drafts);
      assert.equal(copied.nativeCalls.some(command=>['save_fields_batch','add_cover','remove_cover','remove_tags'].includes(command)),false);
      await workspace.copy('original');
      assert.deepEqual(workspace.snapshot().copyRequestSent,{path:'retained.wav',version:'original',updates:{}});
      assert.deepEqual(workspace.snapshot().copiedCovers,[oldCover]);
      assert.deepEqual(workspace.snapshot().pendingCovers,staged.pendingCovers);
    });
    for(const outcome of ['copyCancels','copyFails','copyThrows']) {
      test(`${sourceState}: ${outcome} retains real ${operation} artwork intent`,async()=>{
        const workspace=createWorkspace({TITLE:['Original']},{...sourceOptions,covers:[oldCover],clipboardSource:clipboardSource([pendingCover]),[outcome]:true});
        if(operation==='Paste')await pasteCopiedTags(workspace);else await workspace.operate('cut-tags');
        const before=workspace.snapshot();
        assert.deepEqual(before.pendingCovers,[['retained.wav',operation==='Paste'?[pendingCover]:[]]]);
        await workspace.copy();const after=workspace.snapshot();
        assert.deepEqual(after.pendingCovers,before.pendingCovers);
        assert.deepEqual(after.drafts,before.drafts);
        assert.equal(after.busy,false);
        assert.match(after.status,/cancelled|failed|retained/i);
      });
    }
  }
}
for(const operation of ['Paste','Cut']) {
  test(`unreadable retained metadata prevents ${operation} from staging destructive artwork changes`,async()=>{
    const workspace=createWorkspace({TITLE:['Original']},{covers:[oldCover],workingReadFails:true,clipboardSource:clipboardSource([pendingCover])});
    await workspace.mutate('filename_to_tags');
    assert.equal(workspace.snapshot().workingReadErrors.length,1);
    if(operation==='Paste')await pasteCopiedTags(workspace);else await workspace.operate('cut-tags');
    assert.deepEqual(workspace.snapshot().pendingCovers,[]);
    await workspace.copy();
    assert.equal(Object.hasOwn(workspace.snapshot().copyRequestSent,'covers'),false);
  });
}
