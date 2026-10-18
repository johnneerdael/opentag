import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';
import {englishCatalog,extractBackendSource} from '../scripts/i18n-extract.mjs';

const source=async file=>readFile(new URL(`../src/${file}`,import.meta.url),'utf8');
const moduleUrl=async(file,replacements={})=>{
  let js=ts.transpileModule(await source(file),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  for(const [from,to] of Object.entries(replacements))js=js.replace(from,to);
  return `data:text/javascript;base64,${Buffer.from(js).toString('base64')}`;
};
const load=async file=>import(await moduleUrl(file));
const {Translator,catalogMeta}=await load('i18n/core.ts'),{DomTranslator}=await load('i18n/dom.ts');
const {controlMarkup,OptionsUI,defaultSettings,registerOptionsPage,windowTitle,parseExtensions}=await load('options-ui.ts');
const {ColumnUI,defaultLayout,normalizeLayout:normalizeColumns,setColumnNameTranslation,columnName,headerMarkup,editKey}=await load('columns.ts'),{ActionUI,panelKey}=await load('action-ui.ts');
const {CellEditor}=await load('cell-editor.ts');
const {TagPanel,defaultLayout:defaultPanelLayout,defaultSettings:defaultPanelSettings,settingsError}=await load('tag-panel.ts');
const {CoverUI}=await import(await moduleUrl('covers.ts',{"import './covers.css';":'',"'./keyboard'":JSON.stringify(await moduleUrl('keyboard.ts')),"'./cover-model'":JSON.stringify(await moduleUrl('cover-model.ts'))}));
const {PICTURE_TYPES}=await load('cover-model.ts');
const {contextMenuMarkup,RowContextMenu}=await load('row-context-menu.ts');
const {ExtendedTags}=await import(await moduleUrl('extended-tags.ts',{"import './extended-tags.css';":'',"'./tag-edits'":JSON.stringify(await moduleUrl('tag-edits.ts')),"'./row-context-menu'":JSON.stringify(await moduleUrl('row-context-menu.ts'))}));
const {ToolsUI,toolsOptionsPage}=await load('tools.ts'),{installHelperMenus}=await load('helper-menu.ts');
const {customizeToolbar,itemHint,itemLabel,userLabel,visibleItems,isSeparator,isSpacer,TOOLBAR_COMMANDS}=await load('toolbar.ts');
const {copyAsTextDialog,UTILS}=await load('utils-ui.ts'),{PlaylistUI}=await load('playlist-ui.ts'),{openTagToTag}=await load('tag-tag-ui.ts');
const {offerOpenOutputs,ExportUI}=await load('export-ui.ts');
const {FileOpsUI,nameList,defaultDirectoryState}=await load('file-ops.ts');
const {copiedMessage,pastePlan,copyTags}=await load('tag-clipboard.ts');
const {cleanupText,libraryOptionsPage}=await load('library.ts');
const {mappingOptionsPage}=await load('field-mapping.ts');
const {sourcePresentation}=await load('document-state.ts');
const {tagSourcesOptionsPages,TagSourcesUI,webScriptErrorText,resultColumns}=await load('tag-sources.ts');
const {removeTagParts}=await load('remove-tag.ts');
const {converterValidator,createConverterInputTracker,createConverterReview}=await load('converter-review.ts');
const {converterRollbackError,converterRollbackText}=await import(await moduleUrl('converter-errors.ts', {"'./i18n/core'":JSON.stringify(await moduleUrl('i18n/core.ts'))}));
const rollbackCases=JSON.parse(await readFile(new URL('./fixtures/converter-rollback.json',import.meta.url),'utf8'));
const batchModuleURL=await moduleUrl('batch-errors.ts',{"'./i18n/core'":JSON.stringify(await moduleUrl('i18n/core.ts')),"'./converter-errors'":JSON.stringify(await moduleUrl('converter-errors.ts',{"'./i18n/core'":JSON.stringify(await moduleUrl('i18n/core.ts'))}))});
const {causeText,batchErrorText,normalizeBatchDiagnostics,fileErrorText,showFileError,clearFileError,BatchFailure}=await import(batchModuleURL);
const {ColumnImportStatus,mtaImportFailure,importFailure,skippedLines}=await import(await moduleUrl('import-diagnostics.ts',{"'./i18n/core'":JSON.stringify(await moduleUrl('i18n/core.ts')),"'./batch-errors'":JSON.stringify(batchModuleURL)}));
const batchDiagnosticCases=JSON.parse(await readFile(new URL('./fixtures/batch-error-diagnostics.json',import.meta.url),'utf8'));

const {listSummary,technicalSummary}=await load('list-commands.ts');
const {Messages}=await load('messages.ts');
const {WebSourcesUI,settingsMarkup,readSettings}=await import(await moduleUrl('web-sources.ts',{"'./action-ui'":JSON.stringify(await moduleUrl('action-ui.ts'))}));
const main=await source('main.ts'),parsed=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true);
const tagSourceParsed=ts.createSourceFile('tag-sources.ts',await source('tag-sources.ts'),ts.ScriptTarget.Latest,true);
const row=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='rowMarkup').getText(parsed);
// Execute the production row renderer with only its data dependencies stubbed.
const rowJS=ts.transpileModule(`export function render(entry){
  const esc=value=>String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const drafts=new Map([[entry.path,{TITLE:['Help']}]]),selected=new Set();
  const merged=()=>entry.fields,cellValue=()=>entry.fields.TITLE[0],editKey=()=> 'TITLE',panelKey=(_,key)=>key;
  const tableColumns=()=>[{id:'title',value:'%title%',align:'left'}];
  ${row}
  return ${JSON.stringify(main.match(/<tbody\b[^>]*><\/tbody>/)[0])}.replace('</tbody>',rowMarkup(entry,0)+'</tbody>');
}`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {render:renderRows}=await import(`data:text/javascript;base64,${Buffer.from(rowJS).toString('base64')}`);
const catalogs=Object.fromEntries(await Promise.all(['de','nl'].map(async locale=>[locale,JSON.parse(await source(`i18n/catalogs/${locale}.json`))])));
const esc=value=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
function modal(p){return(title,content,buttons)=>{const dialog=p.window.document.createElement('dialog');dialog.innerHTML=`<header>${esc(title)}</header><div class="dialog-content">${content}</div><footer>${buttons}</footer>`;p.body.append(dialog);dialog.setAttribute('open','');return dialog;};}
function mainTemplate(needle,values,index=0){
  values={fileErrorText,...values};
  const templates=[];
  const visit=node=>{if(ts.isTemplateExpression(node)&&(typeof needle==='function'?needle(node):node.head.text.startsWith(needle)))templates.push(node);ts.forEachChild(node,visit);};visit(parsed);
  const template=templates[index];
  assert.ok(template,`Production template ${needle}`);
  const js=ts.transpileModule(`return ${template.getText(parsed)};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  return new Function(...Object.keys(values),js)(...Object.values(values));
}
function rendererVariable(tree,name,values,owner){
  values={BatchFailure,ColumnImportStatus,mtaImportFailure,importFailure,onLanguageChange:()=>()=>{},...values};
  let declaration;
  const visit=node=>{if(ts.isVariableDeclaration(node)&&node.name.getText(tree)===name)declaration=node;ts.forEachChild(node,visit);};visit(tree);
  assert.ok(declaration,`Production renderer ${name}`);
  const js=ts.transpileModule(`const ${declaration.getText(tree)};return ${name};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  return new Function(...Object.keys(values),js).call(owner,...Object.values(values));
}
function mainMessage(call,needle,index,values){
  let expression;
  const visit=node=>{if(ts.isCallExpression(node)&&node.expression.getText(parsed)===call&&node.arguments[index]?.getText(parsed).includes(needle))expression=node.arguments[index];ts.forEachChild(node,visit);};visit(parsed);
  assert.ok(expression,`Production ${needle} message`);
  const js=ts.transpileModule(`return ${expression.getText(parsed)};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  return new Function(...Object.keys(values),js)(...Object.values(values));
}
const configurationMessage=values=>mainMessage('ask','TagGroove reloads its window',0,values);
const removalMessage=(count,tags)=>mainMessage('messages.confirm','removeTagParts',1,{removeTagParts,selected:{size:count},optionsUI:{settings:{tags}}});
function page(){
  const window=new Window();
  for(const key of ['window','document','Node','NodeFilter','Element','HTMLElement','HTMLInputElement','HTMLTextAreaElement','HTMLButtonElement','MutationObserver','MouseEvent','Event'])globalThis[key]=key==='window'?window:window[key];
  const body=window.document.body;
  let translator=new Translator('en');const dom=new DomTranslator(body,()=>translator);
  return {body,window,translateTree:node=>dom.translateTree(node),translate(locale,catalog=catalogs[locale]??{}){translator=new Translator(locale,catalog);dom.apply();return translator;},close(){window.happyDOM.abort();}};
}
const memory=()=>{const data=new Map();return{getItem:key=>data.get(key)??null,setItem:(key,value)=>data.set(key,value)};};

// Happy DOM has no layout: deterministic text-based metrics verify the production
// ordering/clamping, rather than claiming native pixel/layout qualification.
function popupMetrics(p){
  const proto=p.window.HTMLElement.prototype,names=['offsetWidth','offsetHeight','getBoundingClientRect'],saved=Object.fromEntries(names.map(name=>[name,Object.getOwnPropertyDescriptor(proto,name)]));
  const isPopup=node=>node.matches('.column-menu,.cover-menu,.helper-menu,.helper-submenu,.cover-submenu,.menu-list,.group-submenu > div');
  const size=node=>{const labels=[...node.querySelectorAll('button,label,.context-heading')].filter(el=>!el.closest('[hidden]')).map(el=>el.textContent.trim());return{width:Math.min(650,Math.max(80,...labels.map(label=>label.length*9+24))),height:Math.min(400,Math.max(40,labels.reduce((sum,label)=>sum+20+Math.floor(label.length/24)*8,8)))};};
  for(const [name,axis] of [['offsetWidth','width'],['offsetHeight','height']])Object.defineProperty(proto,name,{configurable:true,get(){return isPopup(this)?size(this)[axis]:saved[name]?.get?.call(this)??0;}});
  Object.defineProperty(proto,'getBoundingClientRect',{configurable:true,writable:true,value:function(){if(!isPopup(this))return saved.getBoundingClientRect.value.call(this);const{width,height}=size(this),left=Number.parseFloat(this.style.left)||(this.parentElement?.classList.contains('group-submenu')?(this.parentElement.classList.contains('open-left')?680-width:680):0),top=Number.parseFloat(this.style.top)||0;return{left,top,width,height,right:left+width,bottom:top+height};}});
  Object.defineProperty(p.window,'innerWidth',{value:800,configurable:true});Object.defineProperty(p.window,'innerHeight',{value:600,configurable:true});
  return{size,restore(){for(const name of names)if(saved[name])Object.defineProperty(proto,name,saved[name]);else delete proto[name];}};
}

test('production popups localize synchronously before right/bottom placement for DE/NL and long custom text',()=>{
  const long=Object.fromEntries(['Copy','Copy field','Cut field','Paste field','Customize columns…','Add cover…','Manage history…','Customize tag panel…'].map(key=>[key,`${key} ${'long translated label '.repeat(4)}`]));
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',long]]){
    const p=page(),metrics=popupMetrics(p);
    try{
      const t=p.translate(locale,catalog),translateNow=node=>p.translateTree(node);
      const placed=(menu,label,margin=4,right=800-margin)=>{assert.ok(menu.textContent.includes(t.t(label)),menu.textContent);const size=metrics.size(menu);assert.equal(Number.parseFloat(menu.style.left),right-size.width);assert.equal(Number.parseFloat(menu.style.top),600-size.height-margin);assert.ok(Number.parseFloat(menu.style.left)+size.width<=800-margin);assert.ok(Number.parseFloat(menu.style.top)+size.height<=600-margin);};
      const table=p.window.document.createElement('table');table.innerHTML='<tbody><tr data-path="Help"><td>Help</td></tr></tbody>';p.body.append(table);
      const rows=new RowContextMenu({translateNow,prepare:()=>{},items:()=>[{label:'Copy',id:'copy'}],activate:()=>{}});rows.open(table.querySelector('tr'),795,595);placed(p.body.querySelector('.row-context-menu'),'Copy');rows.close();
      const columns=new ColumnUI({translateNow,layout:defaultLayout});columns.headerMenu(795,595,()=>{});placed(p.body.querySelector('.column-menu'),'Customize columns…');columns.closeMenu();
      const covers=new CoverUI({translateNow,selection:()=>[{path:'Help',covers:[]}],displayed:()=>({path:'Help',covers:[],index:0}),reason:()=>'',clipboard:()=>false,mac:false});covers.openMenu(795,595,false);placed(p.body.querySelector('.cover-menu'),'Add cover…');covers.closeMenu();
      const extended=new ExtendedTags({translateNow,selection:()=>['Help'],listed:()=>['Help'],fields:()=>({TITLE:['Help']}),describe:()=>({name:'Help'}),modal:modal(p),coverMarkup:()=>'',bindCover:()=>{}});extended.show();p.body.querySelector('.extended-row').dispatchEvent(new p.window.MouseEvent('contextmenu',{bubbles:true,clientX:795,clientY:595}));placed(p.body.querySelector('.extended-menu'),'Cut field');p.body.querySelector('.extended-dialog').close();
      p.body.insertAdjacentHTML('beforeend','<div><input id="popup-history" data-helper="history" data-history="probe" data-history-manage></div>');installHelperMenus({translateNow,storage:memory(),fieldNames:()=>[],panelFields:()=>[]});const helper=p.body.querySelector('#popup-history').parentElement.querySelector('.helper-button');helper.getBoundingClientRect=()=>({left:770,right:790,top:560,bottom:580,width:20,height:20});helper.click();placed(p.body.querySelector('.helper-menu'),'Manage history…',4,790);p.body.querySelector('.helper-menu').remove();
      const panel=new TagPanel({translateNow,values:()=>[],disabled:()=>false,directories:()=>({selected:[],all:[]})});p.body.innerHTML=panel.titleHTML()+panel.fieldsHTML();panel.bind(p.body);const details=p.body.querySelector('.panel-menu');details.getBoundingClientRect=()=>({left:780,right:800,top:570,bottom:590,width:20,height:20});details.open=true;details.dispatchEvent(new p.window.Event('toggle'));placed(details.querySelector('.menu-list'),'Customize tag panel…',8);
      p.body.innerHTML='<details class="group-submenu"><summary translate="no">Help</summary><div><button>Copy field</button></div></details>';new ActionUI({translateNow}).bindMenu(p.body);const submenu=p.body.querySelector('details');submenu.open=true;submenu.dispatchEvent(new p.window.Event('toggle'));assert.equal(submenu.querySelector('button').textContent,t.t('Copy field'));assert.ok(submenu.classList.contains('open-left'));assert.ok(submenu.querySelector('div').getBoundingClientRect().right<=800);
    }finally{metrics.restore();p.close();}
  }
});

test('the actual Language options preserve authored catalog names in System default with existing catalog keys',async()=>{
  const custom={'System default ({nameOf})':'Default catalog: {nameOf}',Help:'Assistance',Title:'Heading'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const name of ['Help','Title','System default','System default (Help)','English','Deutsch','Nederlands'])for(const userLocale of ['fr','en']){
    const p=page();
    try{
      const user={'@meta':{name}},file=`${userLocale}.json`,languages=[...(userLocale==='en'?[]:[{locale:'en',name:'English',catalog:{},source:'bundled'}]),{...catalogMeta(user,file),catalog:user,source:'user',file}];
      const provider=`data:text/javascript;base64,${Buffer.from(`export const availableLanguages=()=>${JSON.stringify(languages)};export const systemLanguage=()=> ${JSON.stringify(userLocale)};`).toString('base64')}`;
      const {languageOptionsPage}=await import(await moduleUrl('language-options.ts',{"'./i18n'":JSON.stringify(provider)}));
      const options=languageOptionsPage({}),control=options.controls[0];
      p.body.innerHTML=controlMarkup(control,{pages:{language:{language:'system'}}});const t=p.translate(locale,catalog);
      assert.equal(p.body.querySelector('option[value="system"]').textContent,t.t('System default ({nameOf})',{nameOf:name}));
      assert.equal(p.body.querySelector(`option[value="${userLocale}"]`).textContent,`${name} (${file})`);
      assert.equal(p.body.querySelector(`option[value="${userLocale}"]`).getAttribute('translate'),'no');
      assert.equal(p.body.querySelector('select').value,'system');
      // Happy DOM mis-parses selectedness for a final option; verify the
      // production selected attribute for aliases, leaving native selection
      // behavior to the reviewed desktop suite.
      p.body.innerHTML=controlMarkup(control,{pages:{language:{language:userLocale.toUpperCase()}}});
      assert.equal(p.body.querySelector('option[selected]').value,userLocale);
      assert.ok(!p.body.textContent.includes('Unavailable:'));
    }finally{p.close();}
  }
});

test('reserved system catalog locales cannot duplicate the real Language default choice',async()=>{
  const {languageSet,resolveLanguage}=await load('i18n/core.ts'),english={'@meta':{name:'English',locale:'en'},Help:'Help'};
  const bundled=[{locale:'en',name:'English',catalog:english,source:'bundled'}];
  const invalid=[{file:'system.json',catalog:{'@meta':{name:'Help'},Help:'Wrong system'}},{file:'SyStEm.JSON',catalog:{Help:'Wrong system'}},{file:'fr.json',catalog:{'@meta':{locale:'SYSTEM',name:'Title'},Help:'Wrong system'}}];
  const valid=[{file:'system.json',catalog:{'@meta':{locale:'fr',name:'Français'},Help:'Aide'}},{file:'en.json',catalog:{'@meta':{name:'English custom'},Help:'Assistance'}}];
  const result=languageSet(bundled,[...invalid,...valid],english);
  assert.equal(result.languages.has('system'),false);
  assert.deepEqual(result.warnings,invalid.map(({file})=>({key:'{file}: unknown language',params:{file}})));
  assert.equal(catalogMeta({'@meta':{locale:'system-Latn'}},'other.json').locale,'system-Latn','only the exact reserved choice collides');
  assert.equal(resolveLanguage('system',['fr-FR'],[...result.languages.keys()]),'fr');
  const provider=`data:text/javascript;base64,${Buffer.from(`export const availableLanguages=()=>${JSON.stringify([...result.languages.values()])};export const systemLanguage=()=> 'fr';`).toString('base64')}`;
  const {languageOptionsPage,languageWarningText}=await import(await moduleUrl('language-options.ts',{"'./i18n'":JSON.stringify(provider)}));
  const custom={'{file}: unknown language':'Rejected catalog [{file}]','System default ({nameOf})':'Default [{nameOf}]','SyStEm.JSON':'Wrong filename'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();try{
      const t=new Translator(locale,catalog),options=languageOptionsPage({warningText:warning=>languageWarningText(warning,(key,params)=>t.t(key,params))});options.refresh(result.warnings);
      const control=options.controls[0];p.body.innerHTML=controlMarkup(control,{pages:{language:{language:'system'}}});p.translate(locale,catalog);
      assert.equal(p.body.querySelectorAll('option[value="system"]').length,1);assert.equal(p.body.querySelector('option[selected]').value,'system');assert.equal(p.body.querySelector('option[value="system"]').textContent,t.t('System default ({nameOf})',{nameOf:'Français'}));
      const warnings=options.controls.find(control=>control.kind==='group').controls.find(control=>control.id==='language-warnings');p.body.innerHTML+=warnings.render();p.translateTree(p.body);
      assert.deepEqual([...p.body.querySelectorAll('[data-language-warning]')].map(el=>el.textContent),invalid.map(({file})=>t.t('{file}: unknown language',{file})));
      p.body.innerHTML=controlMarkup(control,{pages:{language:{language:'fr'}}});assert.equal(p.body.querySelector('option[selected]').value,'fr');assert.equal(result.languages.get('fr').catalog.Help,'Aide');assert.equal(result.languages.get('en').catalog.Help,'Assistance');
    }finally{p.close();}
  }
});

test('launch locations reach the real header and native-title renderers with the correct provenance',async()=>{
  const custom={'{length} added locations':'Title',Title:'Heading','TagGroove — {location}':'Window: {location}'};
  const declaration=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='syncWindowTitle');
  const js=ts.transpileModule(declaration.getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  const cases=[[{directories:['/one','/two'],files:[],add:false},'2 added locations',false],[{directories:[],files:['/one/a.mp3','/two/b.mp3'],add:true},'2 added locations',false],[{directories:['/one'],files:['/one/a.mp3'],add:false},'2 added locations',false],[{directories:['2 added locations'],files:[],add:true},'2 added locations',true],[{directories:['No folder loaded'],files:[],add:false},'No folder loaded',true],[{directories:[],files:['/one/a.mp3','/one/b.mp3'],add:false},'/one',true]];
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const [request,expectedLocation,isData] of cases){
    const p=page(),t=new Translator(locale,catalog),titles=[];let loaded=0;
    try{
      const shown=isData?expectedLocation:t.t('{length} added locations',{length:2});
      const ui=new FileOpsUI({preferences:()=>({startup:'empty',subfolders:true}),pendingCount:()=>0,openReason:()=>'',confirm:async()=>true,changed:()=>{},invoke:async()=>defaultDirectoryState(),load:async(paths,options)=>{
        loaded++;assert.deepEqual(paths,[...request.directories,...request.files]);
        const location=options.location,locationIsData=options.locationIsData!==false;
        const locationView=rendererVariable(parsed,'locationView',{entry:undefined,providerEntry:()=>false,providerState:undefined,sourcePresentation,location,locationIsData});
        p.body.innerHTML=rendererVariable(parsed,'locationMarkup',{esc,locationView});p.translate(locale,catalog);
        assert.equal(p.body.querySelector('.path').textContent,shown);assert.equal(p.body.querySelector('.path').title,shown);
        const values={app:{inert:false},translate:text=>t.translate(text),windowTitle,location,locationIsData,optionsUI:{settings:defaultSettings()},desktop:true,getCurrentWindow:()=>({setTitle:async title=>titles.push(title)})};
        new Function(...Object.keys(values),`let shownTitle='';${js};return syncWindowTitle;`)(...Object.values(values))();
        assert.deepEqual(titles,[t.t('TagGroove — {location}',{location:shown})]);return true;
      }});
      await ui.launch(request);assert.equal(loaded,1);
    }finally{p.close();}
  }
});
test('duplicate export rename errors translate their scaffold while preserving the authored conflicting name',async()=>{
  const custom={'An export configuration named {name} already exists.':'Duplicate export: {name}.',Help:'Assistance',Title:'Heading',Rename:'Change name'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const name of ['Help','Title','My  Files']){
    const p=page(),css=Object.getOwnPropertyDescriptor(globalThis,'CSS'),t=new Translator(locale,catalog);
    Object.defineProperty(globalThis,'CSS',{configurable:true,value:p.window.CSS});
    try{
      const source={version:1,configurations:[{id:'original',name:'Original',template:'%title%'},{id:'duplicate',name,template:'%album%'}],preferences:{configuration:'original',filename:'out.txt',append:false,partition:null,write_bom:true,create_directories:false}};let saved;
      const ui=new ExportUI({load:async()=>source,save:async value=>{saved=structuredClone(value);},reason:()=>'',selectedCount:()=>1});
      await ui.open();p.translate(locale,catalog);p.body.querySelector('#export-rename').click();
      const input=p.body.querySelector('.export-rename');input.value=` ${name} `;input.dispatchEvent(new p.window.KeyboardEvent('keydown',{key:'Enter',bubbles:true}));
      await p.window.happyDOM.waitUntilComplete();
      assert.ok(p.body.querySelector('.dialog-error').textContent.includes(name));
      assert.equal(p.body.querySelector('.dialog-error').textContent,t.t('An export configuration named {name} already exists.',{name}));
      assert.deepEqual([...p.body.querySelectorAll('#export-list li')].map(item=>item.textContent),['Original',name]);
      assert.equal(p.body.querySelector('#export-rename').textContent,t.t('Rename'));
      p.body.querySelector('#export-save').click();await p.window.happyDOM.waitUntilComplete();
      assert.deepEqual(saved.configurations,source.configurations);assert.deepEqual(source.configurations.map(item=>item.name),['Original',name]);
    }finally{if(css)Object.defineProperty(globalThis,'CSS',css);else delete globalThis.CSS;p.close();}
  }
});
test('cover viewer type captions translate independently of counters and authored descriptions',async()=>{
  const custom={'Front Cover':'Front image','Back Cover':'Back image',Help:'Assistance',Title:'Heading','Close cover viewer':'Close image'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page(),t=new Translator(locale,catalog);
    try{
      let covers=['CoverFront','CoverBack'].map((picture_type,index)=>({index,digest:`fixture-${index}`,mime:'image/png',size:42,width:1,height:1,picture_type,description:index?'Title':'Help',data_base64:'AA=='})),index=0;
      const ui=new CoverUI({selection:()=>[{path:'Help',covers}],displayed:()=>({path:'Help',covers,index}),reason:()=>'',show:next=>{index=next;ui.updateViewer();}});
      ui.openViewer();p.translate(locale,catalog);
      assert.equal(p.body.querySelector('.cover-viewer header > span').textContent,`${t.t('Front Cover')} · 1 / 2`);
      assert.equal(p.body.querySelector('.cover-viewer footer > span').textContent,'1×1, 42 B, PNG · Help');
      p.body.querySelector('[data-viewer="next"]').click();await p.window.happyDOM.waitUntilComplete();
      assert.equal(p.body.querySelector('.cover-viewer header > span').textContent,`${t.t('Back Cover')} · 2 / 2`);
      assert.equal(p.body.querySelector('.cover-viewer footer > span').textContent,'1×1, 42 B, PNG · Title');
      assert.equal(p.body.querySelector('[data-viewer="close"]').getAttribute('aria-label'),t.t('Close cover viewer'));
      covers=[covers[0]];index=0;ui.updateViewer();await p.window.happyDOM.waitUntilComplete();
      assert.equal(p.body.querySelector('.cover-viewer header > span').textContent,t.t('Front Cover'));
      p.translate('en');assert.equal(p.body.querySelector('.cover-viewer header > span').textContent,'Front Cover');
    }finally{p.close();}
  }
});
test('cover extraction type captions translate beside unchanged technical details and filename previews',async()=>{
  const custom={'Front Cover':'Front image','Back Cover':'Back image',Help:'Assistance',Save:'Store'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const [picture_type,label] of [['CoverFront','Front Cover'],['CoverBack','Back Cover']]){
    const p=page(),t=new Translator(locale,catalog);
    try{
      const ui=new CoverUI({modal:modal(p),evaluate:async()=> 'Help',modalError:(_dialog,error)=>{throw error;}});
      ui.extract('Help',{index:0,mime:'image/png',size:42,width:1,height:1,picture_type});
      await p.window.happyDOM.waitUntilComplete();p.translate(locale,catalog);
      assert.equal(p.body.querySelector('.dialog-content > p').textContent,`${t.t(label)} · 1×1, 42 B, PNG`);
      assert.ok(p.body.querySelector('#cover-extract-preview').textContent.includes('Help.png'));
      assert.equal(p.body.querySelector('#cover-extract-save').textContent,t.t('Save'));
      p.translate('en');assert.equal(p.body.querySelector('.dialog-content > p').textContent,`${label} · 1×1, 42 B, PNG`);
    }finally{p.close();}
  }
});
test('the native window-title wrapper translates generated locations once and preserves authored paths',async()=>{
  const custom={'TagGroove — {location}':'Window: {location}','Provider documents':'Help','{length} added locations':'Title',Help:'Assistance',Title:'Heading','No folder loaded':'Help'};
  const declaration=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='syncWindowTitle');
  const js=ts.transpileModule(declaration.getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const t=new Translator(locale,catalog);
    const cases=[['Provider documents',false,t.t('Provider documents')],['2 added locations',false,t.t('{length} added locations',{length:2})],['No folder loaded',false,null],['No folder loaded',true,'No folder loaded'],['Provider documents',true,'Provider documents'],['2 added locations',true,'2 added locations'],['/My  Files/Help\nTitle',true,'/My  Files/Help\nTitle']];
    for(const [location,locationIsData,label] of cases)for(const show of [false,true]){
      const settings=defaultSettings();settings.general.show_directory_in_title=show;const calls=[];
      const values={app:{inert:false},translate:text=>t.translate(text),windowTitle,location,locationIsData,optionsUI:{settings},desktop:true,getCurrentWindow:()=>({setTitle:async title=>{calls.push(title);}})};
      const sync=new Function(...Object.keys(values),`let shownTitle='';${js};return syncWindowTitle;`)(...Object.values(values));
      sync();await Promise.resolve();
      assert.deepEqual(calls,[show&&label!==null?t.t('TagGroove — {location}',{location:label}):'TagGroove']);
      sync();assert.equal(calls.length,1);
    }
  }
});
test('real keep-field selection preserves field syntax and each loaded value while translating its status',()=>{
  const singular='Keeping the existing {key} values of {size} file.',plural='Keeping the existing {key} values of {size} files.';
  const custom={ARTIST:'Performer',TITLE:'Name',[singular]:'Keep {key} in {size} entry.',[plural]:'Keep {key} in {size} entries.'};
  const functions=['updateDraft','keepField'].map(name=>parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name).getText(parsed)).join('\n');
  const js=ts.transpileModule(functions,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const key of ['ARTIST','TITLE'])for(const count of [2,1]){
    const p=page(),entries=new Map(Array.from({length:count},(_,i)=>[`/file-${i}.mp3`,{path:`/file-${i}.mp3`,format:'MP3',fields:{[key]:[i?'Title':'Help'],OTHER:['loaded']}}])),selected=new Set(entries.keys()),drafts=new Map([...selected].map((path,i)=>[path,{[key]:[`Changed ${i}`],...(i?{}:{OTHER:['pending']})}]));let changed=0;
    try{
      const before=structuredClone([...entries]),t=new Translator(locale,catalog),values={entries,selected,drafts,paths:()=>[...selected],panelKey,merged:entry=>({...entry.fields,...drafts.get(entry.path)}),copyOverrides:new Map(),copyBaseUnknown:()=>false,pendingCovers:new Map(),changedData:()=>{changed++;}};
      const producer=new Function(...Object.keys(values),`let status='';${js};return{keep:keepField,status:()=>status};`)(...Object.values(values));
      const show=()=>{p.body.querySelector('.status-message').textContent=producer.status();};
      const panel=new TagPanel({values:field=>[...selected].map(path=>(drafts.get(path)?.[field]??entries.get(path).fields[field]??[])[0]??''),disabled:()=>false,translateLabel:text=>t.translate(text),translateNow:p.translateTree,keep:producer.keep,changed:show});
      panel.settings.panel.fields=[defaultPanelLayout().fields.find(field=>field.field===key)];p.body.innerHTML=panel.fieldsHTML()+'<span class="status-message"></span>';panel.bind(p.body);p.translate(locale,catalog);
      if(count>1){p.body.querySelector('.combo-toggle').click();p.body.querySelector('.combo-list [data-option="0"]').click();}else{producer.keep(key);show();}
      p.translateTree(p.body);assert.equal(p.body.querySelector('.status-message').textContent,t.t(count===1?singular:plural,{key,size:count}));
      assert.deepEqual([...entries],before);assert.equal(changed,count);assert.equal(drafts.size,1);assert.deepEqual(drafts.get('/file-0.mp3'),{OTHER:['pending']});
    }finally{p.close();}
  }
});
test('tag-panel validation keeps technical field keys raw while shortcut labels retain their locale',()=>{
  const messages=['{key}: at most 5000 custom values are supported.','{key}: custom values must be nonempty single lines.','{key}: remove duplicate custom values.'];
  const custom={ARTIST:'Performer',...Object.fromEntries(messages.map(key=>[key,`Field ${key}`])),'@keys':{Ctrl:'Control'}};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const [index,values] of [Array.from({length:5001},(_,i)=>String(i)),[' padded'],['same','same']].entries()){
    const p=page();try{const settings=defaultPanelSettings();settings.field_values.lists.ARTIST=values;p.body.textContent=settingsError(settings);const t=p.translate(locale,catalog);assert.equal(p.body.textContent,t.t(messages[index],{key:'ARTIST'}));assert.equal(t.translate('Ctrl S'),locale==='de'?'Strg S':locale==='en'?'Control S':'Ctrl S');}finally{p.close();}
  }
});
test('real custom-value notifications and layout errors translate complete scalar-field messages',async()=>{
  const added='Added “{value}” to the custom values for {field}.',duplicate='“{value}” is already a custom value for {field}.',empty='{field}: enter a name for the field.',used='{field} is already used for another field and cannot be added twice.';
  const custom={[added]:'Stored {value} for {field}.',[duplicate]:'Already stored {value} for {field}.',[empty]:'Name required: {field}.',[used]:'Duplicate field: {field}.','Copied {lines} line as text':'Text copy: {lines} line','Copied {lines} lines as text':'Text copy: {lines} lines',Help:'Assistance',ARTIST:'Performer'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();try{
      const panel=new TagPanel({persistent:()=>false,values:()=>[],disabled:()=>false,directories:()=>({selected:[],all:[]}),changed:()=>{},notify:message=>{p.body.querySelector('#notice').textContent=message;},fail:error=>{throw error;},altShortcut:()=>false});
      p.body.innerHTML=panel.fieldsHTML()+'<p id="notice"></p>';panel.bind(p.body);const t=p.translate(locale,catalog),input=p.body.querySelector('[data-field="ARTIST"]');input.value='Help';
      for(const key of [added,duplicate]){input.dispatchEvent(new p.window.KeyboardEvent('keydown',{key:'Enter',shiftKey:true,bubbles:true}));await p.window.happyDOM.waitUntilComplete();assert.equal(p.body.querySelector('#notice').textContent,t.t(key,{value:'Help',field:'ARTIST'}));}
      assert.deepEqual(panel.settings.field_values.lists.ARTIST,['Help']);
      for(const [key,change] of [[empty,settings=>{settings.panel.fields[1].name='';}],[used,settings=>{settings.panel.fields.push({...settings.panel.fields[1]});}]]){
        const settings=defaultPanelSettings();change(settings);p.body.querySelector('#notice').textContent=settingsError(settings);p.translateTree(p.body);assert.equal(p.body.querySelector('#notice').textContent,t.t(key,{field:'ARTIST'}));
      }
      for(const lines of [1,2]){const source=mainTemplate(node=>node.head.text==='Copied '&&node.getText(parsed).includes('lines===1'),{lines});p.body.querySelector('#notice').textContent=source;p.translateTree(p.body);assert.equal(p.body.querySelector('#notice').textContent,t.t(`Copied {lines} line${lines===1?'':'s'} as text`,{lines}));}
      assert.equal(t.translate('Copied Help lines as text'),'Copied Help lines as text');
    }finally{p.close();}
  }
});
test('real column-import and source-install warnings retain joined diagnostics after the translated scaffold',async()=>{
  const keys=['Imported {length} Mp3tag column from {path}; skipped {length2}: {join}','Imported {length} Mp3tag columns from {path}; skipped {length2}: {join}','Installed {names}. {length} tag source file could not be read: {join}','Installed {names}. {length} tag source files could not be read: {join}'];
  const custom=Object.fromEntries(keys.map(key=>[key,`Result: ${key}`]));custom['Help.ini']='Wrong path';custom['Help.src']='Wrong filename';
  const declaration=parsed.statements.filter(ts.isVariableStatement).flatMap(node=>[...node.declarationList.declarations]).find(node=>node.name.getText(parsed)==='columnUI');
  const callback=declaration.initializer.arguments[0].properties.find(node=>node.name?.getText(parsed)==='importLayout').initializer;
  const js=ts.transpileModule(`const run=${callback.getText(parsed)};return{run,status:()=>status};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const count of [1,2]){
    const p=page();try{
      const t=new Translator(locale,catalog),skipped=Array.from({length:count},()=> 'Wrong expression'),layout=defaultLayout();layout.columns=layout.columns.slice(0,count);
      const host=new Function('ColumnImportStatus','importFailure','open','invoke','normalizeLayout','basename',`let status='';${js}`)(ColumnImportStatus,importFailure,async()=>'/tmp/Help.ini',async()=>({layout,skipped}),normalizeColumns,path=>path.split('/').at(-1));
      await host.run();p.body.textContent=host.status().text((key,params)=>t.t(key,params),text=>t.translate(text));p.body.setAttribute('translate','no');
      assert.equal(p.body.textContent,t.t(keys[count===1?0:1],{length:count,path:'Help.ini',length2:count,join:skipped.join('; ')}));
      p.body.removeAttribute('translate');const paths=Array.from({length:count},(_,index)=>`/tmp/${index?'Title':'Help'}.src`),problems=paths.map(path=>({file:path.split('/').at(-1),error:'Wrong expression'}));
      const sources=new WebSourcesUI({pickFiles:async()=>paths,invoke:async()=>({sources:[],problems}),changed:()=>{},report:message=>{p.body.textContent=message;},fail:error=>{throw error;}});
      await sources.install();p.translate(locale,catalog);
      assert.equal(p.body.textContent,t.t(keys[count===1?2:3],{names:paths.map(path=>path.split('/').at(-1)).join(', '),length:count,join:problems.map(problem=>`${problem.file}: ${problem.error}`).join('; ')}));
    }finally{p.close();}
  }
});
test('tag-panel validation preserves invalid identifiers and whitespace in field and custom-value diagnostics',()=>{
  const cases=[['ARTIST ','Field "{id}" must be a trimmed uppercase field name.'],[' TITLE','Field "{id}" must be a trimmed uppercase field name.'],['ARTIST=','Field "{id}" contains an invalid character.'],['TITLE%','Field "{id}" contains an invalid character.'],['_LENGTH','{id}: technical fields cannot be edited in the tag panel.']];
  const custom={ARTIST:'Performer',TITLE:'Heading','ARTIST=':'Wrong artist','TITLE%':'Wrong title',_LENGTH:'Wrong length',...Object.fromEntries(cases.map(([,key])=>[key,`Invalid identifier: {id}`]))};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const [id,key] of cases)for(const target of ['field','list']){
    const p=page();try{
      const settings=defaultPanelSettings();
      if(target==='field')settings.panel.fields[0].field=id;else settings.field_values.lists[id]=['Value'];
      p.body.textContent=settingsError(settings);const t=p.translate(locale,catalog);
      assert.equal(p.body.textContent,t.t(key,{id}));assert.ok(p.body.textContent.includes(id));
      p.translate('en');assert.equal(p.body.textContent,key.replace('{id}',id));
    }finally{p.close();}
  }
});
test('extension validation preserves the exact invalid token through localized error rendering',()=>{
  const key='Invalid file extension: {item}',custom={'(empty)':'(missing)',[key]:'Invalid extension token: {item}'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();try{
      assert.throws(()=>parseExtensions('*.mp3;(empty)'),error=>{
        p.body.textContent=error.message;const t=p.translate(locale,catalog);
        assert.equal(p.body.textContent,t.t(key,{item:'(empty)'}));return true;
      });
    }finally{p.close();}
  }
});
test('real tag-panel and extended-tag dialogs preserve technical tokens under supported custom catalogs',()=>{
  const paragraph=Object.keys(catalogs.de).find(key=>key.startsWith('Choose the fields shown on the tag panel.'));
  const caption='Extended tags — {tag} — {name}',custom={[paragraph]:'Special fields: {DIRECTORY} and {SEPARATOR}',[caption]:'Tags: {tag} / {name}',_DIRECTORY:'Wrong field',_SEPARATOR:'Wrong separator','ID3v2.4':'Wrong tag',Help:'Assistance'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();try{
      const panel=new TagPanel({values:()=>[],disabled:()=>false,directories:()=>({selected:[],all:[]})});panel.configure();const t=p.translate(locale,catalog);
      assert.equal(p.body.querySelector('.dialog-content > p').textContent,t.t(paragraph,{DIRECTORY:'_DIRECTORY',SEPARATOR:'_SEPARATOR'}));
      p.body.querySelector('dialog').close();
      const extended=new ExtendedTags({selection:()=>['Help'],listed:()=>['Help'],fields:()=>({TITLE:['Help']}),describe:()=>({name:'Help',tag:'ID3v2.4'}),modal:modal(p),coverMarkup:()=>'',bindCover:()=>{}});extended.show();p.translate(locale,catalog);
      assert.equal(p.body.querySelector('dialog header').textContent,t.t(caption,{tag:'ID3v2.4',name:'Help'}));
    }finally{p.close();}
  }
});
test('real tag-panel Mp3tag import translates skipped reasons and an all-rejected failure but keeps the authored entries',async()=>{
  const wrap=(value,error)=>({kind:'owned',key:'{value}: {error}',params:{value},children:{error}});
  const owned=(key,params={})=>({kind:'owned',key,params,children:{}});
  const lines=['Field 0 (Hilfe): no field name','Field 1 (Mood): TITLE is already on the panel'];
  const causes=[{index:0,cause:wrap('Field 0 (Hilfe)',owned('no field name'))},{index:1,cause:wrap('Field 1 (Mood)',owned('{id} is already on the panel',{id:'TITLE'}))}];
  const partial={panel:defaultPanelLayout(),skipped:lines,skipped_diagnostics:causes};
  const wire={message:`No field could be imported: ${lines.join('; ')}`,import_failure:{kind:'owned',key:'No field could be imported: {error}',params:{},children:{error:{kind:'joined',parts:causes.map(item=>item.cause),separator:'; '}}}};
  const expected={
    de:{partial:['Field 0 (Hilfe): kein Feldname','Field 1 (Mood): TITLE ist bereits im Tag-Panel vorhanden'],failure:'Kein Feld konnte importiert werden: Field 0 (Hilfe): kein Feldname; Field 1 (Mood): TITLE ist bereits im Tag-Panel vorhanden'},
    nl:{partial:['Field 0 (Hilfe): geen veldnaam','Field 1 (Mood): TITLE staat al op het tagpaneel'],failure:'Er kon geen veld worden geïmporteerd: Field 0 (Hilfe): geen veldnaam; Field 1 (Mood): TITLE staat al op het tagpaneel'},
  };
  for(const locale of ['de','nl']){const p=page();try{
    const active=p.translate(locale,catalogs[locale]);
    const host=importMp3tag=>({values:()=>[],disabled:()=>false,directories:()=>({selected:[],all:[]}),pickOpen:async()=>'/tmp/usrfields.ini',importMp3tag,
      skippedLines:imported=>skippedLines(imported,(key,params)=>active.t(key,params),text=>active.translate(text)),
      batchError:error=>error instanceof BatchFailure?error.text((key,params)=>active.t(key,params),text=>active.translate(text)):undefined});
    const settle=async()=>{for(let i=0;i<6;i++)await new Promise(resolve=>setTimeout(resolve,0));};
    new TagPanel(host(async()=>structuredClone(partial))).configure(true);await settle();
    const dialog=p.body.querySelector('dialog');
    assert.deepEqual([...dialog.querySelectorAll('.tp-import-report li')].map(item=>item.textContent),expected[locale].partial);
    assert.equal(dialog.querySelector('.tp-import-report ul').getAttribute('translate'),'no');
    dialog.close();dialog.remove();
    new TagPanel(host(async()=>{throw importFailure(structuredClone(wire),true);})).configure(true);await settle();
    const failed=p.body.querySelector('dialog .dialog-error');
    assert.ok(failed,'the rejected import is reported in the dialog');
    assert.equal(failed.textContent,expected[locale].failure);assert.equal(failed.getAttribute('translate'),'no');
  }finally{p.close();}}
});
test('mapping validation translates generated row labels while preserving authored field identifiers',async()=>{
  registerOptionsPage(mappingOptionsPage());
  const custom={'Mapping {index}':'Rule {index}',ARTIST:'Performer',Help:'Assistance',...Object.fromEntries(['choose a tag type.','enter the source field.','enter the target field.','field names cannot contain line breaks.','{field} already maps to another target. One source cannot map to several targets.'].map(suffix=>[`{mapping}: ${suffix}`,`Issue {mapping}: ${suffix}`]))};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const field of ['ARTIST','Help','My  Field']){
    const p=page(),globals=Object.fromEntries(['CSS','getComputedStyle'].map(name=>[name,Object.getOwnPropertyDescriptor(globalThis,name)]));
    Object.defineProperty(globalThis,'CSS',{configurable:true,value:p.window.CSS});Object.defineProperty(globalThis,'getComputedStyle',{configurable:true,value:p.window.getComputedStyle.bind(p.window)});
    try{
      const settings=defaultSettings();settings.tags.mappings=[{tag_type:'VorbisComment',source:field,target:'FIRST'},{tag_type:'VorbisComment',source:field,target:'SECOND'}];
      const response={settings,defaults:defaultSettings(),warning:null};let saved;
      const ui=new OptionsUI({load:async()=>response,save:async value=>{saved=structuredClone(value);return{...response,settings:value};},appearance:{get:()=> 'system',set:()=>{}},applied:()=>{}});
      await ui.open('tags-mapping');const t=p.translate(locale,catalog);p.body.querySelector('#options-ok').click();await p.window.happyDOM.waitUntilComplete();
      const key='{mapping}: {field} already maps to another target. One source cannot map to several targets.';
      assert.equal(p.body.querySelector('.dialog-error').textContent,t.t(key,{mapping:t.t('Mapping {index}',{index:2}),field}));assert.equal(saved,undefined);
      assert.equal(p.body.querySelector('#mapping-source').value,field);
      const select=p.body.querySelector('#mapping-list');select.value='1';select.dispatchEvent(new p.window.Event('change',{bubbles:true}));
      const input=p.body.querySelector('#mapping-target');input.value='FIRST';input.dispatchEvent(new p.window.Event('input',{bubbles:true}));p.body.querySelector('#options-ok').click();await p.window.happyDOM.waitUntilComplete();
      assert.deepEqual(saved.tags.mappings.map(rule=>rule.source),[field,field]);assert.equal(settings.tags.mappings[1].target,'SECOND');
    }finally{for(const [name,descriptor] of Object.entries(globals))if(descriptor)Object.defineProperty(globalThis,name,descriptor);else delete globalThis[name];p.close();}
  }
  const valid={tag_type:'VorbisComment',source:'ARTIST',target:'PERFORMER'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const [rule,suffix] of [[{...valid,tag_type:'Help'},'choose a tag type.'],[{...valid,source:''},'enter the source field.'],[{...valid,target:''},'enter the target field.'],[{...valid,source:'Help\nTitle'},'field names cannot contain line breaks.']]){
    const p=page();try{p.body.textContent=mappingOptionsPage().validate({tags:{mappings:[rule]}});const t=p.translate(locale,catalog);assert.equal(p.body.textContent,t.t(`{mapping}: ${suffix}`,{mapping:t.t('Mapping {index}',{index:1})}));}finally{p.close();}
  }
});
test('favorite and included-directory selectors preserve restored paths through redraw and save',async()=>{
  registerOptionsPage(libraryOptionsPage({status:async()=>({created:false,files:0,last_hits:0,last_stored:0}),pickDirectories:async()=>['/Music/Title']}));
  const custom={Help:'Assistance',Title:'Heading','Favorite directories':'Saved folders','Included directories':'Included folders',Save:'Store',Remove:'Delete',Open:'Load','Add…':'Include…'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page(),original={version:1,favorites:['Help','Title','My  Files\nTitle'],recent:['Help']};let persisted;
    try{
      const ui=new FileOpsUI({preferences:()=>({startup:'empty'}),changed:()=>{},openReason:()=>'',pickDirectories:async()=>['/Music/Title'],invoke:async(command,args)=>command==='load_directory_state'?structuredClone(original):command==='take_launch_requests'?[]:command==='save_directory_state'?(persisted=structuredClone(args.state)):undefined});
      await ui.initialize();await ui.manage();const t=p.translate(locale,catalog),dialog=p.body.querySelector('.directories-dialog');
      const values=()=>[...dialog.querySelector('#dir-favorites').options].map(option=>option.textContent);
      assert.deepEqual(values(),['★ Help','Title','My  Files\nTitle']);assert.equal(dialog.querySelector('#dir-favorites').value,'0');assert.equal(dialog.querySelector('#dir-save').textContent,t.t('Save'));
      const select=dialog.querySelector('#dir-favorites');select.value='1';select.dispatchEvent(new p.window.Event('change'));dialog.querySelector('#dir-first').click();p.translateTree(dialog);assert.deepEqual(values(),['★ Title','Help','My  Files\nTitle']);
      dialog.querySelector('#dir-add-folder').click();await p.window.happyDOM.waitUntilComplete();p.translateTree(dialog);assert.deepEqual(values(),['★ Title','Help','My  Files\nTitle','/Music/Title']);
      dialog.querySelector('#dir-save').click();await p.window.happyDOM.waitUntilComplete();assert.deepEqual(persisted.favorites,['Title','Help','My  Files\nTitle','/Music/Title']);assert.deepEqual(original.favorites,['Help','Title','My  Files\nTitle']);
    }finally{p.close();}
    const q=page(),globals=Object.fromEntries(['CSS','getComputedStyle'].map(name=>[name,Object.getOwnPropertyDescriptor(globalThis,name)]));
    Object.defineProperty(globalThis,'CSS',{configurable:true,value:q.window.CSS});Object.defineProperty(globalThis,'getComputedStyle',{configurable:true,value:q.window.getComputedStyle.bind(q.window)});
    try{
      const settings=defaultSettings();settings.pages.library={enabled:true,restrict:true,directories:[...original.favorites]};const response={settings,defaults:defaultSettings(),warning:null};let saved;
      const ui=new OptionsUI({load:async()=>response,save:async value=>{saved=structuredClone(value);return{...response,settings:value};},appearance:{get:()=> 'system',set:()=>{}},applied:()=>{}});
      await ui.open('library');const t=q.translate(locale,catalog),values=()=>[...q.body.querySelector('#library-directories').options].map(option=>option.textContent);
      assert.deepEqual(values(),original.favorites);assert.equal(q.body.querySelector('#library-directories').getAttribute('aria-label'),t.t('Included directories'));
      const select=q.body.querySelector('#library-directories');select.value='1';select.dispatchEvent(new q.window.Event('change'));q.body.querySelector('#library-remove').click();q.body.querySelector('#library-add').click();await q.window.happyDOM.waitUntilComplete();q.translateTree(q.body);
      assert.deepEqual(values(),['Help','My  Files\nTitle','/Music/Title']);assert.equal(q.body.querySelector('#library-remove').textContent,t.t('Remove'));
      q.body.querySelector('#options-ok').click();await q.window.happyDOM.waitUntilComplete();assert.deepEqual(saved.pages.library.directories,['Help','My  Files\nTitle','/Music/Title']);assert.deepEqual(settings.pages.library.directories,original.favorites);
    }finally{for(const [name,descriptor] of Object.entries(globals))if(descriptor)Object.defineProperty(globalThis,name,descriptor);else delete globalThis[name];q.close();}
  }
});
test('catalog warning records preserve independent filenames/counts and rerender live without translating output again',async()=>{
  const provider=`data:text/javascript;base64,${Buffer.from('export const availableLanguages=()=>[];export const systemLanguage=()=>"en";').toString('base64')}`;
  const {languageOptionsPage,languageWarningText,languageReadWarnings}=await import(await moduleUrl('language-options.ts',{"'./i18n'":JSON.stringify(provider)}));
  const {languageSet}=await load('i18n/core.ts'),english={'@meta':{name:'English',locale:'en'},Help:'Help',Title:'Title'};
  const files=[{file:'fr.json',catalog:{'@meta':{name:'French',locale:'fr'},Help:'Aide {wrong}'}},{file:'es.json',catalog:{'@meta':{name:'Spanish',locale:'es'}}},{file:'fr: 3 missing, 0 invalid; es.json',catalog:{'@meta':{name:'Other',locale:'eo'}}},{file:'<Help>&;\n nl.json',catalog:[]},{file:'My  Files.json',catalog:{'@meta':{name:'Broken',locale:'???'}}}];
  const expected=[['{file}: {missing} missing, {invalid} invalid',{file:files[0].file,missing:1,invalid:1}],['{file}: {missing} missing, {invalid} invalid',{file:files[1].file,missing:2,invalid:0}],['{file}: {missing} missing, {invalid} invalid',{file:files[2].file,missing:2,invalid:0}],['{file}: not a catalog',{file:files[3].file}],['{file}: unknown language',{file:files[4].file}]];
  const custom={'{file}: {missing} missing, {invalid} invalid':'Missing {missing}; bad {invalid}; file {file}','{file}: not a catalog':'Not catalog: {file}','{file}: unknown language':'Unknown locale: {file}',...Object.fromEntries(files.map(({file})=>[file,'Wrong filename']))};
  custom['Missing 1; bad 1; file fr.json']='Wrong second translation';
  const native=[{file:'large; fr.json',error:'the file is too large',kind:'too_large'},{file:'object: not a catalog.json',error:'not a JSON object',kind:'not_object'},{file:'opaque; nl.json',error:'not a JSON object',kind:'external'}];
  const scan='/Help; fr.json: 3 missing, 0 invalid: Permission denied (os error 13)';
  expected.push(['{file}: the file is too large',{file:native[0].file}],['{file}: not a JSON object',{file:native[1].file}],['',{file:native[2].file,error:native[2].error}],['',{message:scan}]);
  expected.unshift(...expected.splice(5,3));
  const expectedText=(t,key,params)=>key?t.t(key,params):'file' in params?`${params.file}: ${params.error}`:params.message;
  custom['{file}: the file is too large']='Oversized: {file}';custom['{file}: not a JSON object']='Invalid object: {file}';custom[scan]='Wrong scan path';custom['not a JSON object']='Wrong external reason';
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page(),listeners=new Set();let active=new Translator('en');
    try{
      const options=rendererVariable(parsed,'languagePage',{desktop:false,ENGLISH:english,languageOptionsPage,languageWarningText,t:(key,params)=>active.t(key,params),onLanguageChange:listener=>{listeners.add(listener);return()=>listeners.delete(listener);}});
      const warnings=[...languageReadWarnings(native),...languageSet([],files,english).warnings,{message:scan}];options.refresh(warnings);
      const control=options.controls.find(control=>control.kind==='group').controls[1],dialog=p.window.document.createElement('dialog');p.body.append(dialog);dialog.innerHTML=controlMarkup(control,{pages:{}});dialog.showModal();
      if(control.kind==='custom')control.bind(dialog.querySelector('[data-option-custom]'),{pages:{}},()=>{});
      for(const [language,strings] of [[locale,catalog],['en',{}],[locale,catalog]]){
        active=p.translate(language,strings);listeners.forEach(listener=>listener());p.translateTree(dialog);await p.window.happyDOM.waitUntilComplete();
        for(const [key,params] of expected)assert.ok(dialog.textContent.includes(expectedText(active,key,params)),`${language}: ${dialog.textContent}`);
        assert.equal(dialog.querySelectorAll('[data-language-warning]').length,expected.length);p.translateTree(dialog);
        assert.deepEqual([...dialog.querySelectorAll('[data-language-warning]')].map(node=>node.textContent),expected.map(([key,params])=>expectedText(active,key,params)));
      }
      options.refresh([]);assert.equal(dialog.querySelectorAll('[data-language-warning]').length,0);options.refresh(warnings);assert.equal(dialog.querySelectorAll('[data-language-warning]').length,expected.length);
      dialog.close();assert.equal(listeners.size,0);
    }finally{p.close();}
  }
});
test('the production Language page exposes its folder action only after a native desktop platform report',async()=>{
  const provider=`data:text/javascript;base64,${Buffer.from('export const availableLanguages=()=>[];export const systemLanguage=()=>"en";').toString('base64')}`;
  const {languageOptionsPage}=await import(await moduleUrl('language-options.ts',{"'./i18n'":JSON.stringify(provider)})),english=JSON.parse(await source('i18n/catalogs/en.json'));
  let callback;const visit=node=>{if(ts.isCallExpression(node)&&node.expression.getText(parsed)==='invoke'&&node.arguments[0]?.text==='runtime_platform')callback=node.parent.parent.arguments[0];ts.forEachChild(node,visit);};visit(parsed);assert.ok(callback);
  const js=ts.transpileModule(`return (${callback.getText(parsed)});`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  for(const [desktop,family] of [[true,'android'],[true,'ios'],[true,'desktop'],[false,'desktop'],[true,undefined]]){
    const p=page(),calls=[];
    try{
      const languagePage=rendererVariable(parsed,'languagePage',{desktop,ENGLISH:english,languageOptionsPage,onLanguageChange:()=>()=>{},invoke:async(command,args)=>{calls.push([command,args]);}});
      if(family&&desktop){
        const values={languagePage,render:()=>{},syncNativeAppearance:()=>{},optionsReady:Promise.resolve(),profileWorkspaceReady:Promise.resolve(false),fileOps:{initialize:async()=>{}},run:async()=>{}};
        await new Function(...Object.keys(values),`let runtimePlatform={},nativeAppearanceReady=false,directoriesReady=Promise.resolve();${js}`)(...Object.values(values))({family,folder_picker_supported:family==='desktop',document_access_supported:family!=='desktop'});
      }
      p.body.innerHTML=languagePage.controls.map(controlMarkup).join('');p.translate('de');
      const button=p.body.querySelector('[data-option-button="open-languages-folder"]'),supported=desktop&&family==='desktop';
      assert.equal(!!button,supported,`${desktop?'native':'browser'} / ${family??'unknown'}`);
      assert.ok(p.body.querySelector('[data-option="pages.language.language"]'));
      if(supported){assert.equal(button.textContent,new Translator('de',catalogs.de).t('Open languages folder'));const action=languagePage.controls.find(control=>control.kind==='group').controls.find(control=>control.id==='open-languages-folder');await action.run();assert.equal(calls[0][0],'open_languages_folder');assert.equal(JSON.parse(calls[0][1].template)['@meta'].name,'');}
      else assert.deepEqual(calls,[]);
    }finally{p.close();}
  }
});
test('an unavailable language remains an editable preference through the real Options save path',async()=>{
  const languages=[{locale:'en',name:'English',source:'bundled',catalog:{}},{locale:'pt-PT',name:'Português',source:'user',file:'pt-PT.json',catalog:{}}];
  const provider=`data:text/javascript;base64,${Buffer.from(`export const availableLanguages=()=>${JSON.stringify(languages)};export const systemLanguage=()=> 'pt-PT';`).toString('base64')}`;
  const {languageOptionsPage}=await import(await moduleUrl('language-options.ts',{"'./i18n'":JSON.stringify(provider)}));
  registerOptionsPage(languageOptionsPage({}));
  for(const stored of ['pt-BR','SYSTEM'])for(const chooseEnglish of [false,true]){
    const p=page(),globals=Object.fromEntries(['CSS','getComputedStyle'].map(name=>[name,Object.getOwnPropertyDescriptor(globalThis,name)]));
    Object.defineProperty(globalThis,'CSS',{configurable:true,value:p.window.CSS});Object.defineProperty(globalThis,'getComputedStyle',{configurable:true,value:p.window.getComputedStyle.bind(p.window)});
    try{
      const settings=defaultSettings();settings.pages.language={language:stored};const response={settings,defaults:defaultSettings(),warning:null};let saved;
      const ui=new OptionsUI({load:async()=>response,save:async settings=>{saved=structuredClone(settings);return{...response,settings};},appearance:{get:()=> 'system',set:()=>{}},applied:()=>{}});
      await ui.open('language');const t=p.translate('de'),select=p.body.querySelector('[data-option="pages.language.language"]');
      assert.equal(select.value,stored);
      assert.equal(select.selectedOptions[0].textContent,t.t('Unavailable: {name} (using English)',{name:stored}));
      if(chooseEnglish){select.value='en';select.dispatchEvent(new p.window.Event('change',{bubbles:true}));assert.equal(p.body.querySelector('[data-option="pages.language.language"]').value,'en');}
      p.body.querySelector('#options-ok').click();await p.window.happyDOM.waitUntilComplete();
      assert.equal(saved.pages.language.language,chooseEnglish?'en':stored);assert.equal(ui.settings.pages.language.language,chooseEnglish?'en':stored);
    }finally{for(const [name,descriptor] of Object.entries(globals))if(descriptor)Object.defineProperty(globalThis,name,descriptor);else delete globalThis[name];p.close();}
  }
});
test('toolbar hints preserve source names independently of renamed labels and translate once through hover status',async()=>{
  const toolKey='Tool: start {name} for the selected files',actionKey='Action group: apply {name} to the selected files';
  const custom={Help:'Assistance',Title:'Heading',Ready:'Help',[toolKey]:actionKey,[actionKey]:'Use {name}','Select files first.':'Choose files.','Save pending changes first.':'Save first.'};
  const functions=['button','customToolbarMarkup','bindToolbarHints'].map(name=>parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name).getText(parsed)).join('\n');
  const js=ts.transpileModule(functions,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const disabled of [false,true]){
    const p=page(),config={shown:true,items:[{id:'tool:Help',visible:true},{id:'action:Title',label:'Help',visible:true},{id:'save',label:'Help',visible:true},{id:'tool:My  Files\nTitle',visible:true}]};let saved;
    try{
      const t=new Translator(locale,catalog),expected=id=>id.startsWith('tool:')?t.t(toolKey,{name:id.slice(5)}):id.startsWith('action:')?t.t(actionKey,{name:id.slice(7)}):t.translate(itemHint(id));
      customizeToolbar({config:()=>config,save:async value=>{saved=structuredClone(value);},mac:()=>false,custom:{choices:()=>[]}});p.translate(locale,catalog);
      const check=items=>[...p.body.querySelectorAll('.toolbar-items [data-item]')].forEach((row,i)=>{assert.equal(row.querySelector('span').textContent,itemLabel(items[i]));assert.equal(row.querySelector('small').textContent,expected(items[i].id));});check(config.items);
      p.body.querySelector('[data-item="1"]').click();p.body.querySelector('[data-tb="edit"]').click();const input=p.body.querySelector('.toolbar-rename');input.value='Other';input.dispatchEvent(new p.window.KeyboardEvent('keydown',{key:'Enter',bubbles:true}));p.translateTree(p.body);
      const items=config.items.map((item,i)=>i===1?{...item,label:'Other'}:item);check(items);
      p.body.querySelector('#toolbar-ok').click();await p.window.happyDOM.waitUntilComplete();assert.deepEqual(saved.items,items);assert.equal(config.items[1].label,'Help');
      const values={esc,itemHint,itemLabel,userLabel,visibleItems,isSeparator,isSpacer,TOOLBAR_COMMANDS,customToolbar:saved,customItemReason:()=>disabled?'Select files first.':'',enabled:()=>!disabled,operationReason:()=>disabled?'Save pending changes first.':'',glyphs:{},translateNow:p.translateTree};
      const {render,bind}=new Function(...Object.keys(values),`${js};return{render:customToolbarMarkup,bind:bindToolbarHints};`)(...Object.values(values));
      p.body.innerHTML=`${render()}<span class="status-message">Ready</span>`;p.translate(locale,catalog);bind();
      const status=p.body.querySelector('.status-message'),before=status.textContent,toolbar=p.body.querySelector('.toolbar');
      for(const [i,button] of [...toolbar.querySelectorAll('button')].entries()){
        const item=items[i],reason=disabled?t.t(item.id==='save'?'Save pending changes first.':'Select files first.'):'';
        assert.equal(button.dataset.hint,expected(item.id));assert.equal(button.title,reason||itemLabel(item));
        button.dispatchEvent(new p.window.Event('pointerover',{bubbles:true}));p.translateTree(status);await p.window.happyDOM.waitUntilComplete();assert.equal(status.textContent,expected(item.id)+(reason?` — ${reason}`:''));
        toolbar.dispatchEvent(new p.window.Event('pointerleave'));p.translateTree(status);assert.equal(status.textContent,before);
      }
      p.translate('en');assert.equal(status.textContent,'Ready');assert.equal(toolbar.querySelector('button').dataset.hint,'Tool: start Help for the selected files');
    }finally{p.close();}
  }
});
test('preset removal preserves authored option names and values through the real groups save path',async()=>{
  const custom={Help:'Assistance',Title:'Heading',PRESET:'Preset name','Remove preset':'Delete preset','Remove selection preset':'Delete selection preset','Choose a preset…':'Pick a preset…'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page(),names=['Help','Title','MiXeD < & >'];let saved;
    try{
      const original={groups:[],presets:names.map(name=>({name,enabled_groups:[]}))};
      const ui=new ActionUI({load:async()=>original,save:async value=>{saved=structuredClone(value);},canExecute:()=>false,selectedCount:()=>1,changed:()=>{}});
      await ui.groupsDialog();const parent=p.body.querySelector('.action-groups-dialog'),t=p.translate(locale,catalog);
      const mainOptions=()=>[...parent.querySelector('#group-preset').options];
      assert.deepEqual(mainOptions().slice(1).map(option=>[option.value,option.textContent]),names.map(name=>[name,name]));assert.equal(mainOptions()[0].textContent,t.t('Choose a preset…'));
      parent.querySelector('[data-command="remove-preset"]').click();p.translateTree(p.body);
      let child=[...p.body.querySelectorAll('dialog')].at(-1),select=child.querySelector('[data-key="preset"]');
      assert.deepEqual([...select.options].map(option=>[option.value,option.textContent]),names.map(name=>[name,name]));
      assert.equal(child.querySelector('header').textContent,t.t('Remove selection preset'));assert.equal(child.querySelector('[data-remove]').textContent,t.t('Remove preset'));
      child.querySelector('[data-close]').click();parent.querySelector('[data-command="remove-preset"]').click();p.translateTree(p.body);
      child=[...p.body.querySelectorAll('dialog')].at(-1);select=child.querySelector('[data-key="preset"]');select.value='Title';child.querySelector('[data-remove]').click();p.translateTree(parent);
      assert.deepEqual(mainOptions().slice(1).map(option=>option.textContent),['Help','MiXeD < & >']);
      parent.querySelector('#save-groups').click();await p.window.happyDOM.waitUntilComplete();
      assert.deepEqual(saved.presets.map(preset=>preset.name),['Help','MiXeD < & >']);assert.deepEqual(original.presets.map(preset=>preset.name),names);
    }finally{p.close();}
  }
});
test('tool-name edits toggle authored names and generated fallbacks in both real Options selectors',async()=>{
  registerOptionsPage(toolsOptionsPage({pickProgram:async()=>null}));
  const custom={Help:'Assistance',Title:'Heading','Tool {i}':'Utility {i}',New:'Create','No action':'Do nothing'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page(),globals=Object.fromEntries(['CSS','getComputedStyle'].map(name=>[name,Object.getOwnPropertyDescriptor(globalThis,name)]));
    Object.defineProperty(globalThis,'CSS',{configurable:true,value:p.window.CSS});Object.defineProperty(globalThis,'getComputedStyle',{configurable:true,value:p.window.getComputedStyle.bind(p.window)});
    try{
      const settings=defaultSettings();settings.pages.tools={tools:[{name:'',path:'/bin/tool',parameters:'',for_each_file:false},{name:'Title',path:'/bin/other',parameters:'',for_each_file:false}],default_tool:0};
      const response={settings,defaults:defaultSettings(),warning:null};let saved;
      const ui=new OptionsUI({load:async()=>response,save:async value=>{saved=structuredClone(value);return{...response,settings:value};},appearance:{get:()=> 'system',set:()=>{}},applied:()=>{}});
      await ui.open('tools');const t=p.translate(locale,catalog);
      const check=name=>{for(const selector of ['#tools-list','#tool-default']){const select=p.body.querySelector(selector),option=select.querySelector('option[value="0"]');assert.equal(option.textContent,name||t.t('Tool {i}',{i:1}));assert.equal(option.getAttribute('translate'),name?'no':null);assert.equal(select.value,'0');}assert.equal(p.body.querySelector('#tool-name').value,name);assert.equal(p.body.querySelector('#tool-new').textContent,t.t('New'));};
      check('');
      for(const name of ['Help','Title','Tool 1','','Help']){
        const input=p.body.querySelector('#tool-name');input.value=name;input.dispatchEvent(new p.window.Event('input',{bubbles:true}));await p.window.happyDOM.waitUntilComplete();check(name);
        const select=p.body.querySelector('#tools-list');select.value='1';select.dispatchEvent(new p.window.Event('change',{bubbles:true}));const next=p.body.querySelector('#tools-list');next.value='0';next.dispatchEvent(new p.window.Event('change',{bubbles:true}));p.translateTree(p.body);check(name);
      }
      p.body.querySelector('#options-ok').click();await p.window.happyDOM.waitUntilComplete();
      assert.equal(saved.pages.tools.tools[0].name,'Help');assert.equal(saved.pages.tools.tools[1].name,'Title');assert.equal(saved.pages.tools.default_tool,0);assert.equal(settings.pages.tools.tools[0].name,'');
    }finally{for(const [name,descriptor] of Object.entries(globals))if(descriptor)Object.defineProperty(globalThis,name,descriptor);else delete globalThis[name];p.close();}
  }
});
test('freedb server names keep their authored spelling through empty/nonempty edits and redraws',async()=>{
  const custom={Help:'Assistance',Title:'Name','Unnamed server':'No display name',New:'Create',Delete:'Remove','freedb server':'CDDB endpoint'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page(),t=new Translator(locale,catalog);
    try{
      const server={name:'',protocol:'https',address:'fixture.example',path:'/',port:''},draft={pages:{'tag-sources-freedb':{...server,servers:[server],server:0}}};
      const options=tagSourcesOptionsPages(async()=>null).find(page=>page.id==='tag-sources-freedb');
      const control=options.controls.flatMap(control=>control.controls??[control]).find(control=>control.id==='freedb-servers');
      const draw=()=>{p.body.innerHTML=control.render(draft);control.bind(p.body,draft,draw);p.translate(locale,catalog);};draw();
      assert.equal(p.body.querySelector('#freedb-server option').textContent,t.t('Unnamed server'));
      for(const name of ['Help','Title','Unnamed server','','Help','']){
        const input=p.body.querySelector('#freedb-server-name');input.value=name;input.dispatchEvent(new p.window.Event('input',{bubbles:true}));
        await p.window.happyDOM.waitUntilComplete();
        const select=p.body.querySelector('#freedb-server'),option=select.options[0];
        assert.equal(option.textContent,name||t.t('Unnamed server'));assert.equal(option.getAttribute('translate'),name?'no':null);
        assert.equal(input.value,name);assert.equal(select.value,'0');assert.equal(draft.pages['tag-sources-freedb'].servers[0].name,name);
        assert.equal(p.body.querySelector('#freedb-server-new').textContent,t.t('New'));
        draw();assert.equal(p.body.querySelector('#freedb-server option').textContent,name||t.t('Unnamed server'));
      }
    }finally{p.close();}
  }
});
test('real library progress localizes with OS-formatted Unicode and grouped counts',async()=>{
  const original=Number.prototype.toLocaleString,key='Processing files … {done} of {total}';
  for(const osLocale of ['ar-EG','fa-IR','bn-BD','hi-IN-u-nu-deva','de-CH','en-IN','zh-Hans-u-nu-hanidec'])for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',{[key]:'Progress: {done} / {total}'}]]){
    const p=page(),seen=[],expected=[];
    try{
      Number.prototype.toLocaleString=function(language,options){return original.call(this,language??osLocale,options);};
      const status={created:false,files:0,last_hits:0,last_stored:0},draft={pages:{library:{}}},t=new Translator(locale,catalog);
      const options=libraryOptionsPage({status:async()=>status,cleanup:async()=>({checked:1,removed:0,cancelled:false}),progress:async(_job,update)=>{
        for(const [done,total] of [[1,2],[1000002,2000003]]){
          update(done,total);p.translateTree(p.body);
          seen.push(p.body.querySelector('#library-status').textContent);expected.push(t.t(key,{done:original.call(done,osLocale),total:original.call(total,osLocale)}));
        }
        return()=>{};
      }}),control=options.controls.flatMap(control=>control.controls??[control]).find(control=>control.id==='library-maintenance');
      p.body.innerHTML=control.render(draft);control.bind(p.body,draft,()=>{});p.translate(locale,catalog);p.body.querySelector('#library-cleanup').click();
      await p.window.happyDOM.waitUntilComplete();assert.equal(seen.length,2);assert.deepEqual(seen,expected,`${osLocale} → ${locale}`);
    }finally{Number.prototype.toLocaleString=original;p.close();}
  }
});
test('production backend error frames translate wrapped diagnostics and preserve paths, commands and replies',()=>{
  const cases=[['The destination is not a folder',{}],['Cannot open the destination folder: {e}',{e:'Permission denied (os error 13)'}],['Invalid DSF file: {what}',{what:'the file is truncated'}],['The MP4 structure cannot be rewritten safely: {error}',{error:'the file ends inside a box header'}],['Library database: {e}',{e:'disk full'}],['ASF {value} object limits exceeded',{value:'Content'}],['{name} did not respond in time. Check your connection and try again.',{name:'MusicBrainz'}],['The freedb server answered: {code} {value}',{code:400,value:'Help'}],['Invalid chapter {value}: use HH:MM:SS.mmm followed by the title',{value:'"Help  Title"'}],['{field} is stored once for {value}, not per chapter; give its chapter rows the same value',{field:'TITLE',value:'Help.m4a'}]];
  const extra=['{name} is not a known command','this block has no {name}'],custom={...Object.fromEntries([...cases.map(([key])=>key),...extra].map(key=>[key,`Diagnostic: ${[...new Set(key.match(/\{\w+\}/g)??[])].join(' / ')}`])),Help:'Assistance',TITLE:'Wrong field',MusicBrainz:'Wrong provider',Content:'Wrong header',Endif:'Wrong command','Help.m4a':'Wrong file','"Help  Title"':'Wrong chapter'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const [key,params] of cases){
    const p=page();try{
      const t=new Translator(locale,catalog),error=new Translator('en').t(key,params),e={path:'Help',error},expected={...params};
      for(const role of ['what','error'])if(role in expected)expected[role]=t.translate(expected[role]);
      p.body.innerHTML=mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path,publicMessage:value=>value});p.translate(locale,catalog);
      assert.equal(p.body.querySelector('strong').textContent,'Help:');assert.equal(p.body.querySelector('p').textContent,`Help: ${t.t(key,expected)}`);
    }finally{p.close();}
  }
});
test('real format and export diagnostics translate function prose while tokens and error paths stay raw',async()=>{
  const cases=JSON.parse(await readFile(new URL('./fixtures/scripting-diagnostics.json',import.meta.url),'utf8'));
  const arity='${name} expects {minimum}..={maximum} arguments, received {count}',keys=cases.map(item=>item.error.startsWith('$upper expects')?arity:item.error),custom={...Object.fromEntries(keys.map(key=>[key,`Diagnostic: ${key}`])),upper:'Wrong function token',Help:'Wrong file'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();try{
      for(const [index,item] of cases.entries()){
        const t=new Translator(locale,catalog),e={path:'Help',error:item.error},expected=t.t(keys[index],keys[index]===arity?{name:'upper',minimum:1,maximum:1,count:2}:{});
        assert.notEqual(expected,item.error);p.body.innerHTML=mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path,publicMessage:value=>value});p.translate(locale,catalog);
        const shown=p.body.querySelector('p');assert.equal(shown.textContent,`Help: ${expected}`,`${locale} ${item.input}`);assert.deepEqual(shown.textContent.match(/\$[A-Za-z_]\w*(?:\([A-Z]+\))?/g),item.error.match(/\$[A-Za-z_]\w*(?:\([A-Z]+\))?/g));
        p.translateTree(p.body);assert.equal(shown.textContent,`Help: ${expected}`);p.translate('en');assert.equal(shown.textContent,`Help: ${item.error}`);
      }
    }finally{p.close();}
  }
});
test('web-script prefix diagnostics localize their scaffold without looking up authored or library payloads',async()=>{
  const fixtures=JSON.parse(await readFile(new URL('./fixtures/web-scripting-diagnostics.json',import.meta.url),'utf8'));
  const cases=[...fixtures.filter(item=>typeof item.error==='string').map(item=>item.error), '"Help/Title" is not a file name in the sources folder','[IndexFormat] has no %_url% field','[Encoding]=title is not one of utf-8, iso-8859-1, url, url-utf-8 or ansi',...['the regular expression is not valid: ','the regular expression failed: ','the input is not valid JSON: '].map(prefix=>prefix+'Title')];
  const extra={'no number at the current position: "{value}"':'No number: {value}','"{value}" was not found in the current line':'Missing in line: {value}','"{value}" is not a number':'Not numeric: {value}','"{name}" is not a file name in the sources folder':'Not a source file: {name}','[Encoding]={value} is not one of utf-8, iso-8859-1, url, url-utf-8 or ansi':'Invalid encoding: {value}','the regular expression is not valid: {value}':'Invalid regex: {value}','the regular expression failed: {value}':'Failed regex: {value}','the input is not valid JSON: {value}':'Invalid JSON: {value}'};
  const custom={...Object.fromEntries(cases.map(key=>[key,`Diagnostic: ${key}`])),...extra,Title:'Wrong payload',title:'Wrong encoding','Help/Title':'Wrong file'};
  // Remove exact dynamic examples so their raw capture roles, rather than an exact key, are exercised.
  for(const key of cases)if(key.includes('Title')||key.startsWith('[Encoding]'))delete custom[key];
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const error of cases){
    const p=page();try{
      const t=new Translator(locale,catalog),expected=t.translate(error),e={path:'Help',error};assert.notEqual(expected,error,`${locale} ${error}`);
      assert.ok(!expected.includes('Wrong'));if(error.includes('Title'))assert.ok(expected.includes('Title'));if(error.startsWith('[Encoding]'))assert.ok(expected.includes('title'));
      p.body.innerHTML=mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path,publicMessage:value=>value});p.translate(locale,catalog);const row=p.body.querySelector('p');assert.equal(row.textContent,`Help: ${expected}`);p.translateTree(p.body);assert.equal(row.textContent,`Help: ${expected}`);p.translate('en');assert.equal(row.textContent,`Help: ${error}`);
    }finally{p.close();}
  }
});
test('production save and restore conflict frames translate during diagnostics and preserve file identities',()=>{
  const messages=['The file changed during editing; reload it before saving','The file changed during restore; reload it and try again','The cue sheet changed during editing; reload it before saving','Tag disappeared during editing'];
  const custom={...Object.fromEntries(messages.map(key=>[key,'Conflict: reload now.'])),'Save tags':'Wrong path'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const error of messages){
    const p=page();try{
      const t=new Translator(locale,catalog),e={path:'Save tags',error};p.body.innerHTML=mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path,publicMessage:value=>value});p.translate(locale,catalog);const row=p.body.querySelector('p');assert.notEqual(t.t(error),error);assert.equal(row.textContent,`Save tags: ${t.t(error)}`);p.translateTree(p.body);assert.equal(row.textContent,`Save tags: ${t.t(error)}`);p.translate('en');assert.equal(row.textContent,`Save tags: ${error}`);
    }finally{p.close();}
  }
});
test('owned technical diagnostics translate in production error rows while their URI and parser tokens stay unchanged',async()=>{
  const cases=JSON.parse(await readFile(new URL('./fixtures/owned-backend-diagnostics.json',import.meta.url),'utf8')),custom={...Object.fromEntries(cases.map(item=>[item.error,`Diagnostic: ${item.error}`])),Help:'Wrong file'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const {error} of cases){
    const p=page();try{const t=new Translator(locale,catalog),e={path:'Help',error};p.body.innerHTML=mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path,publicMessage:value=>value});p.translate(locale,catalog);const row=p.body.querySelector('p');assert.notEqual(t.t(error),error);assert.equal(row.textContent,`Help: ${t.t(error)}`);assert.deepEqual(row.textContent.match(/URI|json_foreach(?:_end|_counter)?/g),error.match(/URI|json_foreach(?:_end|_counter)?/g));p.translateTree(p.body);assert.equal(row.textContent,`Help: ${t.t(error)}`);p.translate('en');assert.equal(row.textContent,`Help: ${error}`);}finally{p.close();}
  }
});
test('actual selection summary translates each technical component and preserves codec identities',()=>{
  const declaration=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='selectionInfoMarkup'),js=ts.transpileModule(declaration.getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  const custom={'{bitrate} kbps':'Rate {bitrate}','{sampleRate} Hz':'Frequency {sampleRate}','{channels} channel':'Mono {channels}','{channels} channels':'Channels {channels}',Help:'Wrong codec',Title:'Wrong format'};
  const cases=[{format:'Help',technical:{_CODEC:['Title']},bitrate:128,sample_rate:44100,channels:2},{format:'Help',bitrate:0,sample_rate:48000,channels:1},{format:'Title',technical:{_CODEC:['128 kbps']},bitrate:0,sample_rate:0,channels:0}];
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const values of cases){
    const p=page();try{
      const entry={...values,path:'/Help.mp3',duration:60,size:1048576},before=structuredClone(entry),selected=new Set([entry.path]),entries=new Map([[entry.path,entry]]),render=new Function('listSummary','technicalSummary','entries','selected','active','esc',`${js};return selectionInfoMarkup;`)(listSummary,technicalSummary,entries,selected,()=>entry,esc);
      p.body.innerHTML=render([entry],[entry]);p.translate(locale,catalog);const t=new Translator(locale,catalog),codec=entry.technical?._CODEC[0]||entry.format,labels=[entry.bitrate?t.t('{bitrate} kbps',{bitrate:entry.bitrate}):'',entry.sample_rate?t.t('{sampleRate} Hz',{sampleRate:entry.sample_rate.toLocaleString()}):'',entry.channels?t.t(entry.channels===1?'{channels} channel':'{channels} channels',{channels:entry.channels}):''].filter(Boolean),expected=[codec,...labels].filter(Boolean).join(' · '),technical=p.body.lastElementChild;
      assert.equal(technical.textContent,expected,`${locale} ${codec}`);p.translateTree(p.body);assert.equal(technical.textContent,expected);p.translate('en');assert.equal(technical.textContent,listSummary([entry],[entry],1,entry).at(-1));assert.deepEqual(entry,before);
      selected.clear();p.body.innerHTML=render([],[entry]);p.translate(locale,catalog);assert.equal(p.body.children.length,2);assert.ok(!p.body.textContent.includes(codec));
    }finally{p.close();}
  }
});
test('import companions validate associations and preserve untyped or foreign English before joining',()=>{
  const source='Action field must not be empty',custom={'Action {index}: {reason}':'Issue {index}: {reason}',[source]:'Help',Help:'Wrong second lookup',Title:'Wrong raw name','No column could be imported: {error}':'None: {error}'};
  const raw={issues:[{index:2,type_id:'Title',reason:source},{index:9,type_id:'Help',reason:source}]},metadata=[{index:0,cause:{kind:'owned_text',value:source}},{index:1,cause:{kind:'external',value:source}}];
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const t=new Translator(locale,catalog),shown=mtaImportFailure({...raw,issue_diagnostics:metadata}).text((key,params)=>t.t(key,params),text=>t.translate(text));
    assert.equal(shown,t.t('Action {index}: {reason}',{index:3,reason:t.t(source)})+'\n'+t.t('Action {index}: {reason}',{index:10,reason:source}));assert.ok(!shown.includes('Wrong'));
    for(const bad of [undefined,null,{},[],[{index:99,cause:metadata[0].cause}],[metadata[0],metadata[0]],[{index:0,cause:{kind:'unknown'}}],[{index:0,cause:{kind:'owned_text',value:'mismatch'}}]]){const failure=mtaImportFailure({...raw,issue_diagnostics:bad});assert.equal(failure.text((key,params)=>t.t(key,params),text=>t.translate(text)),failure.message);}
    const rawFailure='No column could be imported: Help: '+source;
    for(const error of [rawFailure,{message:rawFailure,import_failure:null},{message:rawFailure,import_failure:{kind:'owned_text',value:'mismatch'}}]){const failure=importFailure(error,true);assert.ok(failure);assert.equal(failure.text((key,params)=>t.t(key,params),text=>t.translate(text)),rawFailure);}
  }
  assert.deepEqual(raw,{issues:[{index:2,type_id:'Title',reason:source},{index:9,type_id:'Help',reason:source}]});
});

