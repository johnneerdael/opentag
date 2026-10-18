import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {build,normalizePath} from 'vite';
import {Window} from 'happy-dom';

const root=normalizePath(fileURLToPath(new URL('../',import.meta.url))),english=JSON.parse(await readFile(new URL('../src/i18n/catalogs/en.json',import.meta.url),'utf8'));
// Bundle the actual application and its renderers; only native APIs are seams.
const result=await build({root,configFile:false,logLevel:'silent',plugins:[{name:'startup-native-seam',enforce:'pre',resolveId(id){if(id.startsWith('@tauri-apps/'))return '\0startup:'+id;},load(id){
  if(!id.startsWith('\0startup:'))return;
  if(id.endsWith('/core'))return `import {defaultSettings} from ${JSON.stringify(root+'src/options-ui.ts')};import {defaultSettings as panelDefaults} from ${JSON.stringify(root+'src/tag-panel.ts')};import {defaultDirectoryState} from ${JSON.stringify(root+'src/file-ops.ts')};export const isTauri=()=>globalThis.__BOOT_HOST__.native;export const invoke=(command,args)=>globalThis.__BOOT_HOST__.invoke(command,args,{defaultSettings,panelDefaults,defaultDirectoryState});`;
  if(id.endsWith('/window'))return 'export const getCurrentWindow=()=>globalThis.__BOOT_HOST__.nativeWindow;';
  if(id.endsWith('/webview'))return 'export const getCurrentWebview=()=>({onDragDropEvent:async callback=>{globalThis.__BOOT_HOST__.drag=callback;return()=>{};}});';
  if(id.endsWith('/app'))return 'export const getVersion=async()=>"0.2.0";';
  if(id.endsWith('/event'))return 'export const listen=async(name,callback)=>{globalThis.__BOOT_HOST__.events.set(name,callback);return()=>{};};';
  if(id.endsWith('/plugin-dialog'))return 'export const open=async options=>{globalThis.__BOOT_HOST__.picks.push(options);return null;};export const save=async()=>null;export const ask=async()=>false;export const message=async()=>{};export const confirm=async()=>false;';
  throw new Error('Unexpected native module: '+id);
}}],build:{write:false,minify:false,target:'es2022',lib:{entry:root+'src/main.ts',formats:['es']},rollupOptions:{output:{inlineDynamicImports:true}}}});
const code=(Array.isArray(result)?result.flatMap(build=>build.output):result.output).find(output=>output.type==='chunk').code,url='data:text/javascript;base64,'+Buffer.from(code+'\n//# sourceURL=taggroove-startup-fixture.mjs').toString('base64');
let serial=0;
const tick=()=>new Promise(resolve=>setTimeout(resolve,0));
async function until(predicate){for(let i=0;i<200;i++){if(predicate())return;await new Promise(resolve=>setTimeout(resolve,5));}assert.fail('Boot condition did not settle');}
function deferred(){let resolve,reject,done=false;const promise=new Promise((yes,no)=>{resolve=value=>{done=true;yes(value);};reject=error=>{done=true;no(error);};});return{promise,resolve,reject,get done(){return done;}};}
async function boot({native=true,remembered='fr',choice=remembered,preferred=['en-US'],updateCheck=false,platformFailure=false,subfolders=true,profile=null,initialLaunches=[],rejectExtraProfileReads=false}={}){
  const window=new Window({url:'http://localhost:1420'});
  for(const key of ['window','document','Node','NodeFilter','Text','Element','HTMLElement','HTMLInputElement','HTMLTextAreaElement','HTMLButtonElement','HTMLSelectElement','HTMLDetailsElement','HTMLTableRowElement','HTMLImageElement','MutationObserver','MouseEvent','KeyboardEvent','Event','CSS','localStorage','navigator','getComputedStyle','requestAnimationFrame','cancelAnimationFrame'])Object.defineProperty(globalThis,key,{value:key==='window'?window:typeof window[key]==='function'&&['getComputedStyle','requestAnimationFrame','cancelAnimationFrame'].includes(key)?window[key].bind(window):window[key],configurable:true,writable:true});
  Object.defineProperty(window.navigator,'languages',{value:preferred,configurable:true});Object.defineProperty(window.navigator,'language',{value:preferred[0],configurable:true});
  window.document.documentElement.lang='en';window.document.body.innerHTML='<div id="app"></div>';window.localStorage.setItem('opentag.language',remembered);window.localStorage.setItem('opentag.last-version','0.2.0');
  const settings=deferred(),catalogs=deferred(),directories=deferred(),calls=[],events=new Map(),titles=[],paints=[];
  const recovery=profile?{id:'profile-receipt',attempted:false,window:{version:1,preferences:{'opentag.language':remembered},workspace:{paths:['/A.flac'],selected:['/A.flac'],roots:['/'],location:'/',location_is_data:true,filter:'',filter_draft:null,anchor:'/A.flac',playlist:null}}}:null;let pending=recovery,profileReads=0;
  const host={native,calls,events,picks:[],profileCalls:[],get pending(){return pending;},drag:undefined,settings,catalogs,directories,defaults:undefined,scan:[],scanRecords:[],scanErrors:[],launches:structuredClone(initialLaunches),nativeWindow:{setTitle:async title=>titles.push(title),setTheme:async()=>{},onCloseRequested:async()=>()=>{},isMaximized:async()=>false,isMinimized:async()=>false},invoke:async(command,args,factories)=>{
    calls.push(command);host.defaults=factories;
    if(command==='profile_migration_status'){
      host.profileCalls.push(args);profileReads++;
      if(rejectExtraProfileReads&&profileReads>1&&!args?.start&&!args?.ack)throw new Error('second status read refused');
      if(args?.start){if(profile?.failure==='claim')throw new Error('controlled claim refusal');assert.equal(args.start,pending.id);pending.attempted=true;}
      if(args?.ack){assert.equal(args.ack,pending.id);if(profile?.failure==='ack')throw new Error('controlled acknowledgement refusal');pending=null;}
      return{desktop:true,phase:profile?'applied':'none',report:null,restorable:!!profile,pending:structuredClone(pending),preference_keys:profile?['opentag.language']:[]};
    }
    if(command==='load_settings')return settings.promise;
    if(command==='language_catalogs')return catalogs.promise;
    if(command==='runtime_platform'){if(platformFailure)throw new Error('runtime unavailable');return{family:'desktop',folder_picker_supported:true,document_access_supported:false};}
    if(command==='load_action_library')return{version:1,groups:[]};
    if(command==='load_columns')return null;
    if(command==='load_tag_panel')return factories.panelDefaults();
    if(command==='load_helper_lists')return{exists:false,lists:{}};
    if(command==='web_sources_list')return{folder:'',sources:[],settings:[],problems:[]};
    if(command==='load_directory_state')return directories.promise;
    if(command==='take_launch_requests')return host.launches.splice(0);
    if(command==='scan_files'){
      host.scan.push(args.paths);host.scanRecords.push(args);
      if(profile&&args.paths.includes('/A.flac')&&profile.failure==='scan')return{entries:[],errors:[{path:'/A.flac',error:'controlled missing recovery file'}]};
      return{entries:profile?args.paths.map(path=>{path=path.endsWith('.flac')?path:path+'/track.flac';return{path,filename:path.split('/').at(-1),format:'FLAC',duration:1,bitrate:128,sample_rate:44100,channels:2,size:4096,fields:{TITLE:[path]},covers:[]};}):[],errors:host.scanErrors};
    }
    if(command==='evaluate_columns'&&profile)return{values:args.items.map(()=>args.templates.map(()=>'')),errors:[]};
    if(command==='check_for_updates')return{current:'0.2.0',latest:{version:'0.2.1',name:'TagGroove',url:'https://example.invalid/release',prerelease:false,published:null},newer:true};
    if(command==='history_state')return{undo:null,redo:null,notice:null};
    if(command==='notification_area_supported')return false;
    if(command==='set_window_title')return;
    if(command==='save_directory_state')return;
    throw new Error('Unexpected native invocation: '+command);
  }};
  globalThis.__BOOT_HOST__=host;
  const app=window.document.querySelector('#app'),visible=()=>window.getComputedStyle(app).visibility!=='hidden'&&window.getComputedStyle(app).display!=='none';
  let frame=0;
  const observer=new window.MutationObserver(()=>{if(frame||paints.length)return;frame=window.requestAnimationFrame(()=>{frame=0;if(visible()&&app.querySelector('[data-menu="File"]'))paints.push({file:app.querySelector('[data-menu="File"] > summary').textContent,language:window.document.documentElement.lang});});});observer.observe(window.document.body,{subtree:true,childList:true,characterData:true,attributes:true});
  await import(url+'#boot-'+serial++);await tick();
  const response=()=>{const value=host.defaults.defaultSettings();value.pages.language={language:choice};value.directories.subfolders=subfolders;value.pages['directories-startup']={startup:'favorite'};value.pages.updates={check_on_startup:updateCheck,release_notes:false};return{settings:value,defaults:host.defaults.defaultSettings(),warning:null};};
  return{window,host,app,visible,titles,paints,response,async close(){if(!settings.done)settings.resolve(response());if(!catalogs.done)catalogs.resolve({catalogs:[],errors:[]});if(!directories.done)directories.resolve(host.defaults.defaultDirectoryState());for(let i=0;i<8;i++)await tick();observer.disconnect();if(frame)window.cancelAnimationFrame(frame);await window.happyDOM.abort();}};
}

