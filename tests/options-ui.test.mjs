import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/options-ui.ts',import.meta.url),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const ui=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const fixture=JSON.parse(await readFile(new URL('./fixtures/default-settings.json',import.meta.url),'utf8'));
const draft=(change=()=>{})=>{const value={...ui.defaultSettings(),appearance:'system'};change(value);return value;};

test('frontend defaults match the backend defaults fixture',()=>{
  assert.deepEqual(ui.defaultSettings(),fixture);
});
test('built-in pages form the documented tree with Tags sub-pages',()=>{
  const tree=ui.pageTree().map(({page,depth})=>`${depth}:${page.id}`);
  assert.deepEqual(tree.slice(0,8),['0:general','0:files','0:directories','0:appearance','0:tags','1:tags-mpeg','1:tags-ape','1:tags-advanced']);
});
test('registered pages are ordered under their parent, orphans become top-level and ids are unique',()=>{
  const pages=[{id:'b',label:'B',order:2,controls:[]},{id:'a',label:'A',order:1,controls:[]},{id:'child',label:'C',parent:'b',controls:[]},{id:'orphan',label:'O',parent:'missing',order:3,controls:[]}];
  assert.deepEqual(ui.pageTree(pages).map(({page,depth})=>[page.id,depth]),[['a',0],['b',0],['child',1],['orphan',0]]);
  ui.registerOptionsPage({id:'tag-panel-test',label:'Tag Panel',controls:[{kind:'checkbox',path:'pages.tag-panel-test.enabled',label:'Enabled'}]});
  assert.ok(ui.pageTree().some(({page})=>page.id==='tag-panel-test'));
  assert.throws(()=>ui.registerOptionsPage({id:'tag-panel-test',label:'Again',controls:[]}),/already registered/);
  assert.throws(()=>ui.registerOptionsPage({id:'Bad Id',label:'Bad',controls:[]}),/Invalid/);
});
test('paths read and create nested values, including extension page storage',()=>{
  const value=draft();
  assert.equal(ui.getPath(value,'tags.mpeg.read_ape'),true);
  assert.equal(ui.getPath(value,'pages.missing.value'),undefined);
  ui.setPath(value,'pages.tag-panel.fields',['TITLE']);ui.setPath(value,'tags.mpeg.id3v2_format','v24_utf8');
  assert.deepEqual(value.pages,{'tag-panel':{fields:['TITLE']}});assert.equal(value.tags.mpeg.id3v2_format,'v24_utf8');
});
test('validation names the page that needs attention',()=>{
  assert.equal(ui.validateOptions(draft()),undefined);
  assert.deepEqual(ui.validateOptions(draft(d=>{d.tags.advanced.id3v2_comment_language='en';})).page,'tags-advanced');
  assert.equal(ui.validateOptions(draft(d=>{d.tags.mpeg.write_id3v1=d.tags.mpeg.write_id3v2=false;})).page,'tags-mpeg');
  assert.equal(ui.validateOptions(draft(d=>{d.tags.ape.write_ape=false;})).page,'tags-ape');
  assert.equal(ui.validateOptions(draft(d=>{d.files.restrict_extensions='*.mp3;../x';})).page,'files');
  assert.equal(ui.validateOptions(draft(d=>{d.files.folder_separator='/';})).page,'files');
  assert.equal(ui.validateOptions(draft(d=>{d.files.folder_separator='';})),undefined);
  assert.equal(ui.validateOptions(draft(d=>{d.files.folder_separator=' - ';})),undefined);
  const custom=[{id:'custom',label:'Custom',controls:[],validate:d=>d.pages.custom?.ok?undefined:'Needs ok'}];
  assert.deepEqual(ui.validateOptions(draft(),custom),{page:'custom',message:'Needs ok'});
});
test('extension lists accept Mp3tag-style patterns',()=>{
  assert.equal(ui.parseExtensions(''),null);
  assert.deepEqual(ui.parseExtensions('*.MP3;*.flac, .ogg mp+ *.mp3'),['mp3','flac','ogg','mp+']);
  assert.throws(()=>ui.parseExtensions('*.mp3;*'),/Invalid/);
});
test('controls render current values, dependent enabling and escaped labels',()=>{
  const mpeg=ui.pageTree().find(({page})=>page.id==='tags-mpeg').page;
  const markup=d=>mpeg.controls.map(c=>ui.controlMarkup(c,d)).join('');
  const defaults=markup(draft());
  assert.match(defaults,/data-option="tags\.mpeg\.read_ape" checked/);
  assert.match(defaults,/data-option="tags\.mpeg\.id3v2_only_if_needed"\s+>/);
  assert.match(defaults,/value="v23_utf16" checked/);
  const disabled=markup(draft(d=>{d.tags.mpeg.write_id3v1=false;d.tags.mpeg.write_id3v2=false;}));
  assert.match(disabled,/data-option="tags\.mpeg\.id3v2_only_if_needed"\s+disabled/);
  assert.match(disabled,/<fieldset class="options-choices" disabled>/);
  assert.equal(ui.controlMarkup({kind:'text',path:'files.restrict_extensions',label:'<b>'},draft(d=>{d.files.restrict_extensions='"x"';})).includes('&lt;b&gt;')&&ui.controlMarkup({kind:'text',path:'files.restrict_extensions',label:'x'},draft(d=>{d.files.restrict_extensions='"x"';})).includes('value="&quot;x&quot;"'),true);
});
test('sorting and window title follow General options',()=>{
  const natural=ui.naturalCollator(fixture),plain=ui.naturalCollator({...fixture,general:{...fixture.general,natural_sort:false}});
  assert.deepEqual(['11','2','1'].sort(natural.compare),['1','2','11']);
  assert.deepEqual(['11','2','1'].sort(plain.compare),['1','11','2']);
  assert.equal(ui.windowTitle('/music',fixture),'TagGroove — /music');
  assert.equal(ui.windowTitle('No folder loaded',fixture),'TagGroove');
  assert.equal(ui.windowTitle('/music',{...fixture,general:{...fixture.general,show_directory_in_title:false}}),'TagGroove');
});

test('Directories offers the cover art directory stored with the extension pages',()=>{
  const page=ui.pageTree().find(({page})=>page.id==='directories').page;
  const markup=page.controls.map(c=>ui.controlMarkup(c,draft(d=>{d.pages.directories={cover_art:'%_folderpath%Scans'};}))).join('');
  assert.match(markup,/data-option="pages\.directories\.cover_art" value="%_folderpath%Scans"/);
  assert.match(page.controls.map(c=>ui.controlMarkup(c,draft())).join(''),/data-option="pages\.directories\.cover_art" value=""/);
});


test('associated files has a saved opt-in policy with an unchecked default',()=>{
  const page=ui.pageTree().find(({page})=>page.id==='files').page;
  const markup=value=>page.controls.map(control=>ui.controlMarkup(control,value)).join('');
  assert.match(markup(draft()),/data-option="pages\.files\.associated_files"\s*>/);
  assert.match(markup(draft(value=>{value.pages.files={associated_files:true};})),/data-option="pages\.files\.associated_files" checked/);
  assert.match(markup(draft()),/Copy, other converters and action-driven renames do not use this option/);
});