test('actual rejected column import renders retained causes in its dialog and global errors and releases resets',async()=>{
  const reason='the sort format string is not supported',names=['Help: the sort format string is not supported; Title','Title: Help\nName'];
  const causes=names.map(value=>({kind:'owned',key:'{value}: {error}',params:{value},children:{error:{kind:'owned',key:reason,params:{},children:{}}}}));
  const cause={kind:'owned',key:'No column could be imported: {error}',params:{},children:{error:{kind:'joined',parts:causes,separator:'; '}}},wire={message:causeText(cause),import_failure:cause},original=structuredClone(wire);
  const declaration=parsed.statements.filter(ts.isVariableStatement).flatMap(node=>[...node.declarationList.declarations]).find(node=>node.name.getText(parsed)==='columnUI');
  const js=ts.transpileModule(`let status='',errors=[];const ${declaration.getText(parsed)};return{host:columnUI,errors:()=>errors};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  const custom={[reason]:'Help',Help:'Wrong second lookup',Title:'Wrong raw name','No column could be imported: {error}':'None: {error}'};
  const p=page();let active=new Translator('en');const listeners=new Set();try{
    const values={ColumnImportStatus,importFailure,BatchFailure,ColumnUI:class{constructor(host){return host;}},translateNow:p.translateTree,columnLayout:defaultLayout(),applyColumns:async()=>assert.fail('Rejected import cannot apply'),open:async()=>'/tmp/Help\nTitle.ini',invoke:async()=>{throw wire;},normalizeLayout:normalizeColumns,basename:path=>path.split('/').at(-1),active:()=>undefined,desktop:false,esc,render:()=>{},t:(key,params)=>active.t(key,params),translate:text=>active.translate(text),onLanguageChange:listener=>{listeners.add(listener);return()=>listeners.delete(listener);}};
    const model=new Function(...Object.keys(values),js)(...Object.values(values)),ui=new ColumnUI(model.host);ui.customize();const dialog=p.body.querySelector('dialog'),settle=async()=>{for(let i=0;i<4;i++)await new Promise(resolve=>setTimeout(resolve,0));};dialog.querySelector('#column-import').click();await settle();
    for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom],['en',{}],['de',catalogs.de]]){active=p.translate(locale,catalog);listeners.forEach(listener=>listener());p.translateTree(dialog);const expected=causeText(cause,(key,params)=>active.t(key,params),text=>active.translate(text));assert.equal(dialog.querySelector('.dialog-error').textContent,expected);assert.equal(dialog.querySelector('.dialog-error').getAttribute('translate'),'no');assert.ok(!expected.includes('Wrong'));for(const name of names)assert.ok(expected.includes(name));}
    model.host.report(importFailure(wire,true));const e=model.errors()[0],section=p.window.document.createElement('section');p.body.append(section);section.innerHTML=mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path,publicMessage:value=>value,t:(key,params)=>active.t(key,params),translate:text=>active.translate(text)});p.translateTree(section);assert.equal(section.querySelector('span').textContent,causeText(cause,(key,params)=>active.t(key,params),text=>active.translate(text)));assert.equal(section.querySelector('span').getAttribute('translate'),'no');
    assert.deepEqual(wire,original);assert.equal(dialog.querySelectorAll('.column-list li').length,defaultLayout().columns.length);dialog.querySelector('#column-defaults').click();assert.equal(dialog.querySelector('.dialog-error'),null);assert.equal(listeners.size,0);active=p.translate('nl');listeners.forEach(listener=>listener());assert.equal(dialog.querySelector('.dialog-error'),null);dialog.querySelector('#column-import').click();await settle();assert.equal(listeners.size,1);dialog.close();assert.equal(listeners.size,0);
  }finally{p.close();}
});

test('actual column Options import translates each skipped reason while retaining raw column names and source status',async()=>{
  const reason='the sort format string is not supported',name='Help: the sort format string is not supported; Title',path='/tmp/Help\nTitle.ini';
  const declaration=parsed.statements.filter(ts.isVariableStatement).flatMap(node=>[...node.declarationList.declarations]).find(node=>node.name.getText(parsed)==='columnUI');
  const statusExpression=main.match(/<span class="status-message" role="status">\$\{([^}]+)\}<\/span>/)[1];
  const hintsFn=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='bindToolbarHints');
  const historyFn=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='refreshHistory');
  const statusFn=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='statusMarkup');
  const js=ts.transpileModule(`let status='',history={};const ${declaration.getText(parsed)};${statusFn?.getText(parsed)||''};${hintsFn.getText(parsed)};${historyFn.getText(parsed)};return{host:columnUI,status:()=>status,set:value=>status=value,refresh:refreshHistory,bindHints:bindToolbarHints,render:()=>${statusExpression}};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  const custom={[reason]:'Help',Help:'Wrong second lookup',Title:'Wrong identity',[name]:'Wrong raw name','Imported {length} Mp3tag column from {path}; skipped {length2}: {join}':'Result [{path}] {length}/{length2}: {join}'};
  for(const locale of ['de','nl']){const p=page();let active;const listeners=new Set(),calls=[];try{
    const catalog={...catalogs[locale],[reason]:locale==='de'?'der Sortierformatstring wird nicht unterstützt':'de sorteernotatie wordt niet ondersteund',[name]:'Wrong raw name','Help\nTitle.ini':'Wrong raw path'};active=p.translate(locale,catalog);
    const layout=defaultLayout();layout.columns=layout.columns.slice(0,1);const imported={layout,skipped:[`${name}: ${reason}`],skipped_diagnostics:[{index:0,cause:{kind:'owned',key:'{value}: {error}',params:{value:name},children:{error:{kind:'owned',key:reason,params:{},children:{}}}}}]},original=structuredClone(imported);
    const values={ColumnImportStatus,importFailure,BatchFailure,ColumnUI:class{constructor(host){return host;}},translateNow:p.translateTree,itemHint:()=> 'Options saved',customItemReason:()=>'',operationReason:()=>'',columnLayout:defaultLayout(),applyColumns:async()=>assert.fail('Import changes only the draft'),open:async()=>path,invoke:async(command)=>{calls.push(command);if(command==='import_mp3tag_columns')return imported;if(command==='history_state')return{notice:'Select files first.'};throw new Error(command);},normalizeLayout:normalizeColumns,basename:path=>path.split('/').at(-1),active:()=>undefined,desktop:true,esc,render:()=>{},t:(key,params)=>active.t(key,params),translate:text=>active.translate(text),onLanguageChange:listener=>{listeners.add(listener);return()=>listeners.delete(listener);}};
    const model=new Function(...Object.keys(values),js)(...Object.values(values)),ui=new ColumnUI(model.host);ui.customize();const dialog=p.body.querySelector('dialog');dialog.querySelector('#column-import').click();for(let i=0;i<4;i++)await new Promise(resolve=>setTimeout(resolve,0));assert.ok(calls.includes('import_mp3tag_columns'));
    const notice=p.window.document.createElement('section');notice.className='status-message';p.body.append(notice);const toolbar=p.window.document.createElement('div');toolbar.className='toolbar';toolbar.innerHTML='<button data-hint="ignored" data-op="save">Save tags</button>';p.body.append(toolbar);model.bindHints();
    const source=String(model.status());
    for(const [language,strings] of [[locale,catalog],['en',custom],['en',{}],['nl',catalogs.nl],['de',catalogs.de]]){
      active=p.translate(language,strings);listeners.forEach(listener=>listener());notice.innerHTML=model.render();p.translateTree(notice);
      const expected=active.t('Imported {length} Mp3tag column from {path}; skipped {length2}: {join}',{length:1,path:'Help\nTitle.ini',length2:1,join:`${name}: ${active.t(reason)}`});
      assert.equal(notice.textContent,expected);assert.equal(notice.querySelector('span').getAttribute('translate'),'no');assert.ok(!notice.textContent.includes('Wrong'));assert.equal(String(model.status()),source);
      toolbar.querySelector('button').dispatchEvent(new p.window.Event('pointerover',{bubbles:true}));assert.equal(notice.textContent,active.t('Options saved'));toolbar.dispatchEvent(new p.window.Event('pointerleave'));p.translateTree(notice);assert.equal(notice.textContent,expected);
    }
    await model.refresh();notice.innerHTML=model.render();p.translateTree(notice);assert.ok(notice.textContent.endsWith('. '+active.t('Select files first.')));
    model.set('Options saved');notice.innerHTML=model.render();p.translateTree(notice);assert.equal(notice.textContent,active.t('Options saved'));assert.equal(notice.querySelector('[translate="no"]'),null);
    assert.deepEqual(imported,original);assert.equal(dialog.querySelectorAll('.column-list li').length,1);dialog.close();assert.equal(listeners.size,0);
  }finally{p.close();}}
});

test('tag panel Mp3tag import translates each skipped reason while the authored entry stays verbatim',()=>{
  const wrap=(value,error)=>({kind:'owned',key:'{value}: {error}',params:{value},children:{error}});
  const owned=(key,params={})=>({kind:'owned',key,params,children:{}});
  const notes={
    skipped:['Field 7 (Hilfe): TITLE is already on the panel','Field 8 (Mood): no field name','Field 9 (Raw): untouched producer line'],
    skipped_diagnostics:[
      {index:0,cause:wrap('Field 7 (Hilfe)',owned('{id} is already on the panel',{id:'TITLE'}))},
      {index:1,cause:wrap('Field 8 (Mood)',owned('no field name'))},
    ],
  };
  const original=structuredClone(notes);
  const expected={
    de:['Field 7 (Hilfe): TITLE ist bereits im Tag-Panel vorhanden','Field 8 (Mood): kein Feldname','Field 9 (Raw): untouched producer line'],
    nl:['Field 7 (Hilfe): TITLE staat al op het tagpaneel','Field 8 (Mood): geen veldnaam','Field 9 (Raw): untouched producer line'],
  };
  for(const locale of ['de','nl']){const p=page();try{
    const active=p.translate(locale,catalogs[locale]);
    assert.deepEqual(skippedLines(notes,(key,params)=>active.t(key,params),text=>active.translate(text)),expected[locale]);
  }finally{p.close();}}
  const english=new Translator('en');
  assert.deepEqual(skippedLines(notes,(key,params)=>english.t(key,params),text=>english.translate(text)),notes.skipped,'English renders the legacy lines');
  // A companion whose rendering differs from the line, or one that repeats an index, is not trusted.
  const forged={skipped:['Field 8 (Mood): another reason','Field 8 (Mood): no field name'],skipped_diagnostics:[notes.skipped_diagnostics[1],notes.skipped_diagnostics[1],{index:1,cause:owned('no field name')}]};
  const p=page();try{
    const active=p.translate('de',catalogs.de);
    assert.deepEqual(skippedLines(forged,(key,params)=>active.t(key,params),text=>active.translate(text)),forged.skipped);
  }finally{p.close();}
  assert.deepEqual(notes,original,'translation does not mutate the native result');
});

test('actual MTA import renders each unresolved issue once and keeps its source identity through live languages',async()=>{
  const fixture=JSON.parse(await readFile(new URL('./fixtures/import-diagnostics.json',import.meta.url),'utf8')).mta;
  const custom={'Action {index}: {reason}':'Issue {index}: {reason}','Numeric action type {typeId} has no known settings schema':'Unknown [{typeId}]','Action field must not be empty':'Help',Help:'Wrong second lookup',Title:'Wrong identity',[fixture.name]:'Wrong group','/tmp/Help\nTitle.mta':'Wrong source'};
  for(const locale of ['de','nl']){const p=page();let active=new Translator('en');const listeners=new Set(),calls=[];try{
    const imported={group:{name:fixture.name,enabled:false,actions:fixture.issues.map(issue=>({type:'unresolved_mta',type_id:issue.type_id,properties:{T:issue.type_id},reason:issue.reason}))},issues:structuredClone(fixture.issues),issue_diagnostics:fixture.issues.map((issue,index)=>({index,cause:{kind:'owned_text',value:issue.reason}}))},original=structuredClone(imported);
    const invoke=async(command)=>{calls.push(command);if(command==='load_action_library')return{groups:[],presets:[]};if(command==='import_action_group')return imported;throw new Error(command);};
    const host=rendererVariable(parsed,'actionUI',{ActionUI:class{constructor(host){return host;}},t:(key,params)=>active.t(key,params),translate:text=>active.translate(text),onLanguageChange:listener=>{listeners.add(listener);return()=>listeners.delete(listener);},translateNow:()=>{},executeGroups:async()=>assert.fail('Import only changes the group draft'),invoke,open:async()=>'/tmp/Help\nTitle.mta',enabled:()=>true,operationReason:()=>'',selected:new Set(['/Music/Help.wav']),paths:()=>['/Music/Help.wav'],merged:()=>({TITLE:['Title']}),entries:new Map(),render:()=>{}});
    const ui=new ActionUI(host);await ui.initialize();await ui.groupsDialog();const dialog=p.body.querySelector('dialog');dialog.querySelector('[data-command="import"]').click();for(let i=0;i<3;i++)await new Promise(resolve=>setTimeout(resolve,0));const child=p.body.querySelectorAll('dialog')[1];assert.ok(child);active=p.translate('en',custom);p.translateTree(child);assert.equal(child.querySelector('.dialog-content > p').textContent,'/tmp/Help\nTitle.mta');child.querySelector('[data-import]').click();for(let i=0;i<3;i++)await new Promise(resolve=>setTimeout(resolve,0));assert.ok(calls.includes('import_action_group'));
    for(const [language,catalog] of [[locale,catalogs[locale]],['en',custom],['en',{}],['nl',catalogs.nl],['de',catalogs.de]]){
      active=p.translate(language,catalog);listeners.forEach(listener=>listener());p.translateTree(dialog);
      const expected=fixture.issues.map(issue=>active.t('Action {index}: {reason}',{index:issue.index+1,reason:issue.index===1?active.t(issue.reason):active.t('Numeric action type {typeId} has no known settings schema',{typeId:issue.type_id})})).join('\n');
      assert.equal(dialog.querySelector('.dialog-error').textContent,expected);assert.equal(dialog.querySelector('.dialog-error').getAttribute('translate'),'no');assert.ok(!dialog.querySelector('.dialog-error').textContent.includes('Wrong'));assert.deepEqual(imported,original);assert.equal(dialog.querySelector('.group-row > span').textContent,fixture.name);
    }
    dialog.querySelector('[data-enable]').click();assert.equal(dialog.querySelector('.dialog-error'),null);assert.equal(listeners.size,0);active=p.translate('en',custom);listeners.forEach(listener=>listener());assert.equal(dialog.querySelector('.dialog-error'),null);dialog.close();assert.equal(listeners.size,0);assert.ok(!calls.includes('execute_action_groups'));assert.ok(!calls.includes('save_action_library'));
  }finally{p.close();}}
});

test('actual source Apply review keeps typed batch failures through the real main host and live language/reset lifecycle',async()=>{
  const custom={'The cover image could not be saved: {error}':'Cover failed: {error}','Cannot write {path}: {error}':'Writing [{path}]: {error}','Select files first.':'Help',Help:'Wrong second lookup',Title:'Wrong raw name',Back:'Return'};
  for(const item of batchDiagnosticCases.filter(item=>item.diagnostic.kind==='source_cover_failed')){const p=page();let active=new Translator('en');const listeners=new Set(),calls=[];try{
    const folder=item.diagnostic.cause.kind==='external'?'Cancelled: Help; Title':'Help; Title',path='/Music/Help.wav',entry={path,filename:'Help.wav',duration:60,fields:{TITLE:['Help'],ARTIST:['Title'],ALBUM:['Album'],TRACK:['1'],DISCNUMBER:['1']}};
    const invoke=async(command,args)=>{calls.push(command);if(command==='web_source_search')return{direct:true,columns:[],results:[{url:'https://example.invalid/release'}]};if(command==='web_source_release')return{provider:'web',id:'fixture',url:'',artist:'Title',title:'Album',fields:{ALBUM:['Album']},tracks:[{position:'1',disc:1,number:1,title:'Help',artist:'Title',length:60,fields:{TITLE:['Help'],ARTIST:['Title'],TRACK:['1']}}],covers:[],more_covers:false};if(command==='tag_source_apply'){assert.equal(args.items[0].path,path);return normalizeBatchDiagnostics({entries:[],errors:[{path:'/Music/'+folder,error:item.source}],error_diagnostics:[{index:0,diagnostic:item.diagnostic}]},true);}throw new Error('Unexpected '+command);};
    const host=rendererVariable(parsed,'tagSourcesUI',{TagSourcesUI:class{constructor(host){return host;}},BatchFailure,webScriptErrorText,t:(key,params)=>active.t(key,params),translate:text=>active.translate(text),onLanguageChange:listener=>{listeners.add(listener);return()=>listeners.delete(listener);},invoke,paths:()=>[path],entries:new Map([[path,entry]]),displayName:path=>path.split('/').at(-1),field:(entry,key)=>entry.fields[key]?.[0]??'',tagSourceReason:()=>'',optionsUI:{page:()=>({})},columnSpecs:()=>[],run:async(_text,job)=>job(),accept:()=>{},reconcileWorkingEntries:async()=>{},publicMessage:error=>error instanceof Error?error.message:String(error)});
    const ui=new TagSourcesUI(host);await ui.open('web',{web:{id:'fixture',name:'Help',search_by:[]}});const dialog=p.body.querySelector('dialog'),settle=async()=>{for(let i=0;i<4;i++)await new Promise(resolve=>setTimeout(resolve,0));};dialog.querySelector('#ts-search').click();await settle();const apply=dialog.querySelector('#ts-apply');assert.ok(apply);assert.equal(apply.disabled,false);apply.click();await settle();assert.ok(calls.includes('tag_source_apply'));const error=dialog.querySelector('.dialog-error');assert.ok(error);assert.equal(error.getAttribute('translate'),'no');
    for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom],['en',{}],['de',catalogs.de]]){active=p.translate(locale,catalog);listeners.forEach(listener=>listener());p.translateTree(dialog);assert.equal(error.textContent,folder+': '+batchErrorText(item.diagnostic,(key,params)=>active.t(key,params),text=>active.translate(text)));assert.ok(!error.textContent.includes('Wrong'));if(locale!=='en'&&!(item.diagnostic.kind==='action_validation'&&item.diagnostic.cause.kind==='external'))assert.ok(!error.textContent.endsWith(item.source),'owned frame translates while raw context stays unchanged');assert.equal(dialog.querySelector('#ts-back').textContent,active.t('Back'));assert.equal(dialog.querySelector('#'+dialog.getAttribute('aria-labelledby')).textContent,'Help');}
    dialog.querySelector('#ts-back').click();assert.equal(dialog.querySelector('.dialog-error'),null);active=p.translate('en',custom);listeners.forEach(listener=>listener());assert.equal(dialog.querySelector('.dialog-error'),null);dialog.close();assert.equal(listeners.size,0);
  }finally{p.close();}}
});