// Visibility/DOM ordering evidence in Happy DOM, not native pixel qualification.
test('actual native boot resolves custom catalogs before its first visible complete interface',async()=>{
  const cases=[['fr','fr','fr','Fichier'],['en','en','en','Custom File'],['de','de','de','Eigene Datei'],['nl','nl','nl','Eigen bestand'],['system','system','pt-BR','Arquivo'],['nl','fr','fr','Fichier'],['fr','fr',null,'File'],['fr','fr','failure','File'],['fr','settings-failure','fr','Fichier']];
  for(const [remembered,saved,locale,label] of cases){
    const fixture=await boot({remembered,choice:saved==='settings-failure'?'fr':saved,preferred:remembered==='system'?['pt-BR','en-US']:['en-US']});
    try{
      assert.ok(fixture.app.querySelector('[data-menu="File"]'),'actual main renderer initialized its DOM');assert.equal(fixture.visible(),false);assert.equal(fixture.app.inert,true);assert.deepEqual(fixture.paints,[]);assert.deepEqual(fixture.titles,[]);
      const key=new fixture.window.KeyboardEvent('keydown',{key:'a',code:'KeyA',altKey:true,bubbles:true,cancelable:true});fixture.window.document.dispatchEvent(key);assert.equal(key.defaultPrevented,true);assert.equal(fixture.app.querySelector('#actions-menu').open,false);
      if(saved==='settings-failure')fixture.host.settings.reject(new Error('settings unavailable'));else fixture.host.settings.resolve(fixture.response());
      await until(()=>fixture.host.calls.includes('language_catalogs'));assert.equal(fixture.visible(),false);assert.deepEqual(fixture.paints,[]);
      const catalog=locale&&locale!=='failure'?{...english,'@meta':{name:locale,locale},File:label}:undefined;
      if(locale==='failure')fixture.host.catalogs.reject(new Error('catalog read denied'));else fixture.host.catalogs.resolve({catalogs:catalog?[{file:locale+'.json',catalog}]:[],errors:[]});
      await until(()=>fixture.visible()&&fixture.paints.length>0);assert.equal(fixture.app.inert,false);
      assert.equal(fixture.app.querySelector('[data-menu="File"] > summary').textContent,label);assert.ok(fixture.paints.every(paint=>paint.file===label),JSON.stringify(fixture.paints));assert.equal(fixture.window.document.documentElement.lang,catalog?locale:'en');assert.ok(fixture.titles.length>0);
    }finally{await fixture.close();}
  }
});

