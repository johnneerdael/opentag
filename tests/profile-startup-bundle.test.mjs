import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,readFileSync,readdirSync,rmSync,writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';

const root=fileURLToPath(new URL('../',import.meta.url));
const output=mkdtempSync(join(tmpdir(),'taggroove-profile-bundle-'));
// Use the real production Vite target and complete main bundle, including its
// top-level await. Only the native boundary is controlled; no user files open.
execFileSync(process.execPath,[join(root,'node_modules/vite/bin/vite.js'),'build','--outDir',output],{cwd:root,stdio:'pipe',maxBuffer:2*1024*1024});
const asset=readdirSync(join(output,'assets')).find(name=>/^index-.*\.js$/.test(name));
assert.ok(asset,'production main JS exists');
const bundle=readFileSync(join(output,'assets',asset),'utf8');

for(const mode of ['held','denied','browser','android','ios','storage-none','storage-pending'])test(`production main ${mode}: setup recovery precedes configuration construction`,()=>{
  const program=`
import assert from 'node:assert/strict';
import {Window} from ${JSON.stringify(new URL('../node_modules/happy-dom/lib/index.js',import.meta.url).href)};
const window=new Window({url:'http://127.0.0.1/'});
window.document.body.innerHTML='<div id="app"></div>';
for(const key of ['window','document','navigator','localStorage','Node','NodeFilter','Element','HTMLElement','HTMLInputElement','HTMLTextAreaElement','HTMLButtonElement','HTMLSelectElement','HTMLDialogElement','MutationObserver','MouseEvent','Event','KeyboardEvent','ResizeObserver','CSS'])Object.defineProperty(globalThis,key,{value:key==='window'?window:window[key],configurable:true});
globalThis.requestAnimationFrame=window.requestAnimationFrame.bind(window);globalThis.cancelAnimationFrame=window.cancelAnimationFrame.bind(window);
globalThis.isTauri=${mode!=='browser'};
for(const key of Object.getOwnPropertyNames(window))if(/^(HTML|SVG)|Event$|Observer$/.test(key)&&typeof window[key]==='function')Object.defineProperty(globalThis,key,{value:window[key],configurable:true});
const calls=[];let release;
if(${mode.startsWith('storage-')}){Object.defineProperty(window,'localStorage',{get(){throw new Error('controlled storage getter refusal');},configurable:true});Object.defineProperty(globalThis,'localStorage',{get(){throw new Error('controlled storage getter refusal');},configurable:true});}
window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback:()=>1,unregisterCallback:()=>{},convertFileSrc:path=>path,invoke:(command,args)=>{
 calls.push(command);
 if(command==='runtime_platform')return Promise.resolve({family:${JSON.stringify(['android','ios'].includes(mode)?mode:'desktop')},folder_picker_supported:false,document_access_supported:${['android','ios'].includes(mode)}});
 if(command==='profile_migration_status'){
  if(${mode==='denied'})return Promise.reject(new Error('controlled recovery refusal'));
  if(${!['held','denied'].includes(mode)})return Promise.resolve({desktop:${!['android','ios'].includes(mode)},phase:${JSON.stringify(['android','ios'].includes(mode)?'unsupported':'none')},report:null,restorable:false,pending:${mode==='storage-pending'?`{id:'pending',attempted:false,window:{version:1,preferences:{'opentag.language':'nl'},workspace:{paths:[],selected:[],roots:[],location:'',location_is_data:false,filter:'',filter_draft:null,anchor:null,playlist:null}}}`:'null'},preference_keys:['opentag.language']});
  return new Promise(resolve=>{release=()=>resolve({desktop:true,phase:'none',report:null,restorable:false,pending:null,preference_keys:[]});});
 }
 if(command==='load_helper_lists')return Promise.resolve({exists:true,lists:{}});
 return Promise.reject(new Error('unavailable fixture boundary '+command));
}};
const unexpected=[];console.warn=(...values)=>unexpected.push(values.map(value=>value instanceof Error?value.message:String(value)).join(' '));console.error=console.warn;process.on('unhandledRejection',error=>unexpected.push(String(error)));process.on('uncaughtException',error=>{console.log(JSON.stringify({uncaught:error.message,calls,stack:String(error.stack).replace(/data:[^\\s)]+/g,'<bundle>')}));process.exit(1);});
void import(${JSON.stringify(`data:text/javascript;base64,${Buffer.from(bundle).toString('base64')}`)}).catch(error=>unexpected.push(String(error)));
const tick=()=>new Promise(resolve=>setTimeout(resolve,10));
try{
 for(let count=0;count<15;count++)await tick();
 if(${mode==='held'}){
  assert.deepEqual(calls,['profile_migration_status'],'no history/options service reads before recovery resolves');
  release();
  for(let count=0;count<15;count++)await tick();
  assert.ok(calls.indexOf('load_helper_lists')>calls.indexOf('profile_migration_status'),'history construction follows completed recovery');
 }else if(${mode==='denied'}){
  assert.deepEqual(calls,['profile_migration_status']);
  assert.match(window.document.querySelector('#app').textContent,/controlled recovery refusal/);
  assert.equal(window.document.querySelector('#app button').textContent,'Retry startup recovery');
  assert.equal(window.document.querySelector('#app').inert,false);
 }else if(${mode==='storage-none'}){
  assert.ok(window.document.querySelector('.app'),'no-journal native startup tolerates unavailable browser storage');
 }else if(${mode==='storage-pending'}){
  assert.ok(!calls.includes('load_helper_lists'),'pending recovery blocks configuration reads when browser storage fails');
  assert.match(window.document.querySelector('#app').textContent,/controlled storage getter refusal/);
  assert.ok(window.document.querySelector('#app button'),'pending recovery exposes retry');
 }else{
  if(${mode==='browser'})assert.ok(!calls.includes('profile_migration_status'),'browser demo has no native recovery');else assert.equal(calls.filter(command=>command==='profile_migration_status').length,1,'mobile reads only cfg-gated metadata before regular startup');
  assert.ok(window.document.querySelector('.app'),'browser/mobile startup still renders');
 }
 console.log(JSON.stringify({mode:${JSON.stringify(mode)},calls}));process.exit(0);
}catch(error){console.log(JSON.stringify({error:error.message,calls,unexpected}));process.exit(1);}
`;
  let result;
  const entry=join(output,`probe-${mode}.mjs`);writeFileSync(entry,program);
  try{result=execFileSync(process.execPath,[entry],{cwd:root,encoding:'utf8',timeout:10000,maxBuffer:2*1024*1024});}
  catch(error){assert.fail(String(error.stdout||error.stderr||error.message).slice(-8000));}
  assert.match(result,new RegExp(`"mode":"${mode}"`));
});
test.after(()=>rmSync(output,{recursive:true,force:true}));