test('actual ActionUI run, quick and checked-group dialogs retain typed failures and release reset listeners',async()=>{
  const custom={'{value} / action {value2}: {error}':'Action {value2} for [{value}]: {error}','{value}: {error}':'Group [{value}]: {error}','Select files first.':'Help',Help:'Wrong second lookup',Title:'Wrong raw name'};
  const declaration=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='executeGroups'),js=ts.transpileModule(declaration.getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  for(const item of batchDiagnosticCases.filter(item=>item.diagnostic.kind==='action'||item.diagnostic.kind==='action_validation'))for(const mode of ['run','checked','quick']){
    const p=page();let active=new Translator('en');const listeners=new Set(),calls=[];try{
      const group={name:item.diagnostic.group,enabled:true,actions:[{type:'format_value',field:'TITLE',template:'Help'},{type:'format_value',field:'TITLE',template:'Title'}]},library={groups:[group],presets:[]};
      const invoke=async(command,args)=>{calls.push(command);if(command==='load_action_library')return library;if(command==='execute_action_groups')return normalizeBatchDiagnostics({entries:[],errors:[{path:'/Music/Title.wav',error:item.source}],error_diagnostics:[{index:0,diagnostic:item.diagnostic}]},true);throw new Error('Unexpected '+command);};
      const execute=new Function('BatchFailure','enabled','operationReason','paths','run','invoke','columnSpecs','applyRenamed','messages',`let errors=[],lastActionGroups=[];function accept(batch){errors=batch.errors;} ${js};return{execute:executeGroups,state:()=>errors};`)(BatchFailure,()=>true,()=>'',()=>['/Music/Title.wav'],async(_text,job)=>job(),invoke,()=>[],()=>{},{enabled:()=>true});
      const host=rendererVariable(parsed,'actionUI',{ActionUI:class{constructor(host){return host;}},t:(key,params)=>active.t(key,params),translate:text=>active.translate(text),onLanguageChange:listener=>{listeners.add(listener);return()=>listeners.delete(listener);},translateNow:()=>{},invoke,executeGroups:execute.execute,enabled:()=>true,operationReason:()=>'',selected:new Set(['/Music/Title.wav']),paths:()=>['/Music/Title.wav'],merged:()=>({TITLE:['Title']}),entries:new Map(),render:()=>{}});
      const ui=new ActionUI(host);await ui.initialize();if(mode==='run')await ui.runOne(0);else{if(mode==='quick'){ui.quick('format_value');p.body.querySelector('[data-keep]').click();}else{await ui.groupsDialog();p.body.querySelector('#apply-groups').click();}for(let i=0;i<4;i++)await new Promise(resolve=>setTimeout(resolve,0));}
      assert.ok(calls.includes('execute_action_groups'));const dialog=p.body.querySelector('dialog'),error=dialog.querySelector('.dialog-error');assert.ok(error);assert.equal(error.getAttribute('translate'),'no');assert.deepEqual(execute.state()[0].diagnostic,item.diagnostic);
      for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom],['en',{}],['de',catalogs.de]]){active=p.translate(locale,catalog);listeners.forEach(listener=>listener());p.translateTree(dialog);const expected=new BatchFailure(execute.state(),'').text((key,params)=>active.t(key,params),text=>active.translate(text));assert.equal(error.textContent,expected);assert.ok(error.textContent.includes(item.diagnostic.group));assert.ok(!error.textContent.includes('Wrong'));if(locale!=='en'&&!(item.diagnostic.kind==='action_validation'&&item.diagnostic.cause.kind==='external'))assert.ok(!error.textContent.endsWith(item.source),'owned frame translates while raw context stays unchanged');assert.equal(execute.state()[0].error,item.source);}
      if(mode==='checked'){dialog.querySelector('[data-enable]').click();assert.equal(dialog.querySelector('.dialog-error'),null);assert.equal(listeners.size,0);active=p.translate('en',custom);listeners.forEach(listener=>listener());assert.equal(dialog.querySelector('.dialog-error'),null);assert.equal(group.enabled,true);}
      if(mode==='quick'){
        const type=dialog.querySelector('#full-action-type');type.value='case_conversion';type.dispatchEvent(new p.window.Event('change',{bubbles:true}));
        assert.equal(dialog.querySelector('.dialog-error'),null);assert.equal(listeners.size,0,'changing the action type releases the old failure listener');
        active=p.translate('nl');listeners.forEach(listener=>listener());assert.equal(dialog.querySelector('.dialog-error'),null);
        dialog.querySelector('[data-keep]').click();for(let i=0;i<4;i++)await new Promise(resolve=>setTimeout(resolve,0));
        assert.equal(listeners.size,1);active=p.translate('de');listeners.forEach(listener=>listener());p.translateTree(dialog);
        assert.equal(dialog.querySelector('.dialog-error').textContent,new BatchFailure(execute.state(),'').text((key,params)=>active.t(key,params),text=>active.translate(text)));
      }
      dialog.close();assert.equal(listeners.size,0);
    }finally{p.close();}
  }
});