test('actual boot defers outside-root drops and preserves initial launch draining before startup-folder fallback',async()=>{
  const fixture=await boot({updateCheck:true});
  try{
    fixture.host.launches.push({directories:['/launch'],files:[],add:false});fixture.host.events.get('opentag://launch')({payload:{}});
    const drop=fixture.host.drag({payload:{type:'enter'}});await tick();assert.equal(fixture.window.document.querySelector('.drop-overlay'),null);
    fixture.host.settings.resolve(fixture.response());await until(()=>fixture.host.calls.includes('language_catalogs'));assert.equal(fixture.host.calls.includes('check_for_updates'),false);assert.equal(fixture.window.document.querySelector('dialog'),null);
    fixture.host.catalogs.resolve({catalogs:[{file:'fr.json',catalog:{...english,'@meta':{name:'Français',locale:'fr'},File:'Fichier','An update is available':'Mise à jour disponible'}}],errors:[]});
    await until(()=>fixture.visible());await until(()=>fixture.window.document.querySelector('.updates-dialog header')?.textContent==='Mise à jour disponible');assert.equal(fixture.host.calls.includes('take_launch_requests'),false);
    fixture.host.directories.resolve({version:1,recent:[],favorites:['/fallback']});await until(()=>fixture.host.scan.length>0);await drop;
    assert.deepEqual(fixture.host.scan,[['/launch']]);assert.ok(fixture.window.document.querySelector('.drop-overlay'));
    await fixture.host.drag({payload:{type:'leave'}});assert.equal(fixture.window.document.querySelector('.drop-overlay'),null);
  }finally{await fixture.close();}
});

