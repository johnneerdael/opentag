import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');
const parsed=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
const declaration=parsed.statements.filter(ts.isVariableStatement).flatMap(statement=>[...statement.declarationList.declarations]).find(item=>item.name.getText(parsed)==='profileMigrationUI');
const callback=declaration.initializer.arguments[0].properties.find(item=>item.name?.getText(parsed)==='restoreWorkspace').initializer;
const javascript=ts.transpileModule(`const restore=${callback.getText(parsed)};return{restore,state:()=>({paths:[...entries.keys()],selected:[...selected],roots:listRoots,location,location_is_data:locationIsData,filter,filter_draft:filterDraft??null,anchor:anchor??null,playlist:loadedPlaylist??null})};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
function host(batch){
  const entries=new Map([['/currently-open.flac',{path:'/currently-open.flac'}]]),selected=new Set(['/currently-open.flac']),calls=[];
  const invoke=async(command,args)=>{calls.push([command,args]);return batch;};
  const render=()=>{},changedData=()=>{},rowFile=path=>path;
  const clearWorkspace=()=>{entries.clear();selected.clear();};const accept=batch=>batch.entries.forEach(entry=>entries.set(entry.path,entry));
  const run=new Function('entries','selected','invoke','render','changedData','rowFile','clearWorkspace','accept',`let status='',listRoots=[],location='',locationIsData=false,filter='',filterDraft,anchor,loadedPlaylist;${javascript}`);
  return{...run(entries,selected,invoke,render,changedData,rowFile,clearWorkspace,accept),calls};
}
const workspace=paths=>({paths,selected:[paths.at(-1)],roots:['/Music'],location:'/Music',location_is_data:true,filter:'ARTIST HAS Practice',filter_draft:'draft filter',anchor:paths.at(-1),playlist:'/Music/practice.m3u8'});
test('production workspace host restores order, selection, roots, location, filter and playlist at the supported file-count boundary',async()=>{
  const paths=Array.from({length:10000},(_,index)=>`/Music/practice-${index}.flac`),batch={entries:[...paths].reverse().map(path=>({path})),errors:[]},fixture=host(batch),expected=workspace(paths);
  assert.equal(await fixture.restore(expected),true);assert.deepEqual(fixture.state(),expected);
  assert.deepEqual(fixture.calls,[['scan_files',{paths,recursive:false}]]);
});
test('production workspace host leaves the current list intact when the saved files cannot all be rescanned',async()=>{
  const fixture=host({entries:[{path:'/Music/available.flac'}],errors:[]}),expected=workspace(['/Music/available.flac','/Music/missing.flac']);
  await assert.rejects(fixture.restore(expected),/Some workspace files are unavailable/);assert.deepEqual(fixture.state().paths,['/currently-open.flac']);assert.deepEqual(fixture.state().selected,['/currently-open.flac']);
});