test('actual global and modal batch error renderers retain source records and raw context through live language changes',()=>{
  const keys={'{value} / action {value2}: {error}':'Action {value2} for [{value}]: {error}','{value}: {error}':'Group [{value}]: {error}','Changed outside TagGroove after {value}; it was not {verb}':'After [{value}], not {verb}','It was not {verb}: {error}':'Not {verb}: {error}','Folders and side files could not be {verb}: {error}':'Folders not {verb}: {error}','Destination exists or another file maps to {value}':'Taken: [{value}]','Line {line}: {error}':'At {line}: {error}','The cover image could not be saved: {error}':'Cover failed: {error}','Cannot write {path}: {error}':'Writing [{path}]: {error}',undone:'reverted',redone:'reapplied','Select files first.':'Help',Help:'Wrong second lookup',Title:'Wrong raw name'};
  const declaration=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='modalError'),js=ts.transpileModule(declaration.getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  for(const item of batchDiagnosticCases){const p=page();let active=new Translator('en');const listeners=new Set();try{
    const rawPath=item.diagnostic.kind==='history_folders_failed'?'Help / action 1: Title':'/Music/Help; Title.wav',row={path:rawPath,error:item.source};
    const result=normalizeBatchDiagnostics({errors:[row],error_diagnostics:[{index:0,diagnostic:item.diagnostic}]},true),e=result.errors[0],failure=new BatchFailure(result.errors,'Failed',{name:path=>path.split('/').at(-1)});
    assert.equal(failure.message,(item.diagnostic.kind==='history_folders_failed'?rawPath:rawPath.split('/').at(-1))+': '+item.source);
    const renderModal=new Function('showFileError','t','translate','onLanguageChange','publicMessage',`${js};return modalError;`)(showFileError,(key,params)=>active.t(key,params),text=>active.translate(text),listener=>{listeners.add(listener);return()=>listeners.delete(listener);},error=>error instanceof Error?error.message:String(error));
    const dialog=modal(p)('Failure','', '<button>Close</button>');renderModal(dialog,failure);const errors=p.window.document.createElement('section');p.body.append(errors);
    for(const [locale,strings] of [['de',catalogs.de],['nl',catalogs.nl],['en',keys],['en',{}],['nl',catalogs.nl]]){active=p.translate(locale,strings);listeners.forEach(listener=>listener());errors.innerHTML=mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path.split('/').at(-1),publicMessage:value=>value,t:(key,params)=>active.t(key,params),translate:text=>active.translate(text)});p.translateTree(p.body);
      const expected=fileErrorText(e,(key,params)=>active.t(key,params),text=>active.translate(text));assert.equal(errors.querySelector('span').textContent,expected);assert.equal(errors.querySelector('span').getAttribute('translate'),'no');assert.equal(dialog.querySelector('.dialog-error').textContent,failure.text((key,params)=>active.t(key,params),text=>active.translate(text)));assert.ok(!errors.textContent.includes('Wrong'));if(locale!=='en'&&!(item.diagnostic.kind==='action_validation'&&item.diagnostic.cause.kind==='external'))assert.notEqual(expected,item.source);assert.equal(e.error,item.source);
      if(item.diagnostic.kind==='history_folders_failed')assert.equal(errors.querySelector('strong').textContent,rawPath+':');
    }
    clearFileError(dialog);assert.equal(listeners.size,0);active=p.translate('de');listeners.forEach(listener=>listener());assert.equal(dialog.querySelector('.dialog-error'),null);renderModal(dialog,'Select files first.');assert.equal(dialog.querySelector('.dialog-error').hasAttribute('translate'),false);p.translateTree(dialog);assert.equal(dialog.querySelector('.dialog-error').textContent,active.t('Select files first.'));dialog.close();assert.equal(listeners.size,0);
    const legacy=normalizeBatchDiagnostics({errors:[row]},true).errors[0];errors.innerHTML=mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e:legacy,esc,displayName:path=>path,publicMessage:value=>value,t:(key,params)=>active.t(key,params),translate:text=>active.translate(text)});p.translateTree(errors);assert.equal(errors.querySelector('span').textContent,item.source);
  }finally{p.close();}}
});