test('rejected platform startup still holds early launches for the saved language and scan preferences',async()=>{
  const fixture=await boot({platformFailure:true,subfolders:false});
  try{
    fixture.host.launches.push({directories:['/launch'],files:[],add:false});fixture.host.events.get('opentag://launch')({payload:{}});await tick();
    assert.equal(fixture.host.calls.includes('take_launch_requests'),false);assert.equal(fixture.visible(),false);
    fixture.host.settings.resolve(fixture.response());await until(()=>fixture.host.calls.includes('language_catalogs'));assert.equal(fixture.host.calls.includes('take_launch_requests'),false);
    fixture.host.catalogs.resolve({catalogs:[{file:'fr.json',catalog:{...english,'@meta':{name:'Français',locale:'fr'},File:'Fichier'}}],errors:[]});
    await until(()=>fixture.visible()&&fixture.host.scan.length>0&&fixture.paints.length>0);assert.deepEqual(fixture.host.scan,[['/launch']]);assert.equal(fixture.host.scanRecords[0].recursive,false);assert.ok(fixture.paints.length>0);assert.ok(fixture.paints.every(paint=>paint.file==='Fichier'));
  }finally{await fixture.close();}
});

test('actual startup errors translate their owned context while same-spelling native paths remain raw',async()=>{
  const labels=['Options','Action groups','Columns','Tag panel','Directories','Platform','Command line','Window','Drag and drop'];
  for(const locale of ['de','nl']){
    const fixture=await boot({remembered:locale,choice:locale});
    try{
      fixture.host.settings.reject(new Error('settings unavailable'));await until(()=>fixture.host.calls.includes('language_catalogs'));fixture.host.catalogs.resolve({catalogs:[],errors:[]});
      await until(()=>fixture.visible()&&fixture.paints.length>0);
      assert.equal(fixture.app.querySelector('.errors strong').textContent,locale==='de'?'Optionen:':'Opties:');
      fixture.host.scanErrors=labels.map(label=>({path:'/missing/'+label,error:'opaque native failure'}));fixture.host.launches.push({directories:['/launch'],files:[],add:false});fixture.host.events.get('opentag://launch')({payload:{}});
      fixture.host.directories.resolve(fixture.host.defaults.defaultDirectoryState());await until(()=>fixture.app.querySelectorAll('.errors strong').length===labels.length);
      assert.deepEqual([...fixture.app.querySelectorAll('.errors strong')].map(node=>node.textContent),labels.map(label=>label+':'));
    }finally{await fixture.close();}
  }
});

test('two real Options saves keep the latest language and preferences across older catalog success or failure',async()=>{
  for(const outcome of ['older-success','older-failure','latest-failure']){
    const fixture=await boot({remembered:'en',choice:'en'}),reads=[],collators=[],NativeCollator=Intl.Collator;
    try{
      fixture.host.settings.resolve(fixture.response());await until(()=>fixture.host.calls.includes('language_catalogs'));
      fixture.host.catalogs.resolve({catalogs:[{file:'nl.json',catalog:{...english,'@meta':{name:'Retained Dutch',locale:'nl'},File:'Bestaand bestand'}},{file:'pt.json',catalog:{...english,'@meta':{name:'Retained choice',locale:'pt'}}}],errors:[]});
      fixture.host.directories.resolve(fixture.host.defaults.defaultDirectoryState());await until(()=>fixture.visible());
      // Keep the real Intl implementation while recording actual optionsApplied continuations.
      Intl.Collator=new Proxy(NativeCollator,{construct(target,args){collators.push(args[1]);return Reflect.construct(target,args);}});
      let saved=structuredClone(fixture.response().settings);const saves=[],invoke=fixture.host.invoke;
      fixture.host.invoke=async(command,args,factories)=>{
        if(command==='load_settings')return{settings:structuredClone(saved),defaults:factories.defaultSettings(),warning:null};
        if(command==='save_settings'){saved=structuredClone(args.settings);saves.push(saved.pages.language.language);return{settings:structuredClone(saved),defaults:factories.defaultSettings(),warning:null};}
        if(command==='language_catalogs'){const read=deferred();reads.push(read);return read.promise;}
        if(command==='set_tray_options')return{supported:false,active:false};
        return invoke(command,args,factories);
      };
      const open=async()=>{fixture.app.querySelector('[data-op="options"]').click();await until(()=>fixture.window.document.querySelector('.options-dialog'));const dialog=fixture.window.document.querySelector('.options-dialog');dialog.querySelector('[data-page="language"]').click();return dialog;};
      const save=async(language)=>{const dialog=await open();dialog.querySelector('[data-page="general"]').click();const natural=dialog.querySelector('[data-option="general.natural_sort"]');natural.checked=language==='de';natural.dispatchEvent(new fixture.window.Event('change',{bubbles:true}));dialog.querySelector('[data-page="language"]').click();const select=dialog.querySelector('select[data-option="pages.language.language"]');select.value=language;select.dispatchEvent(new fixture.window.Event('change',{bubbles:true}));dialog.querySelector('#options-ok').click();await until(()=>!dialog.isConnected&&reads.length===saves.length);};
      await save('de');assert.equal(reads.length,1);await save('nl');assert.equal(reads.length,2);assert.deepEqual(saves,['de','nl']);
      if(outcome==='latest-failure')reads[1].reject(new Error('Newest catalog failure'));
      else reads[1].resolve({catalogs:[{file:'nl.json',catalog:{...english,'@meta':{name:'Newest Dutch',locale:'nl'},File:'Nieuw bestand'}},{file:'pt.json',catalog:{...english,'@meta':{name:'Newest choice',locale:'pt'}}}],errors:[{file:'Newest warning.json',error:'not a JSON object',kind:'not_object'}]});
      const expectedFile=outcome==='latest-failure'?'Bestaand bestand':'Nieuw bestand',expectedWarning=outcome==='latest-failure'?'Newest catalog failure':'Newest warning.json';
      await until(()=>fixture.window.document.documentElement.lang==='nl'&&collators.some(options=>options?.numeric===false));assert.equal(fixture.app.querySelector('[data-menu="File"] > summary').textContent,expectedFile);
      const dialog=await open();assert.ok(dialog.textContent.includes(expectedWarning));assert.ok(dialog.querySelector('option[value="pt"]'));const applied=collators.length;
      if(outcome==='older-failure')reads[0].reject(new Error('Older catalog failure'));
      else reads[0].resolve({catalogs:[{file:'de.json',catalog:{...english,'@meta':{name:'Older German',locale:'de'},File:'Alte Datei'}}],errors:[{file:'Older warning.json',error:'not a JSON object',kind:'not_object'}]});
      for(let i=0;i<8;i++)await tick();
      assert.equal(saved.pages.language.language,'nl');assert.equal(saved.general.natural_sort,false);assert.equal(fixture.window.document.documentElement.lang,'nl');assert.equal(fixture.window.localStorage.getItem('opentag.language'),'nl');assert.equal(fixture.app.querySelector('[data-menu="File"] > summary').textContent,expectedFile);assert.equal(collators.length,applied,'older optionsApplied must not overwrite the latest collator');assert.ok(dialog.textContent.includes(expectedWarning));assert.ok(!dialog.textContent.includes('Older'));
      dialog.querySelector('#options-cancel').click();const reopened=await open();assert.ok(reopened.querySelector('option[value="pt"]'));reopened.querySelector('#options-cancel').click();
    }finally{Intl.Collator=NativeCollator;for(const read of reads)if(!read.done)read.resolve({catalogs:[],errors:[]});await fixture.close();}
  }
});