test('actual playlist expansion retains independently indexed diagnostics while merging two lists',async()=>{
  const cases=batchDiagnosticCases.filter(item=>item.diagnostic.kind==='playlist_line');
  assert.ok(cases.length>=2);
  const calls=[],replies=cases.slice(0,2).map((item,index)=>({files:[`/Music/track-${index}.wav`],errors:[{path:`/Music/Help; Title-${index}.m3u`,error:item.source}],error_diagnostics:[{index:0,diagnostic:item.diagnostic}]}));
  const ui=new PlaylistUI({invoke:async(command,args)=>{assert.equal(command,'read_playlist');calls.push(args.path);return normalizeBatchDiagnostics(replies[calls.length-1],true);}});
  const expanded=await ui.expand(['/Music/first.m3u','/Music/ordinary.wav','/Music/second.cue']);
  assert.deepEqual(expanded.paths,['/Music/track-0.wav','/Music/ordinary.wav','/Music/track-1.wav']);
  assert.deepEqual(calls,['/Music/first.m3u','/Music/second.cue']);
  assert.equal(expanded.playlist,'/Music/second.cue');
  for(const [index,item] of cases.slice(0,2).entries()){
    assert.equal(expanded.errors[index].error,item.source);
    assert.deepEqual(expanded.errors[index].diagnostic,item.diagnostic);
    for(const locale of ['de','nl']){const active=new Translator(locale,catalogs[locale]);assert.equal(fileErrorText(expanded.errors[index],(key,params)=>active.t(key,params),text=>active.translate(text)),batchErrorText(item.diagnostic,(key,params)=>active.t(key,params),text=>active.translate(text)));}
  }
});
test('actual invoke normalizes only covered command replies before callers merge errors in both performance paths',async()=>{
  const fn=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='invoke'),choice=parsed.statements.filter(ts.isVariableStatement).flatMap(node=>[...node.declarationList.declarations]).find(node=>node.name.getText(parsed)==='DIAGNOSTIC_COMMANDS');
  const js=ts.transpileModule(`const ${choice.getText(parsed)};${fn.getText(parsed)};return invoke;`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  const fixture=batchDiagnosticCases[0],row={path:'Help',error:fixture.source};
  for(const measured of [false,true])for(const metadata of [undefined,[],[{index:0,diagnostic:fixture.diagnostic}],null,[{index:2,diagnostic:fixture.diagnostic}]]){
    const raw={entries:[],errors:[row],...(metadata===undefined?{}:{error_diagnostics:metadata})},records=[];
    const invoke=new Function('normalizeBatchDiagnostics','coreInvoke','perfSink','performance',js)(normalizeBatchDiagnostics,async()=>raw,()=>measured?records:undefined,performance);
    const shown=await invoke('execute_action_groups');
    assert.equal(shown.errors[0].diagnostic!==undefined,Array.isArray(metadata)&&metadata.length===1&&metadata[0].index===0);
    assert.equal(shown.errors[0].diagnosticRaw===true,metadata===undefined||metadata===null||metadata?.[0]?.index===2);
    if(measured){assert.equal(records[0].out,JSON.stringify(raw).length);assert.equal(records[0].pending,false);}else assert.equal(records.length,0);
    const unrelated=await invoke('runtime_platform');assert.equal(unrelated,raw,'shape alone never opts another command into the protocol');
  }
});

test('actual history menus replay progress completion and typed errors translate owned labels once',async()=>{
  const {historyLabel}=await load('edit-history.ts'),{redoOrRepeat}=await load('utils-ui.ts');
  const native=await readFile(new URL('../src-tauri/src/lib.rs',import.meta.url),'utf8');
  assert.ok(native.includes('format!("Action {}", group.name)'));assert.ok(native.includes('format!("Actions ({} groups)", enabled.len())'));
  const declaration=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='replayHistory');
  const replay=ts.transpileModule(declaration.getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  const runReplay=new Function('label','kind','files','failures',`return(async()=>{
    let history={undo:{label,files},redo:{label,files},notice:null},status='',progress;
    const errors=Array.from({length:failures},()=>({path:'Help',error:'raw'})),entries=new Map(),rowFile=path=>path,applyRenamed=()=>{},accept=()=>{};
    const invoke=async()=>({label,entries:Array.from({length:files},(_,i)=>({path:String(i)})),history}),run=async(text,job)=>{progress=text;await job();};
    ${replay};await replayHistory(kind);return{progress,status,history};})()`);
  const english=JSON.parse(await source('i18n/catalogs/en.json'));
  const custom={...english,'Action {name}':'Copy of {name}','Actions ({count} groups)':'Copy of {count} groups','Copy of Help':'Again',Help:'Wrong name','Action Help':'Wrong action-like name','Actions (2 groups)':'Wrong count-like name'};
  const reordered={...custom,'Action {name}':'[{name}] applied','Actions ({count} groups)':'[{count}] groups applied','[Help] applied':'Again'};
  const cases=['Help','Action Help','Actions (2 groups)','  Help  ','Help\nTitle'].map(name=>({label:`Action ${name}`,key:'Action {name}',params:{name}}));
  cases.push({label:'Actions (2 groups)',key:'Actions ({count} groups)',params:{count:'2'}});
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom],['en',reordered]])for(const [i,item] of cases.entries())for(const kind of ['undo','redo']){
    const files=i%2+1,failures=i%2,observed=await runReplay(item.label,kind,files,failures),step={label:item.label,files};
    const raw={menu:kind==='undo'?historyLabel('Undo',step):redoOrRepeat({redo:step},undefined).label,progress:observed.progress,status:observed.status};
    assert.equal(observed.history[kind].label,item.label);assert.ok(observed.progress.startsWith(kind==='undo'?'Undoing ':'Redoing '));
    const p=page();try{
      for(const [language,strings] of [[locale,catalog],['en',{}],[locale,catalog]]){
        const tr=new Translator(language,strings),t=(key,params)=>tr.t(key,params),label=t(item.key,item.params),verb=t(kind==='undo'?'undone':'redone');
        if(language!=='en'||strings===custom||strings===reordered)assert.notEqual(label,item.label);
        const menuKey=kind==='undo'?`{kind} {label} ({files} file${files===1?'':'s'})`:`Redo {label} ({files} file${files===1?'':'s'})`;
        const expected={menu:t(menuKey,{kind:t('Undo'),label,files}),progress:t(`${kind==='undo'?'Undoing':'Redoing'} {label}…`,{label}),status:t(`${kind==='undo'?'Undid':'Redid'} {label}: {restored} file${files===1?'':'s'}${failures?`, {length} not ${kind==='undo'?'undone':'redone'}`:''}`,{label,restored:files,length:failures})};
        const diagnostic={kind:'history_changed',label:item.label,direction:kind},e={path:'Help',error:batchErrorText(diagnostic),diagnostic};
        p.body.innerHTML=Object.entries(raw).map(([key,text])=>`<p data-history="${key}">${esc(text)}</p>`).join('')+mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path,publicMessage:value=>value,t,translate:text=>tr.translate(text)});
        p.translate(language,strings);p.translateTree(p.body);
        for(const [key,text] of Object.entries(expected))assert.equal(p.body.querySelector(`[data-history="${key}"]`).textContent,text,`${language} ${key} ${JSON.stringify(item.label)}`);
        const expectedError=t('Changed outside TagGroove after {value}; it was not {verb}',{value:label,verb});
        assert.equal(p.body.querySelector('span[translate="no"]').textContent,expectedError);assert.equal(p.body.querySelector('strong').textContent,'Help:');
        assert.equal(e.diagnostic.label,item.label);assert.equal(observed.history[kind].label,item.label);
      }
    }finally{p.close();}
  }
});
function nativeHistoryLabels(sources){
  const labels=new Set();
  for(const text of Object.values(sources)){const production=text.split(/#\[cfg\(test\)\]\s*mod tests/)[0];for(const match of production.matchAll(/(?:history::)?\brecord(?:_[a-z_]+)?\s*\(\s*"([^"]+)"/g))labels.add(match[1]);}
  for(const [name,start] of [['utilities','impl Util {'],['covers','fn history_label(']])for(const match of sources[name].split(start)[1].split(/\r?\n}\r?\n/)[0].matchAll(/"([^"]+)"/g))labels.add(match[1]);
  return labels;
}
test('native fixed history label producers have registered exact translations and unknown labels remain raw',async()=>{
  const sources={};for(const name of ['lib','history','converters','sources/mod','utilities','covers'])sources[name]=await readFile(new URL(`../src-tauri/src/${name}.rs`,import.meta.url),'utf8');
  const labels=nativeHistoryLabels(sources);
  for(const newline of ['\n','\r\n'])assert.deepEqual(nativeHistoryLabels(Object.fromEntries(Object.entries(sources).map(([name,text])=>[name,text.replace(/\r?\n/g,newline)]))),labels);
  assert.equal(labels.size,21);const generated=englishCatalog();
  for(const label of labels)assert.equal(generated[label],label,`register native history label ${label}`);
  const {historyLabelText}=await load('i18n/core.ts');assert.equal(typeof historyLabelText,'function');
  for(const locale of ['de','nl']){
    const tr=new Translator(locale,catalogs[locale]),t=(key,params)=>tr.t(key,params);
    for(const label of labels)assert.equal(historyLabelText(label,t),t(label));
    for(const label of ['Help','Help / action 1: Title; it was not undone','  unknown label  ']){
      assert.equal(historyLabelText(label,t),label);
      assert.equal(batchErrorText({kind:'history_changed',label,direction:'undo'},t,text=>tr.translate(text)),t('Changed outside TagGroove after {value}; it was not {verb}',{value:label,verb:t('undone')}));
    }
  }
});
test('conditional empty rename diagnostics register owned keys without translating identical external text',async()=>{
  const producer=await readFile(new URL('../crates/tag-core/src/actions.rs',import.meta.url),'utf8');
  const conditional=/ErrorCause::owned\(if renames_file\s*\{\s*"([^"]+)"\s*\}\s*else\s*\{\s*"([^"]+)"\s*\}\)/.exec(producer);
  assert.ok(conditional,'read both keys from the actual Format Value producer');
  const keys=conditional.slice(1),extra=JSON.parse(await readFile(new URL('../src/i18n/extra-keys.json',import.meta.url),'utf8'));
  const registered=new Set([...extractBackendSource(producer),...extra]),generated=englishCatalog();
  for(const key of keys){assert.ok(registered.has(key),`register ${key}`);assert.equal(generated[key],key);}
  const unexpectedFallback=()=>assert.fail('owned and external causes must not enter the DOM text matcher');
  for(const locale of ['de','nl'])for(const key of keys){
    const translator=new Translator(locale,catalogs[locale]),t=(key,params)=>translator.t(key,params);
    const owned={kind:'owned',key,params:{},children:{}},external={kind:'external',value:key};
    const localized=causeText(owned,t,unexpectedFallback);assert.notEqual(localized,key,`${locale} ${key}`);
    assert.equal(causeText(external,t,unexpectedFallback),key);
    const diagnostic={kind:'action',group:key,index:1,cause:owned},raw=batchErrorText(diagnostic);
    const path=`  ${key}  `,result=normalizeBatchDiagnostics({errors:[{path,error:raw}],error_diagnostics:[{index:0,diagnostic}]},true);
    assert.equal(result.errors[0].path,path);assert.equal(result.errors[0].error,raw);
    const shown=fileErrorText(result.errors[0],t,unexpectedFallback);
    assert.ok(shown.includes(localized),'translate the owned failure');
    assert.ok(shown.includes(key),'preserve the same-spelling authored group name');
  }
});
test('forwarded cover verification messages translate in typed and legacy production error rows',async()=>{
  const tags=await readFile(new URL('../crates/tag-core/src/tags.rs',import.meta.url),'utf8');
  const covers=await readFile(new URL('../crates/tag-core/src/cover_ops.rs',import.meta.url),'utf8');
  const containers=await readFile(new URL('../crates/tag-core/src/container_formats.rs',import.meta.url),'utf8');
  const lastLiteral=body=>[...body.matchAll(/"([^"]+)"/g)].at(-1)[1];
  const asfFailure=source=>lastLiteral(source.split('return asf::change_pictures_diagnostic(')[1].split('\n        );')[0]);
  const removed=containers.split('pub(super) fn remove_cover(')[1].split('\n}')[0];
  const keys=[asfFailure(tags),asfFailure(covers),lastLiteral(removed)];assert.equal(new Set(keys).size,3);
  const extra=JSON.parse(await readFile(new URL('../src/i18n/extra-keys.json',import.meta.url),'utf8'));
  const registered=new Set([...extractBackendSource(tags),...extractBackendSource(covers),...extractBackendSource(containers),...extra]),generated=englishCatalog();
  for(const key of keys){assert.ok(registered.has(key),`register forwarded ${key}`);assert.equal(generated[key],key);}
  for(const locale of ['de','nl'])for(const key of keys){
    const p=page();try{
      const translator=new Translator(locale,catalogs[locale]),t=(key,params)=>translator.t(key,params),translate=text=>translator.translate(text);
      const localized=causeText({kind:'owned_text',value:key},t,translate);assert.notEqual(localized,key,`${locale} ${key}`);
      assert.equal(causeText({kind:'external',value:key},t,translate),key);
      const diagnostic={kind:'action',group:key,index:1,cause:{kind:'owned_text',value:key}};
      assert.ok(batchErrorText(diagnostic,t,translate).includes(localized));
      const path=`  ${key}  `,e={path,error:key};
      p.body.innerHTML=mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path,publicMessage:value=>value});
      p.translate(locale,catalogs[locale]);assert.equal(p.body.querySelector('p').textContent,`${path}: ${localized}`);
      p.translate('en');assert.equal(p.body.querySelector('p').textContent,`${path}: ${key}`);
    }finally{p.close();}
  }
});
test('typed batch frames preserve raw collision text, source-owned versus IO causes and indexed association',()=>{
  const keys={
    '{value} / action {value2}: {error}':'Action {value2} for [{value}]: {error}',
    'Changed outside TagGroove after {value}; it was not {verb}':'After [{value}], not {verb}',
    'It was not {verb}: {error}':'Not {verb}: {error}',
    'Folders and side files could not be {verb}: {error}':'Folders not {verb}: {error}',
    'Destination exists or another file maps to {value}':'Taken: [{value}]',
    'Line {line}: {error}':'At {line}: {error}',
    'The cover image could not be saved: {error}':'Cover failed: {error}','{value}: {error}':'Group [{value}]: {error}','Cannot write {path}: {error}':'Writing [{path}]: {error}',undone:'reverted',redone:'reapplied','Select files first.':'Choose tracks',Help:'Wrong raw name',Title:'Wrong raw name','Choose tracks':'Wrong second lookup'};
  const t=new Translator('en',keys);
  for(const item of batchDiagnosticCases){
    assert.equal(batchErrorText(item.diagnostic),item.source);
    const result={entries:[],errors:[{path:'old',error:'Plain error'},{path:'Help: Title; Path',error:item.source}],error_diagnostics:[{index:1,diagnostic:item.diagnostic}]};
    const normalized=normalizeBatchDiagnostics(result,true);assert.equal(normalized.errors[0],result.errors[0]);assert.deepEqual(normalized.errors[1].diagnostic,item.diagnostic);
    const shown=batchErrorText(normalized.errors[1].diagnostic,(key,params)=>t.t(key,params),text=>t.translate(text));assert.notEqual(shown,item.source);assert.ok(!shown.includes('Wrong'));assert.equal(t.translate(item.source),item.source,'producer frames never enter automatic reconstruction');
    const cause=item.diagnostic.cause;if(cause?.kind==='owned')assert.ok(shown.includes('Choose tracks'));if(cause?.kind==='external')assert.ok(shown.includes('Select files first.'));
    if(item.diagnostic.group)assert.ok(shown.includes(item.diagnostic.group));if(item.diagnostic.label)assert.ok(shown.includes(item.diagnostic.label));if(item.diagnostic.path)assert.ok(shown.includes(item.diagnostic.path));
    const merged=[{path:'another',error:'other'},...normalized.errors].reverse();assert.deepEqual(merged[0].diagnostic,item.diagnostic);
    for(const metadata of [null,{},[{index:1,diagnostic:{kind:'unknown'}}],[{index:99,diagnostic:item.diagnostic}],[{index:1,diagnostic:item.diagnostic},{index:1,diagnostic:item.diagnostic}],[{index:1,diagnostic:{...item.diagnostic,...(item.diagnostic.kind==='action'?{index:999}:{kind:'destination_collision',path:'mismatch'})}}]]){
      const fallback=normalizeBatchDiagnostics({...result,error_diagnostics:metadata},true);assert.equal(fallback.errors[1].diagnostic,undefined);assert.equal(fallback.errors[1].diagnosticRaw,true);assert.equal(fallback.errors[1].error,item.source);
    }
    const legacy=normalizeBatchDiagnostics({errors:result.errors},true);assert.equal(legacy.errors[1].diagnosticRaw,true);
    const empty=normalizeBatchDiagnostics({...result,error_diagnostics:[]},true);assert.equal(empty.errors[1].diagnosticRaw,undefined);
  }
});

test('structured converter rollback formatter matches native English and protects raw detail independently of nested UI text',()=>{
  const custom={'{error}; rename rollback {value}':'Rename [{value}] because [{error}]','{error}; text import rollback {value}':'Import [{value}] because [{error}]',completed:'Done now','original backup retained at {path}':'Backup: {path}','could not retain recovery copy: {value}':'No backup: {value}','no recovery copy is available':'Backup missing','Select files first.':'Help',Help:'Wrong second lookup',Title:'Wrong path','File changed externally after commit; rollback did not overwrite it':'Changed externally','Missing rollback copy':'Missing backup','Moved file changed externally; retained at destination':'Moved externally','Help: IO;  Title':'Wrong external reason','Title: keep;  Help':'Wrong retention reason','Err(Os { code: 13, kind: PermissionDenied, message: "Help" })':'Wrong debug','Cannot create {value}: {e}':'Create failed for [{value}] because [{e}]','Cannot rename {value} to {value2}: {error}; rollback: {rollback}':'From [{value}] to [{value2}]: [{error}], restore [{rollback}]'};
  for(const item of rollbackCases){
    assert.equal(converterRollbackText(item.payload),item.source);assert.deepEqual(converterRollbackError({converter_rollback:item.payload}),item.payload);
    for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
      const t=new Translator(locale,catalog),shown=converterRollbackText(item.payload,(key,params)=>t.t(key,params),text=>t.translate(text));assert.notEqual(shown,item.source);assert.ok(!shown.includes('Wrong'));
      if(item.payload.error.kind==='external')assert.ok(shown.includes(item.payload.error.message));
      for(const failure of item.payload.failures){assert.ok(shown.includes(failure.path));if(failure.error.kind==='external')assert.ok(shown.includes(failure.error.message));if(failure.error.kind==='rename'){const detail=failure.error.message;for(const key of ['path','source','destination','rollback'])if(typeof detail[key]==='string')assert.ok(shown.includes(detail[key]));if(typeof detail.error==='string')assert.ok(shown.includes(detail.error));if(detail.error?.kind==='external')assert.ok(shown.includes(detail.error.error));}if(failure.recovery?.kind==='retained')assert.ok(shown.includes(failure.recovery.path));if(failure.recovery?.kind==='failed')assert.ok(shown.includes(failure.recovery.error));}
      if(!item.payload.failures.length)assert.ok(shown.includes(locale==='de'?'abgeschlossen':locale==='nl'?'voltooid':'Done now'));
      if(item.id==='rename-helper-variants'){assert.ok(shown.includes(locale==='de'?'kann nicht erstellt werden':locale==='nl'?'niet aanmaken':'Create failed for'));assert.ok(!shown.includes('Cannot create '));assert.ok(!shown.includes('No free temporary name')||locale==='en');}
    }
  }
  for(const error of ['Select files first.',null,{}, {converter_rollback:{operation:'bad'}},{converter_rollback:{operation:'rename',error:{kind:'message',message:'Help'},failures:[{path:1}]}}])assert.equal(converterRollbackError(error),undefined);
});

test('actual converter Apply failure keeps structured rollback in both global errors and modal across live language changes',async()=>{
  const names=['modal','modalError','value','reviewedConverter'],declarations=names.map(name=>parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name));assert.ok(declarations.every(Boolean));
  const js=ts.transpileModule(declarations.map(node=>node.getText(parsed)).join('\n'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  const custom={'{error}; rename rollback {value}':'Rename [{value}] because [{error}]','{error}; text import rollback {value}':'Import [{value}] because [{error}]',completed:'Done now','original backup retained at {path}':'Backup: {path}','could not retain recovery copy: {value}':'No backup: {value}','no recovery copy is available':'Backup missing','Select files first.':'Help',Help:'Wrong second lookup',Title:'Wrong path'};
  for(const item of rollbackCases){const p=page();let active=new Translator('en');const listeners=new Set(),calls=[];let thrown={converter_rollback:item.payload};
    try{
      p.body.innerHTML='<section class="app-errors"></section>';const sourcePaths=['/A - Help.wav','/B - Title.wav','/C - Other.wav'];
      const dependencies={BatchFailure,showFileError,clearFileError,esc,app:p.body,paths:()=>sourcePaths,dirtySelected:()=>false,dataRevision:0,compactScreen:()=>false,columnSpecs:()=>[],converterValidator,createConverterInputTracker,createConverterReview,converterRollbackError,converterRollbackText,t:(key,params)=>active.t(key,params),translate:text=>active.translate(text),onLanguageChange:listener=>{listeners.add(listener);return()=>listeners.delete(listener);},publicMessage:error=>String(error),applyRenamed:()=>assert.fail('Failed Apply must not apply rename results'),accept:()=>{},run:async(_text,job)=>job(),invoke:async(command,args)=>{calls.push(command);if(command==='scan_files')return{entries:[],errors:[]};if(command==='apply_filename_conversion'||command==='apply_text_import')throw thrown;if(command==='preview_filename_conversion')return{plan_token:'fixture',items:sourcePaths.map((path,index)=>({path,target:'/renamed-'+index+'.wav',captures:{TITLE:['Help']}}))};if(command==='preview_text_import')return{plan_token:'fixture',items:sourcePaths.map((path,index)=>({path,selection_index:index,source_line:index+1,fields:{TITLE:['Help']}}))};throw new Error('Unexpected '+command);}};
      const model=new Function(...Object.keys(dependencies),`let errors=[],status='',busy=false,reviewedConverterContextChanged;${js};return{open:reviewedConverter,state:()=>({errors,status})};`)(...Object.values(dependencies));
      const kind=item.payload.operation==='rename'?'filename':'text';model.open(kind);const dialog=p.body.querySelector('dialog');
      if(kind==='text'){const select=dialog.querySelector('#text-source-kind');select.value='text';select.dispatchEvent(new p.window.Event('change',{bubbles:true}));dialog.querySelector('#text-source-records').value='Help\nTitle\nOther';dialog.querySelector('#text-mask').value='%title%';}
      const settle=async()=>{for(let i=0;i<4;i++)await new Promise(resolve=>setTimeout(resolve,0));};
      dialog.querySelector('#reviewed-preview-button').click();await settle();assert.equal(dialog.querySelector('#reviewed-apply').disabled,false);dialog.querySelector('#reviewed-apply').click();await settle();
      assert.ok(calls.includes(kind==='filename'?'apply_filename_conversion':'apply_text_import'));const error=model.state().errors[0];assert.deepEqual(error.rollback,item.payload);assert.equal(error.error,item.source);
      const render=()=>{p.body.querySelector('.app-errors').innerHTML=mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e:error,esc,displayName:path=>path,publicMessage:value=>value,converterRollbackText,t:(key,params)=>active.t(key,params),translate:text=>active.translate(text)});};
      for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom],['en',{}],['de',catalogs.de]]){active=p.translate(locale,catalog);listeners.forEach(listener=>listener());render();p.translateTree(p.body);await settle();const expected=converterRollbackText(item.payload,(key,params)=>active.t(key,params),text=>active.translate(text));assert.equal(dialog.querySelector('.dialog-error').textContent,expected);assert.equal(dialog.querySelector('.dialog-error').getAttribute('translate'),'no');assert.equal(p.body.querySelector('.app-errors p').textContent,`${active.t('Conversion')}: ${expected}`);assert.deepEqual(error.rollback,item.payload);}
      dialog.querySelector('#reviewed-preview-button').click();await settle();assert.equal(dialog.querySelector('.dialog-error'),null);active=p.translate('nl');listeners.forEach(listener=>listener());assert.equal(dialog.querySelector('.dialog-error'),null,'invalidated rollback must not reappear on language change');
      thrown='Select files first.';dialog.querySelector('#reviewed-apply').click();await settle();assert.equal(model.state().errors[0].rollback,undefined);assert.equal(model.state().errors[0].error,'Select files first.');assert.equal(dialog.querySelector('.dialog-error').hasAttribute('translate'),false);p.translateTree(p.body);assert.equal(dialog.querySelector('.dialog-error').textContent,active.t('Select files first.'));
      dialog.close();assert.equal(listeners.size,0);
    }finally{p.close();}
  }
});