test('overlapping real Options saves still apply tag preferences saved before the latest language choice',async()=>{
  for(const boundary of ['clean','dirty','provider']){
  const fixture=await boot({remembered:'en',choice:'en'}),reads=[];
  try{
    fixture.host.settings.resolve(fixture.response());await until(()=>fixture.host.calls.includes('language_catalogs'));fixture.host.catalogs.resolve({catalogs:[],errors:[]});fixture.host.directories.resolve(fixture.host.defaults.defaultDirectoryState());await until(()=>fixture.visible());
    let saved=structuredClone(fixture.response().settings);const scans=[],invoke=fixture.host.invoke;
    fixture.host.invoke=async(command,args,factories)=>{
      if(command==='load_settings')return{settings:structuredClone(saved),defaults:factories.defaultSettings(),warning:null};
      if(command==='save_settings'){saved=structuredClone(args.settings);return{settings:structuredClone(saved),defaults:factories.defaultSettings(),warning:null};}
      if(command==='language_catalogs'){const read=deferred();reads.push(read);return read.promise;}
      if(command==='scan_files'){scans.push({args,readId3v2:saved.tags.mpeg.read_id3v2});return{entries:[{path:'/Help.mp3',filename:'Help.mp3',format:'MP3',duration:1,bitrate:128,sample_rate:44100,channels:2,size:4096,fields:{TITLE:[saved.tags.mpeg.read_id3v2?'Before preferences':'After preferences']},technical:boundary==='provider'?{_PATH:['content://media/Help']}:undefined,covers:[]}],errors:[]};}
      if(command==='evaluate_columns')return{values:args.items.map(()=>args.templates.map(()=>'')),errors:[]};
      if(command==='set_tray_options')return{supported:false,active:false};
      return invoke(command,args,factories);
    };
    fixture.host.launches.push({directories:[],files:['/Help.mp3'],add:false});fixture.host.events.get('opentag://launch')({payload:{}});await until(()=>fixture.app.querySelector('tr[data-path="/Help.mp3"]')&&!fixture.app.querySelector('[data-op="options"]').disabled);assert.equal(scans.length,1);
    if(boundary==='dirty'){const title=fixture.app.querySelector('input[data-field="TITLE"]');assert.equal(title.disabled,false);title.value='Pending  Help';title.dispatchEvent(new fixture.window.Event('input',{bubbles:true}));assert.ok(fixture.app.querySelector('tr[data-path="/Help.mp3"] .rowmarker'));}
    const open=async()=>{fixture.app.querySelector('[data-op="options"]').click();await until(()=>fixture.window.document.querySelector('.options-dialog'));return fixture.window.document.querySelector('.options-dialog');};
    const first=await open();first.querySelector('[data-page="tags-mpeg"]').click();const read=first.querySelector('[data-option="tags.mpeg.read_id3v2"]');read.checked=false;read.dispatchEvent(new fixture.window.Event('change',{bubbles:true}));first.querySelector('[data-page="language"]').click();let language=first.querySelector('[data-option="pages.language.language"]');language.value='de';language.dispatchEvent(new fixture.window.Event('change',{bubbles:true}));first.querySelector('#options-ok').click();await until(()=>!first.isConnected&&reads.length===1);
    const second=await open();second.querySelector('[data-page="language"]').click();language=second.querySelector('[data-option="pages.language.language"]');language.value='nl';language.dispatchEvent(new fixture.window.Event('change',{bubbles:true}));second.querySelector('#options-ok').click();await until(()=>!second.isConnected&&reads.length===2);
    reads[1].resolve({catalogs:[],errors:[]});for(let i=0;i<10;i++)await tick();reads[0].resolve({catalogs:[],errors:[]});for(let i=0;i<10;i++)await tick();
    assert.equal(saved.tags.mpeg.read_id3v2,false);assert.equal(saved.pages.language.language,'nl');assert.equal(fixture.window.document.documentElement.lang,'nl');assert.equal(scans.length,boundary==='clean'?2:1,'only clean local files reload for the cumulative tag-read preference change');if(boundary==='clean'){assert.equal(scans[1].readId3v2,false);assert.deepEqual(scans[1].args.paths,['/Help.mp3']);assert.equal(scans[1].args.recursive,false);}assert.equal(fixture.app.querySelector('input[data-field="TITLE"]').value,boundary==='clean'?'After preferences':boundary==='dirty'?'Pending  Help':'Before preferences');assert.ok(!fixture.host.calls.some(command=>command==='write_fields'||command==='write_tags'));
  }finally{for(const read of reads)if(!read.done)read.resolve({catalogs:[],errors:[]});await fixture.close();}
  }
});

test('browser preview keeps its immediate first render without native initialization',async()=>{
  const fixture=await boot({native:false,remembered:'en'});
  try{assert.equal(fixture.visible(),true);assert.equal(fixture.app.querySelector('[data-menu="File"] > summary').textContent,'File');assert.deepEqual(fixture.host.calls.filter(command=>['load_settings','language_catalogs','runtime_platform'].includes(command)),[]);}finally{await fixture.close();}
});

async function finishProfileBoot(fixture){
  fixture.host.settings.resolve(fixture.response());await until(()=>fixture.host.calls.includes('language_catalogs'));
  fixture.host.catalogs.resolve({catalogs:[],errors:[]});fixture.host.directories.resolve({version:1,recent:[],favorites:['/fallback']});
}
test('verified no-journal bootstrap initializes normally without a redundant fallible second status read',async()=>{
  const fixture=await boot({remembered:'en',choice:'en',subfolders:false,rejectExtraProfileReads:true});
  try{
    await finishProfileBoot(fixture);await until(()=>fixture.visible()&&fixture.host.scan.length>0);
    assert.equal(fixture.host.profileCalls.length,1);assert.equal(fixture.app.inert,false);assert.ok(fixture.app.querySelector('[data-op="options"]'));
    assert.equal(fixture.host.scanRecords[0].recursive,false,'saved preferences initialized before file opening');
  }finally{await fixture.close();}
});
test('pending workspace A restores before an explicit queued B launch, preserving replace and add intent',async()=>{
  for(const add of [false,true]){
    const fixture=await boot({remembered:'en',choice:'en',profile:{},initialLaunches:[{directories:[],files:['/B.flac'],add}]});
    try{
      assert.deepEqual(fixture.host.scan,[]);await finishProfileBoot(fixture);await until(()=>!fixture.app.inert&&fixture.app.querySelector('tr[data-path="/B.flac"]'));
      assert.deepEqual(fixture.host.scan,[['/A.flac'],['/B.flac']]);assert.equal(!!fixture.app.querySelector('tr[data-path="/A.flac"]'),add);assert.equal(fixture.host.pending,null);
      assert.ok(fixture.host.calls.indexOf('take_launch_requests')>fixture.host.calls.indexOf('scan_files'));
    }finally{await fixture.close();}
  }
});
test('successful workspace recovery suppresses only favorite fallback when no explicit launch is queued',async()=>{
  const fixture=await boot({remembered:'en',choice:'en',profile:{}});
  try{
    await finishProfileBoot(fixture);await until(()=>!fixture.app.inert&&fixture.host.calls.includes('take_launch_requests'));
    assert.deepEqual(fixture.host.scan,[['/A.flac']]);assert.ok(fixture.app.querySelector('tr[data-path="/A.flac"]'));assert.equal(fixture.host.pending,null);
  }finally{await fixture.close();}
});
test('failed workspace reads retain the attempted receipt and still admit explicit launches or the existing favorite fallback',async()=>{
  for(const explicit of [false,true]){
    const fixture=await boot({remembered:'en',choice:'en',profile:{failure:'scan'},initialLaunches:explicit?[{directories:[],files:['/B.flac'],add:false}]:[]});
    try{
      await finishProfileBoot(fixture);const path=explicit?'/B.flac':'/fallback/track.flac';await until(()=>!fixture.app.inert&&fixture.app.querySelector(`tr[data-path="${path}"]`));
      assert.deepEqual(fixture.host.scan,[['/A.flac'],[explicit?'/B.flac':'/fallback']]);assert.equal(fixture.host.pending.attempted,true);assert.ok(!fixture.host.profileCalls.some(args=>args?.ack));
      assert.match(fixture.app.querySelector('.errors').textContent,/Open Restore setup to retry/);
    }finally{await fixture.close();}
  }
});
test('an ambiguous workspace claim failure exposes a blocking retry and never drains newer explicit launches',async()=>{
  const fixture=await boot({remembered:'en',choice:'en',profile:{failure:'claim'},initialLaunches:[{directories:[],files:['/B.flac'],add:false}]});
  try{
    await finishProfileBoot(fixture);await until(()=>fixture.app.querySelector('.profile-recovery-dialog[open]'));
    assert.match(fixture.app.textContent,/controlled claim refusal/);assert.equal(fixture.app.querySelector('button').textContent,'Retry startup recovery');assert.deepEqual(fixture.host.scan,[]);assert.ok(!fixture.host.calls.includes('take_launch_requests'));assert.ok(fixture.host.pending);
    fixture.window.document.dispatchEvent(new fixture.window.KeyboardEvent('keydown',{key:'d',ctrlKey:true,bubbles:true,cancelable:true}));await tick();assert.deepEqual(fixture.host.picks,[],'modal recovery prevents background file-picker shortcuts');
  }finally{await fixture.close();}
});
test('acknowledgement failure retains an actionable receipt without replacing the already restored workspace with a favorite',async()=>{
  const fixture=await boot({remembered:'en',choice:'en',profile:{failure:'ack'}});
  try{
    await finishProfileBoot(fixture);await until(()=>!fixture.app.inert&&fixture.app.querySelector('tr[data-path="/A.flac"]'));
    assert.deepEqual(fixture.host.scan,[['/A.flac']]);assert.equal(fixture.host.pending.attempted,true);assert.match(fixture.app.querySelector('.errors').textContent,/Open Restore setup to retry/);
  }finally{await fixture.close();}
});