test('real Cut/Paste failure statuses translate UI operation names while authored same-spelling names stay raw',()=>{
  const declaration=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='stageTagUpdates');
  const js=ts.transpileModule(declaration.getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  const statusFn=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='statusMarkup');
  const statusMarkup=new Function('esc','status','ColumnImportStatus',statusFn.getText(parsed)+';return `'+main.match(/<span class="status-message"[^>]*>[\s\S]*?<\/span>/)[0]+'`;');
  const custom={'{operation} failed':'Failure: {operation}','Paste tags':'Help','Cut tags':'Title',Help:'Wrong second translation',Title:'Wrong second translation','Copy of {name}':'Duplicate {name}'};
  const clipboard=copyTags(['Help','Title'].map(name=>({name,fields:{TITLE:[name]},covers:0}))),targets=['A','B','C'].map(path=>({path,fields:{TITLE:['unchanged']}}));
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const operation of ['Paste tags','Cut tags']){
    const p=page();try{
      const t=new Translator(locale,catalog);let renders=0;
      const stage=new Function('render',`let errors=[],status='';${js};return {run:stageTagUpdates,state:()=>({errors,status})};`)(()=>{renders++;});
      const plan=operation==='Paste tags'?()=>pastePlan(clipboard,targets):()=>{throw new Error('Select files first.');};
      stage.run(plan,operation,()=>{throw new Error('Success callback must not run');});
      const state=stage.state();assert.equal(renders,1);assert.equal(state.status,`${operation} failed`);assert.deepEqual(targets.map(target=>target.fields),targets.map(()=>({TITLE:['unchanged']})));
      p.body.innerHTML=statusMarkup(esc,state.status,ColumnImportStatus);p.translate(locale,catalog);
      const expected=locale==='de'?`${t.t(operation)} fehlgeschlagen`:locale==='nl'?`${t.t(operation)} mislukt`:`Failure: ${t.t(operation)}`;
      assert.equal(p.body.textContent,expected);p.translateTree(p.body);assert.equal(p.body.textContent,expected);p.translate('en');assert.equal(p.body.textContent,`${operation} failed`);
      const context=(e,translator)=>mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path,publicMessage:value=>value,translate:text=>translator.translate(text)});
      for(const [language,strings] of [[locale,catalog],['en',{}],[locale,catalog]]){
        const current=new Translator(language,strings);
        p.body.innerHTML=context(state.errors[0],current)+context({path:operation,error:'Select files first.'},current);p.translate(language,strings);
        const rows=p.body.querySelectorAll('p');assert.equal(rows[0].querySelector('strong').textContent,current.t(operation)+':');assert.equal(rows[1].querySelector('strong').textContent,operation+':');
        assert.equal(rows[0].textContent,`${current.t(operation)}: ${current.translate(state.errors[0].error)}`);assert.equal(rows[1].textContent,`${operation}: ${current.t('Select files first.')}`);
        p.translateTree(p.body);assert.equal(rows[0].querySelector('strong').textContent,current.t(operation)+':');
      }
      assert.equal(state.errors[0].path,operation);assert.equal(state.errors[0].error,operation==='Paste tags'?'The clipboard holds tags of 2 files. Select 1 or 2 files to paste them.':'Select files first.');
    }finally{p.close();}
  }
});
test('production application error contexts translate once without changing same-spelling file labels',()=>{
  const labels=['Layout','Cover','Tag panel','Playlist','Tools','Notification area','Appearance','Options','Tag sources','Columns','Operation','Reload','Conversion','Repeat','Export','Rename','Action groups','Directories','Platform','Command line','Window','Drag and drop','Cut tags','Paste tags',...Object.values(UTILS).map(spec=>spec.label)];
  const custom={...Object.fromEntries(labels.map(label=>[label,'Help'])),Help:'Wrong second translation'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();try{
      const t=new Translator(locale,catalog),render=e=>mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path,publicMessage:value=>value,translate:text=>t.translate(text)});
      for(const label of labels){p.body.innerHTML=render({path:label,label,error:'Select files first.'})+render({path:label,error:'Select files first.'});p.translate(locale,catalog);const contexts=p.body.querySelectorAll('strong');assert.equal(contexts[0].textContent,t.t(label)+':');assert.equal(contexts[1].textContent,label+':');p.translateTree(p.body);assert.equal(contexts[0].textContent,t.t(label)+':');}
    }finally{p.close();}
  }
});
test('actual Tools and Columns failure callbacks render localized contexts with unchanged body and raw same-name paths',async()=>{
  const custom={Tools:'Help',Columns:'Title',Help:'Wrong second translation',Title:'Wrong second translation','Select files first.':'Choose tracks.'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const kind of ['Tools','Columns']){
    const p=page();let renders=0,requests=0;
    try{
      const variable=parsed.statements.filter(ts.isVariableStatement).flatMap(node=>[...node.declarationList.declarations]).find(node=>node.name.getText(parsed)===(kind==='Tools'?'toolsUI':'columnUI'));
      const js=ts.transpileModule(`const ${variable.getText(parsed)};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
      const tool={name:'Tools',path:'/Tools',parameters:'%_path%',for_each_file:false},layout=defaultLayout(),unchanged=structuredClone(layout);
      const dependencies={ToolsUI,ColumnUI,BatchFailure,onLanguageChange:()=>()=>{},macPlatform:false,translateNow:()=>{},optionsUI:{page:()=>({tools:[tool]})},desktop:true,selected:new Set(['/Tools']),operationReason:()=>'',paths:()=>['/Tools'],listRoots:[],columnLayout:layout,applyColumns:async()=>{requests++;throw new Error('Select files first.');},invoke:async command=>{assert.equal(command,'run_tool');requests++;throw new Error('Select files first.');},publicMessage:error=>error.message??String(error),render:()=>{renders++;}};
      const model=new Function(...Object.keys(dependencies),`let errors=[],status='';${js};return {instance:${kind==='Tools'?'toolsUI':'columnUI'},state:()=>({errors,status})};`)(...Object.values(dependencies));
      p.body.innerHTML=model.instance.menuHTML();model.instance.bindMenu(p.body);
      if(kind==='Tools'){p.translate(locale,catalog);assert.equal(p.body.querySelector('[data-tool] span').textContent,'Tools');await model.instance.run(0);}
      else{p.body.querySelector('[data-column-toggle="title"]').click();await new Promise(resolve=>setTimeout(resolve,0));}
      assert.equal(requests,1);assert.equal(renders,1);const error=model.state().errors[0];assert.equal(error.path,kind);assert.equal(error.error,'Select files first.');assert.deepEqual(layout,unchanged);assert.equal(tool.name,'Tools');
      for(const [language,strings] of [[locale,catalog],['en',{}],[locale,catalog]]){
        const t=new Translator(language,strings),render=e=>mainTemplate(node=>node.head.text==='<p><strong translate="no">',{e,esc,displayName:path=>path,publicMessage:value=>value,translate:text=>t.translate(text)});
        p.body.innerHTML=render(error)+render({path:kind,error:error.error});p.translate(language,strings);const paragraphs=p.body.querySelectorAll('p');assert.equal(paragraphs[0].textContent,`${t.t(kind)}: ${t.t(error.error)}`);assert.equal(paragraphs[1].textContent,`${kind}: ${t.t(error.error)}`);p.translateTree(p.body);assert.equal(paragraphs[0].querySelector('strong').textContent,t.t(kind)+':');assert.equal(model.state().errors[0].error,'Select files first.');
      }
    }finally{p.close();}
  }
});
test('real source statuses retain owned label parts and raw authored titles across live languages',async()=>{
  const custom={'Searching {name}…':'Find {name}','Enter a {name} release ID or address.':'Enter an address for {name}.','freedb (local)':'Local index','MusicBrainz cover art':'MusicBrainz images','Discogs cover art':'Discogs images','Discogs Release ID':'Discogs lookup','Local index':'Wrong second lookup','MusicBrainz images':'Wrong second lookup',Help:'Wrong authored name'};
  const cases=[{provider:'local_freedb',name:'freedb (local)'},{provider:'musicbrainz',coversOnly:true,name:'MusicBrainz cover art'},{provider:'discogs',coversOnly:true,name:'Discogs cover art'},{provider:'freedb',remote:true,name:'freedb'},{provider:'discogs',releaseId:true,empty:true,name:'Discogs Release ID'},...['Help','freedb (local)','MusicBrainz cover art'].map(name=>({provider:'web',name}))];
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const request of cases){
    const p=page();let active=new Translator('en');const listeners=new Set();let finish;
    try{
      const host=rendererVariable(parsed,'tagSourcesUI',{TagSourcesUI:class{constructor(host){return host;}},webScriptErrorText,t:(key,params)=>active.t(key,params),translate:text=>active.translate(text),onLanguageChange:listener=>{listeners.add(listener);return()=>listeners.delete(listener);}});
      Object.assign(host,{files:()=>[],reason:()=>'',options:()=>({}),evaluate:async()=>'',invoke:async name=>{
        if(name==='tag_source_search'||name==='web_source_search')return await new Promise(resolve=>{finish=resolve;});
      }});
      await new TagSourcesUI(host).open(request.provider,{coversOnly:request.coversOnly,releaseId:request.releaseId,web:request.provider==='web'?{id:'fixture',name:request.name,search_by:[]}:undefined});
      const dialog=p.body.querySelector('dialog');if(request.remote){assert.deepEqual([...dialog.querySelector('#ts-mode').options].map(option=>option.value),['toc','artist_album']);dialog.querySelector('#ts-artist').value='Help';dialog.querySelector('#ts-album').value='Title';}
      dialog.querySelector('#ts-search').click();await new Promise(resolve=>setTimeout(resolve,0));
      const key=request.empty?'Enter a {name} release ID or address.':'Searching {name}…';
      const element=dialog.querySelector(request.empty?'.dialog-error':'.ts-status');assert.ok(element);assert.equal(element.getAttribute('translate'),'no');
      for(const [language,strings] of [[locale,catalog],['en',{}],[locale,catalog]]){
        active=p.translate(language,strings);listeners.forEach(listener=>listener());p.translateTree(dialog);await p.window.happyDOM.waitUntilComplete();
        const label=request.provider==='web'?request.name:active.translate(request.name);
        assert.equal(element.textContent,active.t(key,{name:label}));assert.equal(dialog.querySelector('header').textContent,label);
        assert.equal(dialog.querySelector('[data-close]').textContent,active.t('Cancel'));
      }
      if(finish){finish(request.provider==='web'?{direct:false,columns:[],results:[]}:{results:[],total:0});await new Promise(resolve=>setTimeout(resolve,0));assert.equal(dialog.querySelector('.ts-status').hasAttribute('translate'),false);}
      dialog.close();assert.equal(listeners.size,0);
    }finally{p.close();}
  }
});
test('actual web-source search/release dialogs render structured script failures once across live languages',async()=>{
  const frame='line {line} ({text}): {error}',debug='Debug output is in {directory}',joinLimit='joining lines would exceed the 16 MB append limit for one run';
  const custom={[frame]:'{error} (source {text}, line {line})',[debug]:'Debug files: {directory}','this block has no {name}':'Missing {name}','{value} was not found':'Not found: {value}',Help:'Wrong identity',Endif:'Wrong command','Missing Endif':'Wrong second translation','Not found: "Alpha): Beta"':'Wrong second translation','json_foreach_end without json_foreach':'json_foreach_end needs json_foreach','json_foreach_counter outside json_foreach':'json_foreach_counter needs json_foreach','the script ran too long; check Do/While loops':'Step limit reached; inspect Do/While loops','Step limit reached; inspect Do/While loops':'Wrong second translation',[joinLimit]:'Join append budget reached','Join append budget reached':'Wrong second translation'};
  const commands=['IfVar Alpha): Beta','If "Alpha): Beta"',String.raw`If "Alpha\"): Beta"`,'FindLine "Alpha): Beta"','json_foreach_end','json_foreach_counter','While "Help"','JoinLines 1','JoinUntil "Help"'];
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const failedCommand of ['web_source_search','web_source_release'])for(const command of commands){
    const p=page();let active=new Translator('en');const listeners=new Set(),calls=[];
    try{
      const payload={file:'Help  Title): .src',section:failedCommand==='web_source_search'?'[ParserScriptIndex]':'[ParserScriptAlbum]',line:17,command,message:command.startsWith('Join')?joinLimit:command==='While "Help"'?'the script ran too long; check Do/While loops':command==='json_foreach_end'?'json_foreach_end without json_foreach':command==='json_foreach_counter'?'json_foreach_counter outside json_foreach':command.startsWith('FindLine')?'"Alpha): Beta" was not found':'this block has no Endif',debug_path:command.startsWith('FindLine')?'/Help  Title): /debug':null};
      // Capture the production main host so the real formatter and language subscription wiring run.
      const host=rendererVariable(parsed,'tagSourcesUI',{TagSourcesUI:class{constructor(host){return host;}},webScriptErrorText,t:(key,params)=>active.t(key,params),translate:text=>active.translate(text),onLanguageChange:listener=>{listeners.add(listener);return()=>listeners.delete(listener);}});
      Object.assign(host,{files:()=>[],reason:()=>'',options:()=>({}),evaluate:async()=>'',invoke:async name=>{
        calls.push(name);if(name===failedCommand)throw{web_source_script:payload};
        if(name==='web_source_search')return{direct:true,columns:[],results:[{url:'https://example.invalid/release'}]};
      }});
      const ui=new TagSourcesUI(host);await ui.open('web',{web:{id:'fixture',name:'Help',search_by:[]}});
      const dialog=p.body.querySelector('dialog');dialog.querySelector('#ts-search').click();await new Promise(resolve=>setTimeout(resolve,0));
      assert.ok(calls.includes(failedCommand));const error=dialog.querySelector('.dialog-error');assert.ok(error);assert.equal(error.getAttribute('translate'),'no');
      const expected=t=>{const leaf=payload.message===joinLimit?t.t(joinLimit):payload.message.startsWith('the script ran too long')?t.t(payload.message):payload.message.startsWith('json_foreach')?t.t(payload.message):payload.message.startsWith('this block')?t.t('this block has no {name}',{name:'Endif'}):t.t('{value} was not found',{value:'"Alpha): Beta"'});return `${payload.file} ${payload.section} ${t.t(frame,{line:17,text:command,error:leaf})}`+(payload.debug_path===null?'':`. ${t.t(debug,{directory:payload.debug_path})}`);};
      assert.equal(error.textContent,expected(active));
      for(const [language,strings] of [[locale,catalog],['en',{}],[locale,catalog]]){
        active=p.translate(language,strings);listeners.forEach(listener=>listener());p.translateTree(dialog);await p.window.happyDOM.waitUntilComplete();
        assert.equal(error.textContent,expected(active));assert.ok(error.textContent.includes(command));assert.ok(error.textContent.includes(payload.file));
        assert.equal(dialog.querySelector('[data-close]').textContent,active.t('Cancel'));
      }
      dialog.close();assert.equal(listeners.size,0);
    }finally{p.close();}
  }
  const p=page();try{
    const ui=new TagSourcesUI({files:()=>[],reason:()=>'',options:()=>({}),evaluate:async()=>'',invoke:async name=>{if(name==='web_source_search')throw'Select files first.';}});
    await ui.open('web',{web:{id:'fixture',name:'Help',search_by:[]}});p.body.querySelector('#ts-search').click();await new Promise(resolve=>setTimeout(resolve,0));
    const t=p.translate('de');assert.equal(p.body.querySelector('.dialog-error').textContent,t.t('Select files first.'));p.body.querySelector('dialog').close();
    assert.equal(webScriptErrorText({file:'Help.src',section:'[ParserScriptIndex]',line:0,command:'raw',message:'Select files first.',debug_path:null},(key,params)=>t.t(key,params),text=>t.translate(text)),`Help.src [ParserScriptIndex] ${t.t('Select files first.')}`);
  }finally{p.close();}
});
test('symbol-only catalog output replays original production control sources across languages',async()=>{
  for(const output of ['✓','42','…']){
    const p=page();
    try{
      p.body.innerHTML=controlMarkup({kind:'button',id:'symbol-control',label:'Open languages folder',run:()=>{}},{pages:{}})+
        '<input type="button" value="Help"><textarea title="Help" aria-label="Export template" placeholder="Help">Help</textarea><input type="text" value="Help">';
      const button=p.body.querySelector('button'),input=p.body.querySelector('input[type=button]'),textarea=p.body.querySelector('textarea');
      button.title='Help';button.setAttribute('aria-label','Help');
      const checks=(label,help,template)=>{assert.equal(button.textContent,label);assert.equal(button.title,help);assert.equal(button.getAttribute('aria-label'),help);assert.equal(input.getAttribute('value'),help);assert.equal(textarea.title,help);assert.equal(textarea.placeholder,help);assert.equal(textarea.getAttribute('aria-label'),template);assert.equal(textarea.value,'Help');assert.equal(p.body.querySelector('input[type=text]').value,'Help');};
      for(const locale of ['en','de','nl']){
        p.translate('en',{'Open languages folder':output,Help:output,'Export template':output});
        p.translateTree(p.body);await p.window.happyDOM.waitUntilComplete();checks(output,output,output);
        const t=p.translate(locale);checks(t.t('Open languages folder'),t.t('Help'),t.t('Export template'));
      }
    }finally{p.close();}
  }
});
test('external symbol-only writes replace cached sources before synchronous replay or language-off',async()=>{
  for(const boundary of ['observer','manual','language-off'])for(const output of ['✓','42','']){
    const p=page();
    try{
      p.body.innerHTML='<button title="Help">Help</button><input type="button" value="Help"><textarea aria-label="Help">Help</textarea>';
      p.translate('en',{Help:output});
      const button=p.body.querySelector('button'),input=p.body.querySelector('input'),textarea=p.body.querySelector('textarea');
      // An application write equal to our own last output still owns the new source.
      button.firstChild.data=output;button.title=output;input.setAttribute('value',output);textarea.setAttribute('aria-label',output);
      if(boundary==='manual')p.translateTree(p.body);
      if(boundary==='language-off')p.translate('en');
      await p.window.happyDOM.waitUntilComplete();p.translate('de');p.translate('en');
      assert.equal(button.textContent,output);assert.equal(button.title,output);assert.equal(input.getAttribute('value'),output);assert.equal(textarea.getAttribute('aria-label'),output);assert.equal(textarea.value,'Help');
    }finally{p.close();}
  }
});
test('application mutations equal to previous translations become fresh sources at observer and synchronous boundaries',async()=>{
  const custom={'Previewing…':'Preview',Preview:'Inspect',Inspect:'Wrong'};
  for(const boundary of ['observer','manual','language-off']){
    const p=page();
    try{
      p.body.innerHTML='<button title="Previewing…">Previewing…</button><input type="button" value="Previewing…"><textarea title="Previewing…">Help</textarea>';
      p.translate('en',custom);const button=p.body.querySelector('button'),input=p.body.querySelector('input'),textarea=p.body.querySelector('textarea');
      for(let pass=0;pass<2;pass++)p.translateTree(p.body);
      await p.window.happyDOM.waitUntilComplete();
      assert.equal(button.textContent,'Preview');assert.equal(button.title,'Preview');assert.equal(input.getAttribute('value'),'Preview');
      // Multiple real app writes must be invalidated as a batch, before any own output is produced.
      button.firstChild.data='Other';button.firstChild.data='Preview';button.title='Other';button.title='Preview';input.setAttribute('value','Preview');textarea.title='Preview';
      if(boundary==='manual')p.translateTree(p.body);
      if(boundary==='language-off'){
        p.translate('en');assert.equal(button.textContent,'Preview');assert.equal(button.title,'Preview');assert.equal(input.getAttribute('value'),'Preview');
        p.translate('en',custom);
      }
      await p.window.happyDOM.waitUntilComplete();
      assert.equal(button.textContent,'Inspect');assert.equal(button.title,'Inspect');assert.equal(input.getAttribute('value'),'Inspect');assert.equal(textarea.title,'Inspect');assert.equal(textarea.value,'Help');
      p.translateTree(p.body);await p.window.happyDOM.waitUntilComplete();assert.equal(button.textContent,'Inspect');
      p.translate('en');assert.equal(button.textContent,'Preview');assert.equal(button.title,'Preview');assert.equal(input.getAttribute('value'),'Preview');
    }finally{p.close();}
  }
});
test('production converter controls translate their pending and ready sources once',async()=>{
  const p=page();
  try{
    p.body.innerHTML='<dialog open><button id="apply"></button><button id="preview"></button><button id="close"></button></dialog>';
    const dialog=p.body.querySelector('dialog'),previewButton=dialog.querySelector('#preview'),review={pending:true,canApply:()=>false};
    const sync=rendererVariable(parsed,'syncControls',{paths:()=>['Help'],applying:false,picking:false,busy:false,dirtySelected:()=>false,rename:true,apply:dialog.querySelector('#apply'),readRequest:()=>({}),review,context:()=>'',previewButton,closeButton:dialog.querySelector('#close'),dialog});
    sync();p.translate('en',{'Previewing…':'Preview',Preview:'Inspect',Inspect:'Wrong'});p.translateTree(dialog);await p.window.happyDOM.waitUntilComplete();assert.equal(previewButton.textContent,'Preview');
    review.pending=false;sync();p.translateTree(dialog);await p.window.happyDOM.waitUntilComplete();assert.equal(previewButton.textContent,'Inspect');
    review.pending=true;sync();await p.window.happyDOM.waitUntilComplete();assert.equal(previewButton.textContent,'Preview');
    p.translate('en');assert.equal(previewButton.textContent,'Previewing…');
  }finally{p.close();}
});
for(const locale of ['de','nl'])test(`${locale}: provider source locations and names stay raw while unavailable fallbacks translate`,()=>{
  const p=page(),t=new Translator(locale,catalogs[locale]);
  try{
    let state={uri:'Help',name:'Title',writable:true,persisted:true,state:'ready',error:null};
    const declaration=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='sourceMarkup');
    const js=ts.transpileModule(declaration.getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
    const access={states:{fixture:state},refreshError:null};
    const values={active:()=>({path:'fixture'}),providerEntry:()=>true,documentAccess:access,sourcePresentation,sourcePolicy:()=>({mutationReason:'',unsupportedReason:''}),esc,workingMetadataNotice:()=>'',publicMessage:value=>value,button:(_op,label)=>`<button>${label}</button>`};
    const render=new Function(...Object.keys(values),`${js};return sourceMarkup;`)(...Object.values(values));
    p.body.innerHTML=render('files');p.translate(locale);
    assert.equal(p.body.querySelector('.source-location').textContent,'Help');assert.equal(p.body.querySelector('.source-location').title,'Help');assert.equal(p.body.querySelector('.source-access > div > span').textContent,'Title');
    assert.equal(p.body.querySelector('.source-access strong').textContent,t.t('Source writable'));
    assert.equal(p.body.querySelector('button').textContent,t.t('Save a copy…'));
    const locationHeader=()=>{
      const locationView=rendererVariable(parsed,'locationView',{entry:{path:'fixture'},providerEntry:()=>true,providerState:access.states.fixture,sourcePresentation,location:'Help',locationIsData:true});
      return rendererVariable(parsed,'locationMarkup',{esc,locationView});
    };
    p.body.innerHTML=locationHeader();p.translate(locale);assert.equal(p.body.querySelector('.path').textContent,'Help');assert.equal(p.body.querySelector('.path').title,'Help');
    access.states.fixture={...state,uri:'Source location unavailable',name:'Provider document'};p.body.innerHTML=render('files');p.translate(locale);
    assert.equal(p.body.querySelector('.source-location').textContent,'Source location unavailable');assert.equal(p.body.querySelector('.source-access > div > span').textContent,'Provider document');
    p.body.innerHTML=locationHeader();p.translate(locale);assert.equal(p.body.querySelector('.path').textContent,'Source location unavailable');
    for(const next of [{...state,uri:'',name:''},undefined]){
      access.states.fixture=next;p.body.innerHTML=render('files');p.translate(locale);
      assert.equal(p.body.querySelector('.source-location').textContent,t.t('Source location unavailable'));assert.equal(p.body.querySelector('.source-location').title,t.t('Source location unavailable'));assert.equal(p.body.querySelector('.source-access > div > span').textContent,t.t('Provider document'));
      p.body.innerHTML=locationHeader();p.translate(locale);assert.equal(p.body.querySelector('.path').textContent,t.t('Source location unavailable'));assert.equal(p.body.querySelector('.path').title,t.t('Source location unavailable'));
    }
    for(const [location,locationIsData,expected] of [['No folder loaded',false,t.t('No folder loaded')],['2 added locations',false,t.t('{length} added locations',{length:2})],['No folder loaded',true,'No folder loaded'],['2 added locations',true,'2 added locations'],['Help',true,'Help']]){
      const locationView=rendererVariable(parsed,'locationView',{entry:undefined,providerEntry:()=>false,providerState:undefined,sourcePresentation,location,locationIsData});
      p.body.innerHTML=rendererVariable(parsed,'locationMarkup',{esc,locationView});p.translate(locale);
      assert.equal(p.body.querySelector('.path').textContent,expected);assert.equal(p.body.querySelector('.path').title,expected);
    }
  }finally{p.close();}
});
test('the real language reload retains an active catalog on command failure and replaces it on an empty successful snapshot',async()=>{
  const p=page(),storage=Object.getOwnPropertyDescriptor(globalThis,'localStorage');
  Object.defineProperty(globalThis,'localStorage',{configurable:true,value:p.window.localStorage});
  try{
    const english=JSON.parse(await source('i18n/catalogs/en.json'));
    const jsonModule=value=>JSON.stringify(`data:text/javascript;base64,${Buffer.from(`export default ${JSON.stringify(value)};`).toString('base64')}`);
    const indexUrl=await moduleUrl('i18n/index.ts',{"'./core'":JSON.stringify(await moduleUrl('i18n/core.ts')),"'./dom'":JSON.stringify(await moduleUrl('i18n/dom.ts')),"'./catalogs/en.json'":jsonModule(english),"'./catalogs/nl.json'":jsonModule(catalogs.nl),"'./catalogs/de.json'":jsonModule(catalogs.de)});
    const i18n=await import(indexUrl),{languageOptionsPage,languageSetting,languageReadWarnings}=await import(await moduleUrl('language-options.ts',{"'./i18n'":JSON.stringify(indexUrl)}));
    i18n.setUserCatalogs([{file:'fr.json',catalog:{...english,'@meta':{name:'Français'},'Save tags':'Enregistrer'}}]);i18n.setLanguage('fr');
    const languagePage=languageOptionsPage({}),controls=languagePage.controls;
    let failed=true,renders=0,selected='fr',found={catalogs:[],errors:[]};
    const declaration=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='applyLanguage');
    const js=ts.transpileModule(declaration.getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
    const values={desktop:true,languageReadWarnings,invoke:async command=>{assert.equal(command,'language_catalogs');if(failed)throw new Error('languages: access denied');return found;},languagePage,setUserCatalogs:i18n.setUserCatalogs,setLanguage:i18n.setLanguage,languageSetting,optionsUI:{page:()=>({language:selected})},publicMessage:error=>error.message,applyNotificationArea:()=>{},render:()=>{renders++;}};
    const apply=new Function(...Object.keys(values),`let shownTitle='',languageGeneration=0;${js};return applyLanguage;`)(...Object.values(values));
    await apply();assert.equal(i18n.currentLanguage(),'fr');assert.equal(i18n.translate('Save tags'),'Enregistrer');assert.equal(renders,0);
    assert.ok(controls[0].choices.some(([value])=>value==='fr'));
    p.body.innerHTML=controls.map(controlMarkup).join('');assert.ok(p.body.textContent.includes('languages: access denied'));
    failed=false;await apply();assert.equal(i18n.currentLanguage(),'en');assert.equal(i18n.translate('Save tags'),'Save tags');assert.equal(renders,1);
    assert.ok(!controls[0].choices.some(([value])=>value==='fr'));
    p.body.innerHTML=controls.map(controlMarkup).join('');assert.ok(!p.body.textContent.includes('access denied'));
    const regions=['pt-BR','pt-PT','pt'].map(locale=>({file:`${locale}.json`,catalog:{...english,'@meta':{name:locale},'Save tags':`Save in ${locale}`}}));
    i18n.setUserCatalogs(regions);i18n.setLanguage('pt-BR');selected='pt-BR';found={catalogs:regions.slice(1),errors:[]};
    await apply();assert.equal(i18n.currentLanguage(),'en');assert.equal(i18n.translate('Save tags'),'Save tags');
    assert.ok(!controls[0].choices.some(([value])=>value==='pt-BR'));assert.ok(controls[0].choices.some(([value])=>value==='pt-PT'));
    p.body.innerHTML=controlMarkup(controls[0],{pages:{language:{language:selected}}});
    assert.equal(p.body.querySelector('select').value,'pt-BR');assert.equal(i18n.languageChoice(),'pt-BR');
    assert.equal(p.body.querySelector('select').selectedOptions[0].textContent,'Unavailable: pt-BR (using English)');
    p.body.innerHTML=controlMarkup(controls[0],{pages:{language:{language:'system'}}});assert.equal(p.body.querySelector('select').value,'system');
  }finally{if(storage)Object.defineProperty(globalThis,'localStorage',storage);else delete globalThis.localStorage;p.close();}
});
test('the real cell-editor host preserves custom column labels and translates default display names once',()=>{
  const custom={Title:'Help',Help:'Assistance','Edit {label}':'Change {label}','Edit cell':'Change the cell'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const column of [{...defaultLayout().columns.find(c=>c.id==='title')},...['Help','Title','cell','My  Files'].map(name=>({...defaultLayout().columns[1],id:'custom',name}))]){
    const p=page(),path='/Help.mp3',raw='Help  Title';let editor;
    try{
      const t=new Translator(locale,catalog);setColumnNameTranslation(text=>t.translate(text));
      p.body.innerHTML=`<div class="table-wrap"><table><thead><tr>${headerMarkup([column],[])}</tr></thead><tbody><tr><td data-col="${column.id}"></td></tr></tbody></table></div>`;
      editor=rendererVariable(parsed,'cellEditor',{CellEditor,commitCell:()=>path,revealRow:()=>p.body.querySelector('tbody tr'),CSS:p.window.CSS,entries:new Map([[path,{filename:'Help.mp3'}]]),editKey,columnLayout:{columns:[column]},columnName,field:()=>raw,anchor:undefined,paths:()=>[],selected:new Set()});
      assert.equal(editor.start({path,column:column.id}),true);p.translate(locale,catalog);
      const input=p.body.querySelector('.cell-editor'),shown=column.id==='title'?t.t('Title'):column.name;
      assert.equal(p.body.querySelector('[data-sort] > span').textContent,shown);
      assert.equal(input.getAttribute('aria-label'),t.t('Edit {label}',{label:shown}));assert.equal(input.value,raw);
      input.value='Typed  Help';input.setSelectionRange(2,6);
      setColumnNameTranslation(text=>text);p.translate('en');editor.reattach();
      assert.equal(input.getAttribute('aria-label'),`Edit ${column.name}`);assert.equal(input.value,'Typed  Help');assert.equal(input.selectionStart,2);assert.equal(input.selectionEnd,6);
      setColumnNameTranslation(text=>t.translate(text));editor.reattach();p.translate(locale,catalog);
      assert.equal(input.getAttribute('aria-label'),t.t('Edit {label}',{label:shown}));assert.equal(input.value,'Typed  Help');
    }finally{editor?.cancel();setColumnNameTranslation(text=>text);p.close();}
  }
});
test('source dialogs keep authored accessible names through search and review while built-in labels translate',async()=>{
  const custom={Help:'Assistance',Title:'Heading','freedb (local)':'Local catalog','MusicBrainz cover art':'MusicBrainz images',Search:'Find',Cancel:'Dismiss',Back:'Previous'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const request of [{provider:'web',name:'Help'},{provider:'web',name:'Title',title:'Help'},{provider:'web',name:'MiXeD < & >'},{provider:'local_freedb',name:'freedb (local)'},{provider:'musicbrainz',name:'MusicBrainz cover art',coversOnly:true}]){
    const p=page();
    try{
      const calls=[],ui=new TagSourcesUI({files:()=>[],reason:()=>'',options:()=>({}),evaluate:async()=>'',invoke:async command=>{
        calls.push(command);
        if(command==='web_source_search')return{direct:true,columns:[],results:[{url:'https://example.invalid/release'}]};
        if(command==='tag_source_search')return{results:[{id:'fixture',artist:'Help',title:'Title',year:'',country:'',label:'',catalog_number:'',format:'',track_count:0,score:0,url:''}],total:1};
        if(command==='web_source_release'||command==='tag_source_release')return{provider:request.provider,id:'fixture',url:'',artist:'Help',title:'Title',fields:{},tracks:[],covers:[]};
      }});
      await ui.open(request.provider,{coversOnly:request.coversOnly,title:request.title,web:request.provider==='web'?{id:'fixture',name:request.name,search_by:[]}:undefined});
      const dialog=p.body.querySelector('dialog'),name=request.title||request.name,t=p.translate(locale,catalog),expected=request.provider==='web'?name:t.translate(name);
      if(request.provider==='local_freedb')assert.equal(expected,locale==='de'?'freedb (lokal)':locale==='nl'?'freedb (lokaal)':'Local catalog');
      const accessibleName=()=>dialog.hasAttribute('aria-labelledby')?p.window.document.getElementById(dialog.getAttribute('aria-labelledby')).textContent:dialog.getAttribute('aria-label');
      assert.equal(accessibleName(),expected);assert.equal(dialog.querySelector('header').textContent,expected);
      assert.equal(dialog.querySelector('[data-close]').textContent,t.t('Cancel'));
      assert.equal(dialog.querySelector('#ts-search').textContent,t.t('Search'));
      dialog.querySelector('#ts-search').click();await new Promise(resolve=>setTimeout(resolve,0));
      if(request.provider!=='web'){dialog.querySelector('#ts-next').click();await new Promise(resolve=>setTimeout(resolve,0));}
      p.translateTree(dialog);
      assert.deepEqual(calls,request.provider==='web'?['web_source_search','web_source_release']:['tag_source_search','tag_source_release']);assert.equal(accessibleName(),expected);
      assert.equal(dialog.querySelector('header').textContent,`${expected}: Help – Title`);assert.equal(dialog.querySelector('#ts-back').textContent,t.t('Back'));
      dialog.querySelector('#ts-back').click();p.translateTree(dialog);assert.equal(accessibleName(),expected);assert.equal(dialog.querySelector('header').textContent,expected);
      p.translate('en');assert.equal(accessibleName(),name);dialog.close();
    }finally{p.close();}
  }
});
test('web-source result headers and search labels remain authored while application table/search controls translate',()=>{
  const custom={ARTIST:'Performer',Artist:'Performer',Help:'Assistance',Title:'Name',Search:'Find','Artist + Album':'Performer + Record','Search results':'Found records',Preview:'Inspect','Release artwork':'Release image','More results':'Show more','{length} of {total} shown':'{length} / {total} displayed'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();
    try{
      const webColumns=['ARTIST','Help','Title','MiXeD < & >'],values=['Help','Title','My  Files\nTitle','< & >'];
      const results=[{artist:'Help',title:'Title',year:'2026',country:'Help',label:'Help',catalog_number:'Title',format:'Help',track_count:1,score:100,id:'category/disc',url:'Help'}];
      const options={results,chosen:0,html:esc,rowAttributes:()=> 'data-index="0" aria-selected="true" tabindex="0"',webColumns,webRows:[{values}],total:2,columns:null,idLabel:'Entry ID'};
      const render=rendererVariable(tagSourceParsed,'resultMarkup',{...options,web:true});
      p.body.innerHTML=render();const t=p.translate(locale,catalog);
      assert.deepEqual([...p.body.querySelectorAll('th')].map(cell=>cell.textContent),webColumns);
      assert.deepEqual([...p.body.querySelectorAll('td')].map(cell=>[cell.textContent,cell.title]),values.map(value=>[value,value]));
      assert.equal(p.body.querySelector('table').getAttribute('aria-label'),t.t('Search results'));
      assert.equal(p.body.querySelector('tr[data-index]').getAttribute('aria-selected'),'true');
      p.translate('en');assert.deepEqual([...p.body.querySelectorAll('th')].map(cell=>cell.textContent),webColumns);
      const web={search_by:[{label:'ARTIST'},{label:'Search'},{label:'MiXeD label'},{label:'Search',default_label:true},{label:'Artist + Album',default_label:true}]};
      p.body.innerHTML=rendererVariable(tagSourceParsed,'searchFields',{web,webValues:['Help','Title'],html:esc});p.translate(locale,catalog);
      assert.deepEqual([...p.body.querySelectorAll('label > span')].map(span=>span.textContent),['ARTIST','Search','MiXeD label',t.t('Search'),t.t('Artist + Album')]);
      assert.equal(p.body.querySelector('#ts-web-0').value,'Help');assert.equal(p.body.querySelector('#ts-web-1').value,'Title');
      assert.equal(p.body.querySelector('#ts-search').textContent,t.t('Search'));
      for(const provider of ['musicbrainz','discogs','freedb','itunes','deezer','lastfm','fanart','lrclib','acoustid']){
        const render=rendererVariable(tagSourceParsed,'resultMarkup',{...options,web:false,provider,columns:resultColumns(provider),freedb:provider==='freedb',freedbId:()=>({category:'Help',disc:'Title'})});
        p.body.innerHTML=render();p.translate(locale,catalog);
        assert.ok([...p.body.querySelectorAll('th')].some(cell=>cell.textContent===t.t('Artist')));
        assert.equal(p.body.querySelector('tbody td[title="Help"]').textContent,'Help');
        assert.equal(p.body.querySelector('#ts-more').textContent,t.t('More results'));
      }
      for(const web of [false,true]){
        const chosen=rendererVariable(tagSourceParsed,'chosenMarkup',{results,chosen:0,web,provider:'musicbrainz',freedb:false,html:esc,preview:()=>'<button class="ts-preview">Preview</button>'});
        p.body.innerHTML=chosen();p.translate(locale,catalog);
        assert.equal(p.body.querySelector('.ts-url > span[translate="no"]').textContent,'Help');
        assert.equal(p.body.querySelector('.ts-preview').textContent,t.t('Preview'));
      }
      p.body.innerHTML='<section class="ts-cover"></section>';
      const renderCover=rendererVariable(tagSourceParsed,'renderCover',{release:{covers:[{kind:'Help',url:'https://example.invalid/cover.jpg'}]},dialog:p.body,coverIndex:0,$:selector=>p.body.querySelector(selector),storage:{get:()=>null},saveCoverKey:'fixture',syncReview:()=>{},coverTemplate:()=> 'Help',html:esc,loadImage:()=>{},extractCover:()=>{}});
      renderCover();p.translate(locale,catalog);
      assert.equal(p.body.querySelector('code').textContent,'Help');
      assert.equal(p.body.querySelector('#ts-cover-choice').value,'0');
      assert.equal(p.body.querySelector('#ts-cover-choice option').textContent,'1: Help');
    }finally{p.close();}
  }
});
test('source-authored settings prose remains authoritative while TagGroove controls and errors translate',async()=>{
  const custom={Save:'Store',Cancel:'Back',Help:'Assistance',Title:'Name','Select files first.':'Choose files','Save tags':'Write tags','{title} needs a number.':'A number is required for {title}.','This tag source has no settings.':'There are no settings.'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();
    try{
      const schema={key:'fixture',title:'Original source title',settings:[
        {type:'heading',key:'heading',title:'Original heading'},
        {type:'separator',key:'separator',title:'Original note'},
        {type:'bool',key:'flag',title:'Original flag',default:true},
        {type:'number',key:'count',title:'Original number',default:5},
        {type:'string',key:'text',title:'Original text',default:'Title'},
        {type:'string',key:'choice',title:'Original choice',choices:['Help','Title'],default:'Help'},
        {type:'string',key:'Help',default:'Title'},
      ],localizations:{[locale]:[
        {key:'fixture',title:'Save'},{key:'heading',title:'Help'},{key:'separator',title:'Title'},
        {key:'flag',title:'Cancel',description:'Select files first.'},
        {key:'count',title:'Help',description:'Save tags'},
        {key:'text',title:'Title',description:'Help'},{key:'choice',title:'Save',description:'Title'},
      ]}};
      let form={schema,values:{}},saved;
      const ui=new WebSourcesUI({invoke:async(command,args)=>{if(command==='web_source_settings')return form;assert.equal(command,'save_web_source_settings');saved=args.values;form={schema,values:saved};},fail:error=>{throw error;}});
      const entry={id:'fixture',menu:'Fallback name'};
      p.window.document.documentElement.lang=locale;await ui.settings(entry);const t=p.translate(locale,catalog);
      const dialog=p.body.querySelector('dialog');
      assert.equal(dialog.querySelector('header').textContent,'Save');
      assert.equal(p.body.querySelector('#'+dialog.getAttribute('aria-labelledby')).textContent,'Save');
      assert.equal(dialog.querySelector('.ws-heading').textContent,'Help');assert.equal(dialog.querySelector('.ws-note').textContent,'Title');
      assert.equal(dialog.querySelector('.action-check').textContent.trim(),'Cancel');
      assert.deepEqual([...dialog.querySelectorAll('.field > span')].map(node=>node.textContent),['Help','Title','Save','Help']);
      assert.deepEqual([...dialog.querySelectorAll('small')].map(node=>node.textContent),['Select files first.','Save tags','Help','Title']);
      assert.equal(dialog.querySelector('[data-close]').textContent,t.t('Cancel'));assert.equal(dialog.querySelector('#ws-save').textContent,t.t('Save'));
      assert.deepEqual([...dialog.querySelectorAll('option')].map(option=>option.textContent),['Help','Title']);
      assert.equal(dialog.querySelector('[data-setting="2"]').checked,true);assert.equal(dialog.querySelector('[data-setting="3"]').value,'5');
      assert.equal(dialog.querySelector('[data-setting="4"]').value,'Title');assert.equal(dialog.querySelector('[data-setting="5"]').value,'Help');
      dialog.querySelector('[data-setting="3"]').value='';dialog.querySelector('#ws-save').click();await p.window.happyDOM.waitUntilComplete();
      assert.equal(saved,undefined);assert.equal(dialog.querySelector('.dialog-error').textContent,t.t('{title} needs a number.',{title:'Help'}));
      assert.equal(dialog.querySelector('.ws-heading').textContent,'Help');assert.equal(dialog.querySelector('#ws-save').textContent,t.t('Save'));
      dialog.querySelector('[data-setting="3"]').value='7';dialog.querySelector('[data-setting="4"]').value='Help';dialog.querySelector('[data-setting="5"]').value='Title';
      dialog.querySelector('#ws-save').click();await p.window.happyDOM.waitUntilComplete();
      assert.deepEqual(saved,{flag:true,count:7,text:'Help',choice:'Title',Help:'Title'});
      await ui.settings(entry);p.translate(locale,catalog);assert.equal(p.body.querySelector('[data-setting="5"]').value,'Title');
      p.body.querySelector('[data-close]').click();form={schema:{...schema,settings:[]},values:{}};
      await ui.settings(entry);p.translate(locale,catalog);
      assert.equal(p.body.querySelector('header').textContent,'Save');assert.equal(p.body.querySelector('.dialog-content p').textContent,t.t('This tag source has no settings.'));
    }finally{p.close();}
  }
});

test('web-source settings save and redraw preserve choice tokens under bundled and custom catalogs',async()=>{
  const custom={Help:'Assistance',Title:'Nom','FORMAT STRING':'Format local','Select files first.':'Choisir des fichiers','Save':'Enregistrer'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();
    try{
      const schema={key:'fixture',settings:[{type:'string',key:'choice',title:'Format string',description:'Select files first.',choices:['Help','Title'],default:'Help'}]};
      let values={choice:'Title'},saved;
      const ui=new WebSourcesUI({invoke:async(command,args)=>{
        if(command==='web_source_settings')return{schema,values};
        assert.equal(command,'save_web_source_settings');saved=structuredClone(args.values);values=saved;
      },fail:error=>{throw error;}}),entry={id:'fixture',menu:'Source'};
      p.window.document.documentElement.lang=locale;
      await ui.settings(entry);const t=p.translate(locale,catalog),select=p.body.querySelector('select');
      assert.equal(select.value,'Title');assert.deepEqual([...select.options].map(option=>option.textContent),['Help','Title']);
      assert.equal(p.body.querySelector('label > span').textContent,'Format string');
      assert.equal(p.body.querySelector('small').textContent,'Select files first.');
      assert.equal(p.body.querySelector('#ws-save').textContent,t.t('Save'));
      select.value='Help';p.body.querySelector('#ws-save').click();await p.window.happyDOM.waitUntilComplete();
      assert.deepEqual(saved,{choice:'Help'});assert.equal(p.body.querySelector('dialog'),null);
      await ui.settings(entry);p.translate(locale,catalog);
      assert.equal(p.body.querySelector('select').value,'Help');
      assert.deepEqual([...p.body.querySelectorAll('option')].map(option=>option.textContent),['Help','Title']);
    }finally{p.close();}
  }
});

test('native Remove Tag translates empty settings and conjunctions while preserving tag identifiers',async()=>{
  const fallback='nothing (no tag type is checked under Options → Tags → {page})',list='{name} and {names}';
  const custom={[fallback]:'No selected types in {page}',[list]:'{name} plus {names}','Remove tags from {count} selected files?':'Remove metadata from {count} files?','MP3, TTA: {kinds}':'MPEG: {kinds}','APE, MPC, WavPack, TAK, OptimFROG: {kinds}':'APE family: {kinds}','Other formats: all tags':'Other metadata: all types',Mpeg:'MPEG options','Ape, Mpc':'APE options',ID3v1:'Wrong type',ID3v2:'Wrong type',APE:'Wrong type'};
  const nativeSource=(await source('i18n/dialog.ts')).replace(/^import .*;\r?\n/gm,'').replace(/^export /gm,''),js=ts.transpileModule(nativeSource,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const checked of [false,true]){
    const t=new Translator(locale,catalog),calls=[],ask=new Function('dialog','t','translate','translationActive',`${js}\nreturn ask;`)({ask:async(...args)=>{calls.push(args);return true;}},key=>t.t(key),text=>t.translate(text),()=>true);
    const flags={remove_id3v1:checked,remove_id3v2:checked,remove_ape:checked},tags={mpeg:flags,ape:flags},messages=new Messages({values:()=>({}),ask:(text,title)=>ask(text,title)});
    await messages.confirm('confirm_remove_tags',removalMessage(2,tags),'Remove tag');
    const kinds=page=>checked?t.t(list,{name:'ID3v1, ID3v2',names:'APE'}):t.t(fallback,{page:t.translate(page)});
    assert.equal(calls[0][0],[t.t('Remove tags from {count} selected files?',{count:2}),'',t.t('MP3, TTA: {kinds}',{kinds:kinds('Mpeg')}),t.t('APE, MPC, WavPack, TAK, OptimFROG: {kinds}',{kinds:kinds('Ape, Mpc')}),t.translate('Other formats: all tags')].join('\n'));
  }
});
test('structured confirmations translate source parts once through Messages and the native ask wrapper',async()=>{
  const first='Replace the current options, action groups, export configurations, columns, tag panel, favorite directories and remembered lists with {path}?';
  const reload='TagGroove reloads its window afterwards. The Discogs access token is not part of a configuration file.';
  const translator=new Translator('en',{[first]:'Save tags',[reload]:'Save tags','Save tags':'Write tags','Save tags Save tags':'Incorrect second translation',
    'Remove tags from {count} selected files?':'Save tags','MP3, TTA: {kinds}':'Save tags','APE, MPC, WavPack, TAK, OptimFROG: {kinds}':'Save tags','Other formats: all tags':'Save tags',
    'Save tags Save tags Save tags Save tags':'Incorrect second translation',Yes:'Save tags',No:'Keep tags','Load configuration':'Save tags','Remove tag':'Save tags'});
  const calls=[],nativeSource=(await source('i18n/dialog.ts')).replace(/^import .*;\r?\n/gm,'').replace(/^export /gm,'');
  const js=ts.transpileModule(nativeSource,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  const ask=new Function('dialog','t','translate','translationActive',`${js}\nreturn ask;`)({ask:async(...args)=>{calls.push(args);return false;}},key=>translator.t(key),text=>translator.translate(text),()=>true);
  await ask(configurationMessage({translate:text=>translator.translate(text),basename:path=>path,path:'Help  Title.json',drafts:{size:0}}),'Load configuration');
  assert.equal(calls.at(-1)[0],'Save tags\n\nSave tags');
  assert.deepEqual(calls.at(-1)[1],{title:'Save tags',okLabel:'Save tags',cancelLabel:'Keep tags'});
  const messages=new Messages({values:()=>({}),ask:(text,title)=>ask(text,title)}),tags={mpeg:{remove_id3v1:true},ape:{remove_ape:true}};
  await messages.confirm('confirm_remove_tags',removalMessage(2,tags),'Remove tag');
  assert.equal(calls.at(-1)[0],'Save tags\n\nSave tags\nSave tags\nSave tags');
});

for(const locale of ['de','nl']){
  test(`${locale}: post-export filenames and paths remain verbatim while prompt controls translate`,async()=>{
    const p=page();
    try{
      const opened=[],host={modal:modal(p),modalError:(_dialog,error)=>{throw error;},open:async paths=>{opened.push(paths);},reveal:async()=>{},fileManager:'Finder'};
      for(const output of ['Help','Title']){
        const done=offerOpenOutputs(host,[output],'Save tags');const t=p.translate(locale);
        assert.equal(p.body.querySelector('strong').textContent,output);
        assert.equal(p.body.querySelector('p').textContent,t.translate('Save tags.'));
        assert.equal(p.body.querySelector('#open-output-run').textContent,t.t('Open'));
        p.body.querySelector('#open-output-run').click();await p.window.happyDOM.waitUntilComplete();await done;
        assert.deepEqual(opened.at(-1),[output]);p.body.replaceChildren();
      }
      const outputs=['Help','My  Files/Title'],done=offerOpenOutputs(host,outputs,'Save tags');const t=p.translate(locale),select=p.body.querySelector('select');
      assert.deepEqual([...select.options].map(option=>option.textContent),['Help — Help','Title — My  Files']);
      assert.deepEqual([...select.options].map(option=>option.title),outputs);
      assert.equal(select.getAttribute('aria-label'),t.t('Written files'));
      select.value='1';p.body.querySelector('#open-output-run').click();await p.window.happyDOM.waitUntilComplete();await done;
      assert.deepEqual(opened.at(-1),[outputs[1]]);
    }finally{p.close();}
  });
  test(`${locale}: web-source choice labels and saved tokens stay verbatim while settings UI translates`,()=>{
    const p=page();
    try{
      const choices=['Help','Title','My  Files','A&B"<'],schema={key:'fixture',settings:[{type:'string',key:'choice',title:'Format string',description:'Select files first.',choices,default:'Help'}]};
      p.body.innerHTML=settingsMarkup({schema,values:{choice:'Title'}},locale);
      const t=p.translate(locale),select=p.body.querySelector('select');
      assert.deepEqual([...select.options].map(option=>option.textContent),choices);
      assert.deepEqual([...select.options].map(option=>option.value),choices);
      assert.equal(select.value,'Title');assert.deepEqual(readSettings(schema,p.body),{choice:'Title'});
      assert.equal(p.body.querySelector('label > span').textContent,'Format string');
      assert.equal(p.body.querySelector('small').textContent,'Select files first.');
      for(const choice of choices){select.value=choice;assert.deepEqual(readSettings(schema,p.body),{choice});}
      p.translate('en');assert.equal(select.value,choices.at(-1));assert.deepEqual([...select.options].map(option=>option.textContent),choices);
    }finally{p.close();}
  });
  test(`${locale}: saved tag-source cover notes translate their scaffold and retain the returned path`,async()=>{
    const p=page(),t=new Translator(locale,catalogs[locale]);
    try{
      for(const path of ['Help','Title','/My  Files/Help\nTitle.jpg']){
        p.body.innerHTML='<dialog open><p class="ts-cover-note" role="status"></p></dialog>';
        const dialog=p.body.querySelector('dialog'),note=dialog.querySelector('p'),calls=[];
        const extract=rendererVariable(tagSourceParsed,'extractCover',{release:{artist:'Artist',title:'Album',covers:[{url:'https://example.invalid/cover.png'}]},coverIndex:0,working:false,dialog,$:selector=>dialog.querySelector(selector),error:error=>{throw error;}},{host:{pickSave:async()=>path,invoke:async(command,args)=>{calls.push([command,args]);return path;}}});
        await extract();assert.equal(note.textContent,`Saved to ${path}`);p.translate(locale);
        assert.notEqual(note.textContent,`Saved to ${path}`);
        assert.equal(note.textContent,t.t('Saved to {outputPath}',{outputPath:path}));
        assert.deepEqual(calls,[['tag_source_save_cover',{url:'https://example.invalid/cover.png',path}]]);
        p.translate('en');assert.equal(note.textContent,`Saved to ${path}`);
      }
    }finally{p.close();}
  });
  test(`${locale}: copied and staged tag statuses translate optional UI sentences and retain numeric count guards`,()=>{
    const p=page(),t=new Translator(locale,catalogs[locale]);
    try{
      for(const count of [1,2])for(const cut of [false,true])for(const covers of [0,1,2])for(const failed of [0,1,2]){
        const clipboard={items:Array.from({length:count},()=>({artwork:Array.from({length:covers},()=>({}))}))};
        const source=copiedMessage(clipboard,cut,failed),art=covers?` ${t.t(`with {covers} cover${count*covers===1?'':'s'}`,{covers:count*covers})}`:'';
        const missingNote=failed?` ${t.t(`Artwork of {failed} file${failed===1?'':'s'} could not be copied.`,{failed})}`:'';
        const expected=t.t(`${cut?'Cut':'Copied'} tags of {files} file${count===1?'':'s'}{art}.{missingNote}`,{files:count,art,missingNote});
        p.body.textContent=source;p.translate(locale);assert.notEqual(p.body.textContent,source);assert.equal(p.body.textContent,expected);
        for(const restricted of [false,true]){
          const statusMessage=()=>source;
          const staged=mainTemplate(node=>node.getText(parsed).includes('new Set([...updates,...coverUpdates]'),{done:statusMessage,statusMessage,updates:[{path:'Help'}],coverUpdates:[],sourcePolicy:()=>({mutationReason:restricted?'read-only':''})});
          p.body.textContent=staged;p.translate(locale);
          const key=`{statusMessage} ${restricted?'Use Save a copy to keep these edits.':'Save tags to write them.'}`;
          assert.equal(p.body.textContent,t.t(key,{statusMessage:expected}));
        }
      }
      assert.equal(t.translate('Saved tags: Help files'),'Saved tags: Help files');
      assert.equal(t.translate('1 of Help tracks have a file. Saving writes the checked fields immediately.'),'1 of Help tracks have a file. Saving writes the checked fields immediately.');
    }finally{p.close();}
  });
  test(`${locale}: cleanup and index summaries translate UI phrases while bounded filename lists preserve raw values`,()=>{
    const p=page(),t=new Translator(locale,catalogs[locale]);
    try{
      for(const cancelled of [false,true])for(const removed of [0,1,2]){
        const source=cleanupText({checked:2,removed,cancelled});p.body.textContent=source;p.translate(locale);assert.notEqual(p.body.textContent,source);
        const removalSummary=t.t('removed {plural}',{plural:t.t(`{count} orphaned entr${removed===1?'y':'ies'}`,{count:removed})});
        assert.equal(p.body.textContent,t.t(cancelled?'Cleanup cancelled after {plural}; {removalSummary}.':'Cleanup checked {plural} and {removalSummary}.',{plural:t.t('{count} files',{count:2}),removalSummary}));
      }
      const indexText=rendererVariable(tagSourceParsed,'indexText',{directory:draft=>draft.folder});
      for(const discs of [1,2])for(const directory of ['Help','/My  Files/Title']){
        const source=indexText({discs,directory},{folder:'Help'});p.body.textContent=source;p.translate(locale);assert.notEqual(p.body.textContent,source);
        const key=directory==='Help'?'Text index: {discSummary} from this folder.':'The text index ({discSummary}) was built for {directory}. Build it again for this folder.';
        assert.equal(p.body.textContent,t.t(key,{discSummary:t.t(`{discs} disc${discs===1?'':'s'}`,{discs}),directory}));
      }
      const names=['Help','My  Files\nTitle'];const source=nameList([...names,'Other'],2);p.body.textContent=source;p.translate(locale);
      assert.equal(p.body.textContent,t.t('{names} and {length} more',{names:names.join(', '),length:1}));
      for(const field of ['TRACK','TOTALTRACKS','DISCNUMBER','TOTALDISCS']){
        p.body.textContent=`${field} requires a 16-bit unsigned number`;p.translate(locale);
        assert.equal(p.body.textContent,t.t('{field} requires a 16-bit unsigned number',{field}));
      }
    }finally{p.close();}
  });
  test(`${locale}: web-source names remain verbatim in enabled tooltips while disabled reasons translate`,()=>{
    const p=page();let reason='';
    try{
      const ui=new WebSourcesUI({reason:()=>reason,last:()=>null,shortcut:'Ctrl+I'});
      ui.listing.sources=['Help','Title'].map(name=>({id:name,menu:name,name,search_by:[],direct:false}));
      p.body.innerHTML=ui.menuHTML();const t=p.translate(locale);
      assert.deepEqual([...p.body.querySelectorAll('[data-web-source]')].map(button=>button.title),['Help','Title']);
      reason='Select files first.';p.body.innerHTML=ui.menuHTML();p.translate(locale);
      for(const button of p.body.querySelectorAll('[data-web-source]')){assert.equal(button.title,t.t(reason));assert.ok(button.disabled);}
      assert.deepEqual([...p.body.querySelectorAll('[data-web-source] > span')].map(span=>span.textContent),['Help…','Title…']);
    }finally{p.close();}
  });
  for(const newline of ['\n','\r\n'])test(`${locale}: native file confirmations preserve data and paragraphs (${JSON.stringify(newline)})`,async()=>{
    const translator=new Translator(locale,catalogs[locale]),calls=[];
    const nativeSource=(await source('i18n/dialog.ts')).replace(/^import .*;\r?\n/gm,'').replace(/^export /gm,'');
    const js=ts.transpileModule(nativeSource,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
    const dialogs=new Function('dialog','t','translate','translationActive',`${js}\nreturn {ask,confirm};`)({ask:async(...args)=>{calls.push(args);return false;},confirm:async(...args)=>{calls.push(args);return false;}},key=>translator.t(key),text=>translator.translate(text),()=>true);
    const {confirm,ask}=dialogs;
    const names=['Help',`My  Files${newline}Title`],destination='/Music/My  Files';
    for(const mac of [false,true])for(const operation of ['file-delete','file-delete-permanent','file-copy','file-move']){
      const host={mac,selection:()=>names.map(name=>({name,path:name})),openReason:()=>'',pickDirectories:async()=>[destination],confirm:(text,title)=>confirm(text.replace(/\r?\n/g,newline),title)};
      await new FileOpsUI(host).operate(operation);
      const [message,options]=calls.at(-1);
      assert.ok(message.includes(`${newline}${newline}${names.join(', ')}${newline}${newline}`),message);
      if(operation==='file-copy'||operation==='file-move')assert.ok(message.includes(destination));
      assert.notEqual(options.title,operation==='file-copy'?'Copy files':operation==='file-move'?'Move files':operation==='file-delete'?'Delete files':'Delete permanently');
      assert.equal(options.okLabel,translator.t('OK'));assert.equal(options.cancelLabel,translator.t('Cancel'));
    }
    const tags={mpeg:{remove_id3v1:true,remove_id3v2:false,remove_ape:false},ape:{remove_id3v1:false,remove_id3v2:false,remove_ape:true}};
    await ask({...removalMessage(2,tags),separator:newline},'Remove tag');
    const removal=calls.at(-1)[0];
    assert.ok(removal.includes(`${newline}${newline}MP3, TTA: ID3v1${newline}APE, MPC, WavPack, TAK, OptimFROG: APE${newline}`),removal);
    assert.notEqual(removal.split(newline)[0],'Remove tags from 2 selected files?');
    assert.notEqual(removal.split(newline).at(-1),'Other formats: all tags');
    for(const size of [0,1,2]){
      const filename=`Help  ${newline}Title.json`;
      const prompt=configurationMessage({basename:path=>path,path:filename,drafts:{size}});
      await ask({...prompt,separator:newline+newline},'Load configuration');
      const configuration=calls.at(-1)[0];
      assert.ok(configuration.includes(filename),configuration);
      assert.ok(configuration.includes(`${newline}${newline}${translator.t('TagGroove reloads its window afterwards. The Discogs access token is not part of a configuration file.')}`),configuration);
      assert.notEqual(configuration.split(newline)[0].startsWith('Replace the current options'),true);
    }
  });
  test(`${locale}: generated copy, playlist and tag-to-tag previews retain data and translate their controls`,async()=>{
    const p=page();
    try{
      const fail=(_dialog,error)=>{throw error;},uiModal=modal(p);
      copyAsTextDialog({modal:uiModal,modalError:fail,values:()=>({}),count:()=>7,evaluate:async()=>['Help','Title','My  Files\nTitle'],write:async()=>{},remember:async()=>{},done:()=>{}});
      new PlaylistUI({modal:uiModal,modalError:fail,values:()=>({}),files:()=>['Help'],workingDirectory:()=>'',loadedPlaylist:()=>'',columns:()=>[],invoke:async()=>({playlists:[{path:'Help',files:1}],errors:[]})}).generate();
      openTagToTag({modal:uiModal,modalError:fail,esc,paths:()=>['Help'],fieldNames:()=>[],displayName:path=>path,canApply:()=>true,preview:async()=>[{path:'Help',value:'Help'}],storage:{getItem:()=>JSON.stringify({field:'Help'}),setItem:()=>{}}});
      await p.window.happyDOM.waitUntilComplete();
      const t=p.translate(locale);
      assert.equal(p.body.querySelector('#copy-text-preview > div').textContent,'Help');
      assert.equal(p.body.querySelector('#copy-text-preview > div:nth-child(2)').textContent,'Title');
      assert.equal(p.body.querySelector('#copy-text-preview > div:nth-child(3)').textContent,'My  Files\nTitle');
      assert.equal(p.body.querySelector('#copy-text-preview').getAttribute('aria-label'),t.t('Preview'));
      assert.equal(p.body.querySelector('#copy-text-preview small').textContent,t.translate('… 2 more'));
      assert.equal(p.body.querySelector('#generate-preview > div:nth-child(2) > span').textContent,'Help');
      assert.equal(p.body.querySelector('#generate-preview small').textContent,`(${t.translate('1 file')})`);
      const tagRow=p.body.querySelector('#tag-tag-preview .preview > div');
      assert.equal(tagRow.querySelector('small').textContent,'Help');
      assert.equal(tagRow.textContent,'HelpHelp: Help');
      assert.equal(p.body.querySelector('#tag-tag-preview .preview').getAttribute('aria-label'),t.t('Tag → Tag results'));
      p.body.querySelector('#tag-tag-field').value='_DIRECTORY';p.body.querySelector('#tag-tag-field').dispatchEvent(new p.window.Event('input'));
      await p.window.happyDOM.waitUntilComplete();p.translate(locale);
      assert.equal(p.body.querySelector('#tag-tag-preview .preview > div').textContent,`Help${t.t('New folder name')}: Help`);
    }finally{p.close();}
  });
  test(`${locale}: main converter and numbering previews preserve their data while UI states translate`,()=>{
    const p=page(),data={esc,displayName:value=>value,publicMessage:value=>value};
    try{
      p.body.innerHTML=mainTemplate('<div class="preview">',{...data,result:[{path:'Help',value:'Help'},{path:'Title',value:'Title'}]});
      p.translate(locale);assert.equal(p.body.textContent,'HelpHelpTitleTitle');
      p.body.innerHTML=mainTemplate('<div class="preview">',{...data,result:[{path:'Help',fields:{Title:['Title']}},{path:'Help',fields:null}]},1);
      const t=p.translate(locale);assert.equal(p.body.querySelector('.preview > div').textContent,'HelpTitle: Title');
      assert.equal(p.body.querySelector('.no-match').textContent,`Help${t.t('No match, not changed')}`);
      p.body.innerHTML=mainTemplate('<div class="preview" tabindex="0" aria-label="Numbering preview">',{...data,rows:[{path:'Help',fields:{TRACK:['Help'],DISCNUMBER:['Help']}}],attempt:{request:{options:{disc_enabled:true}}}});
      p.translate(locale);assert.deepEqual([...p.body.querySelectorAll('tbody td')].map(el=>el.textContent),['Help','Help','Help']);
      assert.equal(p.body.querySelector('th').textContent,t.t('File'));
      const fieldRows=rendererVariable(parsed,'fieldRows',{esc});
      const reviewed=rendererVariable(parsed,'previewMarkup',{esc,fieldRows});
      p.body.innerHTML=reviewed([{path:'Help',target:'Title',captures:{Help:['Title','']}},{path:'Title',fields:{Help:[]},source_line:1,selection_index:0}],['Help']);
      p.translate(locale);
      assert.equal(p.body.querySelector('.preview').getAttribute('aria-label'),t.t('Reviewed conversion results'));
      assert.deepEqual([...p.body.querySelectorAll('strong')].map(el=>el.textContent),['Title','Title']);
      assert.equal(p.body.querySelector('small').textContent,'Help');
      assert.equal(p.body.querySelector('dt').textContent,'Help');
      assert.equal(p.body.querySelector('dd > span > span').textContent,'Title');
      assert.deepEqual([...p.body.querySelectorAll('em')].map(el=>el.textContent),[t.t('(empty)'),t.t('No values')]);
      assert.equal(p.body.querySelector('.no-match').textContent,`Help${t.t('No match, not changed')}`);
    }finally{p.close();}
  });
  test(`${locale}: tag-source matching previews preserve file and field data, including accessible labels`,()=>{
    const p=page();
    try{
      const track={position:'Title',length:1};
      const render=rendererVariable(tagSourceParsed,'matchMarkup',{release:{tracks:[track]},files:[{path:'Help',length:1}],slots:['Help',null],trackField:'Title',html:esc,duration:()=>'',fileLength:()=>'',fileName:path=>path||'',trackValue:()=> 'Help'});
      p.body.innerHTML=render();const t=p.translate(locale);
      assert.equal(p.body.querySelector('thead th:nth-child(2)').textContent,'Title');
      assert.equal(p.body.querySelector('tbody td').textContent,'Title');
      assert.equal(p.body.querySelector('tbody tr td:nth-child(4)').textContent,'Help');
      assert.equal(p.body.querySelector('tbody tr td:nth-child(4) span').title,'Help');
      assert.equal(p.body.querySelector('.ts-track-value').getAttribute('aria-label'),t.t('{trackField} of track {trackPosition}',{trackField:'Title',trackPosition:'Title'}));
      assert.equal(p.body.querySelector('thead th:nth-child(4)').textContent,t.t('File'));
      assert.equal(p.body.querySelector('tbody tr:nth-child(2) em').textContent,t.t('Not written'));
      track.position='A1';p.body.innerHTML=render();p.translate(locale);
      assert.equal(p.body.querySelector('.ts-track-value').getAttribute('aria-label'),t.t('{trackField} of track {trackPosition}',{trackField:'Title',trackPosition:'A1'}));
      for(const extra of [{country:'Title'},{barcode:'Help'}]){
        const chosen=rendererVariable(tagSourceParsed,'chosenMarkup',{results:[{artist:'Title',title:'Help',...extra}],chosen:0,web:false,provider:'musicbrainz',freedb:false,html:esc,preview:()=>''});
        p.body.innerHTML=chosen();p.translate(locale);
        assert.equal(p.body.querySelector('strong').textContent,'Title – Help');
        assert.equal(p.body.querySelector('strong + span').textContent,extra.country||t.translate('Barcode Help',0,true));
      }
    }finally{p.close();}
  });
  test(`${locale}: Manage Covers keeps description edits verbatim and translates the cleared state`,async()=>{
    const p=page();
    try{
      const cover={digest:'fixture',mime:'image/png',size:42,width:1,height:1,picture_type:'CoverFront',description:'',files:[0],index:0,data_base64:'AA=='};
      const ui=new CoverUI({reason:()=>'',selection:()=>[{path:'Help',covers:[cover]}],invoke:async()=>({files:1,covers:[cover],errors:[]}),modal:modal(p),clipboard:()=>false,displayed:()=>({path:'Help',covers:[cover],index:0})});
      await ui.manage();const t=p.translate(locale),input=p.body.querySelector('#manage-cover-description');
      const shown=()=>p.body.querySelector('.manage-cover-text > span');
      assert.equal(shown().textContent,t.t('No description'));
      for(const value of ['Help','','No description','']){
        input.value=value;input.dispatchEvent(new p.window.Event('input'));
        await p.window.happyDOM.waitUntilComplete();
        assert.equal(shown().textContent,value||t.t('No description'));
        input.dispatchEvent(new p.window.Event('change'));p.translate(locale);
        assert.equal(shown().textContent,value||t.t('No description'));
      }
    }finally{p.close();}
  });
  test(`${locale}: textarea attributes translate while editable text and values stay verbatim`,async()=>{
    const p=page();
    try{
      p.body.innerHTML='<textarea id="template" aria-label="Export template" placeholder="Help">Help</textarea><textarea id="field" aria-label="Field value 1">Help</textarea><textarea translate="no" aria-label="Export template">Help</textarea>';
      const template=p.body.querySelector('#template'),field=p.body.querySelector('#field');
      template.value='Title';
      const t=p.translate(locale);
      assert.equal(template.getAttribute('aria-label'),t.t('Export template'));
      assert.equal(template.placeholder,t.t('Help'));
      assert.equal(field.getAttribute('aria-label'),t.t('Field value {index}',{index:1}));
      assert.equal(template.textContent,'Help');assert.equal(template.defaultValue,'Help');assert.equal(template.value,'Title');
      assert.equal(field.value,'Help');assert.equal(p.body.querySelector('[translate="no"]').getAttribute('aria-label'),'Export template');
      field.setAttribute('aria-label','Field value 2');
      field.firstChild.data='Title';
      template.replaceChildren(p.window.document.createTextNode('Help'));
      await p.window.happyDOM.waitUntilComplete();
      assert.equal(field.getAttribute('aria-label'),t.t('Field value {index}',{index:2}));
      assert.equal(field.textContent,'Title');assert.equal(field.defaultValue,'Title');assert.equal(field.value,'Title');
      assert.equal(template.textContent,'Help');assert.equal(template.value,'Title');
      p.body.insertAdjacentHTML('beforeend','<textarea id="added" aria-label="Export template">Help</textarea>');
      await p.window.happyDOM.waitUntilComplete();
      assert.equal(p.body.querySelector('#added').getAttribute('aria-label'),t.t('Export template'));
      assert.equal(p.body.querySelector('#added').value,'Help');
      p.translate('en');
      assert.equal(template.getAttribute('aria-label'),'Export template');assert.equal(template.placeholder,'Help');
      assert.equal(field.getAttribute('aria-label'),'Field value 2');assert.equal(field.value,'Title');assert.equal(template.value,'Title');
    }finally{p.close();}
  });
  test(`${locale}: cover accessible labels translate UI states and types alongside verbatim descriptions`,()=>{
    const p=page(),t=new Translator(locale,catalogs[locale]);
    try{
      const cover={index:0,digest:'fixture',mime:'image/png',size:42,width:1,height:1,picture_type:'CoverFront',description:'Help',data_base64:'AA=='};
      let covers=[cover],selection=[{path:'Help',covers}],index=0;
      const ui=new CoverUI({selection:()=>selection,displayed:()=>({path:'Help',covers,index})});
      const options={fallback:'',error:'',unavailable:false,buttons:''};
      const check=(state,extra={})=>{
        p.body.innerHTML=ui.markup({...options,...extra});p.translate(locale);
        const art=p.body.querySelector('.cover-art'),label=art.getAttribute('aria-label');
        assert.ok(label.includes(t.t(state)),`${state}: ${label}`);
        const key=`Cover art: {coverState}${covers.length>1?', {index} of {length}':''}. Enter views it larger; Shift+F10 opens cover commands.`;
        assert.equal(label,t.t(key,{coverState:t.t(state),index:index+1,length:covers.length}));
        assert.equal(art.title,t.t('Right-click for cover commands; double-click to enlarge'));
        assert.equal(p.body.querySelector('[data-cover="menu"]').getAttribute('aria-label'),t.t('Cover commands'));
      };
      for(const [id,label] of PICTURE_TYPES){
        cover.picture_type=id;check(label);
        assert.equal(p.body.querySelector('[data-cover-image]').getAttribute('alt'),t.t('Embedded {pictureTypeLabel} artwork',{pictureTypeLabel:t.t(label)}));
        assert.equal(p.body.querySelector('.cover-info > span[translate="no"]').textContent,'Help');
      }
      cover.picture_type='CoverFront';covers=[cover,{...cover,index:1,picture_type:'CoverBack'}];selection=[{path:'Help',covers}];index=1;
      check('Back Cover');
      for(const command of ['previous','next'])assert.equal(p.body.querySelector(`[data-cover="${command}"]`).getAttribute('aria-label'),t.t(`${command==='previous'?'Previous':'Next'} cover`));
      ui.openViewer();p.translate(locale);
      assert.equal(p.body.querySelector('.cover-viewer').getAttribute('aria-label'),t.t('Cover viewer'));
      assert.equal(p.body.querySelector('.cover-viewer img').alt,t.t('{pictureTypeLabel} artwork',{pictureTypeLabel:t.t('Back Cover')}));
      assert.equal(p.body.querySelector('[data-viewer="close"]').getAttribute('aria-label'),t.t('Close cover viewer'));
      p.body.querySelector('.cover-viewer').close();
      covers=[];selection=[{path:'Help',covers}];index=0;
      check('No artwork');check('Artwork unavailable',{unavailable:true});
      check('Folder image',{folder:{name:'Help',mime:'image/png',size:42,width:1,height:1,data_base64:'AA=='}});
      assert.equal(p.body.querySelector('[data-folder-image]').alt,t.t('Folder image {name}, not embedded',{name:'Help'}));
      selection=[{path:'Help',covers:[cover]},{path:'Title',covers:[]}];check('Covers vary');
      p.translate('en');assert.ok(p.body.querySelector('.cover-art').getAttribute('aria-label').startsWith('Cover art: Covers vary.'));
    }finally{p.close();}
  });
  test(`${locale}: cover descriptions and folder-image names stay verbatim while cover UI translates`,()=>{
    const p=page();
    try{
      const cover={index:0,digest:'fixture',mime:'image/png',size:42,width:1,height:1,picture_type:'CoverFront',description:'Help',data_base64:'AA=='};
      let covers=[cover];
      const ui=new CoverUI({selection:()=>[{path:'one',covers},{path:'two',covers}],displayed:()=>({path:'one',covers,index:0})});
      const options={fallback:'',error:'',unavailable:false,buttons:'',note:'Loading artwork…'};
      p.body.innerHTML=ui.markup(options);let t=p.translate(locale);
      assert.equal(p.body.querySelector('.cover-info strong').textContent,t.t('Front Cover'));
      assert.ok([...p.body.querySelectorAll('.cover-info > span')].some(span=>span.textContent==='Help'));
      assert.ok([...p.body.querySelectorAll('.cover-info > span')].some(span=>span.textContent===t.t('Shared by {selected} files',{selected:2})));
      assert.equal(p.body.querySelector('.cover-info > span:last-child').textContent,t.t('Loading artwork…'));
      cover.description='Loading artwork…';p.body.innerHTML=ui.markup(options);p.translate(locale);
      assert.equal(p.body.querySelectorAll('.cover-info > span')[1].textContent,'Loading artwork…');
      assert.equal(p.body.querySelector('.cover-info > span:last-child').textContent,t.t('Loading artwork…'));
      covers=[];p.body.innerHTML=ui.markup({...options,folder:{name:'Help',mime:'image/png',size:42,width:1,height:1,data_base64:'AA=='}});p.translate(locale);
      assert.equal(p.body.querySelector('.cover-info strong').textContent,t.t('Folder image'));
      assert.equal(p.body.querySelector('.cover-info > span').textContent,'Help');
      assert.equal(p.body.querySelector('[data-folder-image]').getAttribute('alt'),t.t('Folder image {name}, not embedded',{name:'Help'}));
    }finally{p.close();}
  });
  test(`${locale}: editors translate shipped names and retain custom names through editing`,()=>{
    const p=page(),t=new Translator(locale,catalogs[locale]);
    setColumnNameTranslation(label=>t.translate(label));
    try{
      const layout=defaultLayout();layout.columns=[...layout.columns.slice(0,2),{...layout.columns[1],id:'custom',name:'Help'}];
      new ColumnUI({layout:()=>layout}).customize();
      const panel=new TagPanel({translateLabel:label=>t.translate(label),values:()=>[],disabled:()=>false,directories:()=>({selected:[],all:[]})});
      const title=defaultPanelLayout().fields.find(field=>field.field==='TITLE');
      panel.settings.panel.fields=[title,{...title,field:'CUSTOM',name:'Help'}];
      p.body.insertAdjacentHTML('beforeend',panel.fieldsHTML());panel.bind(p.body.querySelector('.panel-grid'));
      p.body.querySelector('.combo-toggle').click();p.translate(locale);
      assert.equal(p.body.querySelector('.combo-list').getAttribute('aria-label'),t.t('{label} values',{label:t.t('Title')}));
      assert.deepEqual([...p.body.querySelectorAll('.panel-grid > .field > label')].map(label=>label.textContent),[t.t('Title'),'Help']);
      panel.closeList();panel.configure();
      p.translate(locale);
      const columnRows=[...p.body.querySelectorAll('.column-list li')],panelRows=[...p.body.querySelectorAll('.panel-config-row')];
      assert.deepEqual(columnRows.map(row=>row.querySelector('span').textContent),[t.t('Filename'),t.t('Title'),'Help']);
      for(const [i,name] of [t.t('Filename'),t.t('Title'),'Help'].entries())assert.equal(columnRows[i].querySelector('input').getAttribute('aria-label'),t.t('Show {name}',{name}));
      assert.deepEqual(panelRows.map(row=>row.querySelector('span').textContent),[t.t('Title'),'Help']);
      assert.deepEqual(panelRows.map(row=>row.querySelector('small').textContent),['TITLE','CUSTOM']);
      for(const [i,name] of [t.t('Title'),'Help'].entries())assert.equal(panelRows[i].querySelector('input').getAttribute('aria-label'),t.t('Show {name}',{name}));
      assert.equal(p.body.querySelector('#column-name').value,'Filename');assert.equal(p.body.querySelector('#tp-name').value,'Tit&le');
      const columnName=p.body.querySelector('#column-name'),panelName=p.body.querySelector('#tp-name');
      for(const name of ['Help','Filename']){
        columnName.value=name;columnName.dispatchEvent(new p.window.Event('input',{bubbles:true}));p.translate(locale);
        const shown=name==='Filename'?t.t(name):name;
        assert.equal(columnRows[0].querySelector('span').textContent,shown);
        assert.equal(columnRows[0].querySelector('input').getAttribute('aria-label'),t.t('Show {name}',{name:shown}));
      }
      for(const name of ['Help','Tit&le']){
        panelName.value=name;panelName.dispatchEvent(new p.window.Event('input',{bubbles:true}));p.translate(locale);
        const shown=name==='Tit&le'?t.t('Title'):name;
        assert.equal(panelRows[0].querySelector('span').textContent,shown);
        assert.equal(panelRows[0].querySelector('input').getAttribute('aria-label'),t.t('Show {name}',{name:shown}));
      }
    }finally{setColumnNameTranslation(label=>label);p.close();}
  });
  test(`${locale}: file-row data stays verbatim while checkbox and pending tooltip translate`,()=>{
    const p=page();
    try{
      p.body.innerHTML='<table>'+renderRows({path:'/Help',filename:'Help',format:'MP3',fields:{TITLE:['Help']}})+'</table>';
      const t=p.translate(locale),checkbox=p.body.querySelector('.row-select'),cell=p.body.querySelector('td');
      assert.equal(checkbox.getAttribute('aria-label'),t.t('Select {filename}',{filename:'Help'}));
      assert.equal(p.body.querySelector('.rowmarker').title,t.t('Pending changes'));
      assert.equal(cell.querySelector('[translate="no"]').title,'Help');assert.equal(cell.textContent,'●Help');
      p.translate('en');assert.equal(checkbox.getAttribute('aria-label'),'Select Help');assert.equal(cell.querySelector('[translate="no"]').title,'Help');
    }finally{p.close();}
  });
  test(`${locale}: custom columns, action groups and tools keep names in text and accessible labels`,async()=>{
    const p=page();
    try{
      const layout=defaultLayout();layout.columns=[{...layout.columns[0],name:'Help'}];
      new ColumnUI({layout:()=>layout}).customize();
      const actions=new ActionUI({load:async()=>({groups:[{name:'Help',enabled:true,actions:[{type:'format_value',field:'Help',format:'Help'}]}],presets:[]}),canExecute:()=>true,selectedCount:()=>1,changed:()=>{}});
      await actions.groupsDialog();
      const tool={name:'Help',path:'Help',parameters:'',for_each_file:false};
      const tools=new ToolsUI({values:()=>({tools:[tool]}),reason:()=>'',mac:false});
      p.body.insertAdjacentHTML('beforeend',`<nav>${tools.menuHTML()}</nav>`);
      p.body.insertAdjacentHTML('beforeend',contextMenuMarkup([{label:'Help',id:'tool:0',title:'Help',userLabel:true,userTitle:true},{label:'Help',id:'tool:1',title:'Select files first.',disabled:true,userLabel:true}]));
      const options=toolsOptionsPage({pickProgram:async()=>null});
      p.body.insertAdjacentHTML('beforeend',options.controls[0].render({pages:{tools:{tools:[tool]}}}));
      const t=p.translate(locale);
      assert.equal(p.body.querySelector('[data-visible]').getAttribute('aria-label'),t.t('Show {name}',{name:'Help'}));
      assert.equal(p.body.querySelector('.column-list span').textContent,'Help');
      assert.equal(p.body.querySelector('[data-enable]').getAttribute('aria-label'),t.t('Enable {name}',{name:'Help'}));
      assert.equal(p.body.querySelector('.group-actions .action-section-title').textContent,'Help');
      assert.equal(p.body.querySelector('.action-summary-list button span').textContent,'Help');
      assert.equal(p.body.querySelector('[data-tool] span').textContent,'Help');
      assert.equal(p.body.querySelector('[data-tool]').title,'Help');
      assert.equal(p.body.querySelector('[data-context="tool:0"] span').textContent,'Help');
      assert.equal(p.body.querySelector('[data-context="tool:0"]').title,'Help');
      assert.equal(p.body.querySelector('[data-context="tool:1"]').title,t.t('Select files first.'));
      assert.equal(p.body.querySelector('#tools-list option').textContent,'Help');
      p.body.querySelector('.action-groups-dialog [data-command="edit"]').click();p.translate(locale);
      assert.equal(p.body.querySelector('.group-editor-dialog header').textContent,t.t('Actions — {name}',{name:'Help'}));
      assert.equal(p.body.querySelector('.action-edit-list button span').textContent,'Help');
    }finally{p.close();}
  });
  test(`${locale}: saved-history values stay verbatim while controls and empty state translate`,()=>{
    const p=page();
    try{
      p.body.innerHTML='<div><input value="Help" data-helper="history" data-history="probe" data-history-manage></div>';
      const helpers=installHelperMenus({storage:memory(),fieldNames:()=>[],panelFields:()=>[]});
      helpers.history.set('probe',[{name:'Help',value:'Help',description:'Help',favorite:true}]);
      p.body.querySelector('.helper-button').click();
      p.translate(locale);assert.equal([...p.body.querySelectorAll('.helper-menu button')].find(el=>el.textContent==='Help')?.textContent,'Help');
      [...p.body.querySelectorAll('.helper-menu button')].find(el=>el.textContent===catalogs[locale]['Manage history…']).click();
      const t=p.translate(locale),checkbox=p.body.querySelector('[data-favorite]');
      assert.equal(checkbox.title,t.t('Favorite'));
      assert.equal(checkbox.getAttribute('aria-label'),t.t('Favorite: {name}',{name:'Help'}));
      assert.deepEqual([...p.body.querySelectorAll('.history-text > *')].map(el=>el.textContent),['Help','Help','Help']);
      p.body.querySelector('[data-remove]').click();p.translate(locale);
      assert.equal(p.body.querySelector('.history-list').getAttribute('aria-label'),t.t('History'));
      assert.equal(p.body.querySelector('.history-list .group-empty').textContent,t.t('No saved format strings.'));
      p.translate('en');assert.equal(p.body.querySelector('.history-list .group-empty').textContent,'No saved format strings.');
    }finally{p.close();}
  });
}

for(const [ending,newline] of [['LF','\n'],['CRLF','\r\n']]){
  test(`native dialogs honor an active English user catalog and preserve picker data (${ending} source)`,async()=>{
    const nativeSource=(await source('i18n/dialog.ts')).replace(/\r?\n/g,newline).replace(/^import .*;\r?\n/gm,'').replace(/^export /gm,'');
    const js=ts.transpileModule(`export function create(dialog,translator){
      const t=(key)=>translator.t(key),translate=(text)=>translator.translate(text),translationActive=()=>translator.active;
      ${nativeSource}
      return {open,save,ask,confirm,message};
    }`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
    const {create}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
    const calls=[],native=Object.fromEntries(['open','save','ask','confirm','message'].map(name=>[name,(...args)=>(calls.push([name,...args]),args)]));
    const custom=new Translator('en',{'Open files':'Choose tracks','Audio files':'Music','Save tags':'Write metadata',Yes:'Proceed',No:'Decline',OK:'Accept',Cancel:'Back'});
    const dialogs=create(native,custom),picker={title:'Open files',defaultPath:'Help',filters:[{name:'Audio files',extensions:['mp3']}]};
    assert.deepEqual(dialogs.open(picker)[0],{title:'Choose tracks',defaultPath:'Help',filters:[{name:'Music',extensions:['mp3']}]});
    assert.equal(dialogs.save(picker)[0].title,'Choose tracks');
    assert.deepEqual(dialogs.ask('Save tags','Open files'),['Write metadata',{title:'Choose tracks',okLabel:'Proceed',cancelLabel:'Decline'}]);
    assert.deepEqual(dialogs.confirm('Save tags'),['Write metadata',{okLabel:'Accept',cancelLabel:'Back'}]);
    assert.deepEqual(dialogs.message('Save tags'),['Write metadata',{buttons:{ok:'Accept'}}]);
    assert.deepEqual(picker,{title:'Open files',defaultPath:'Help',filters:[{name:'Audio files',extensions:['mp3']}]});
    assert.equal(create(native,new Translator('en')).open(picker)[0],picker);
    assert.deepEqual(create(native,new Translator('en')).ask({parts:['Help','Title'],separator:'\n\n'},'Open files'),['Help\n\nTitle','Open files']);
  });
}
