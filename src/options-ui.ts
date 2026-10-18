// File → Options… (https://docs.mp3tag.de/customization/options/). Built-in pages
// are declarative; other features add pages with registerOptionsPage().
export type Id3v2Format='v23_utf16'|'v23_latin1'|'v24_utf8';
export type TagOptions={
  mpeg:{read_id3v1:boolean;read_id3v2:boolean;read_ape:boolean;write_id3v1:boolean;write_id3v2:boolean;write_ape:boolean;id3v2_only_if_needed:boolean;id3v2_format:Id3v2Format;remove_id3v1:boolean;remove_id3v2:boolean;remove_ape:boolean};
  ape:{read_id3v1:boolean;read_ape:boolean;write_id3v1:boolean;write_ape:boolean;remove_id3v1:boolean;remove_id3v2:boolean;remove_ape:boolean};
  advanced:{id3v2_comment_language:string;popularimeter_ratings:boolean;matroska_video_mapping:boolean;list_chapters:boolean;mp4_single_atom:boolean;reuse_mp4_atoms:boolean;reuse_id3v2_frames:boolean};
  preserve_modified_time:boolean;
  /** Options → Tags → Mapping, edited by the page in field-mapping.ts. */
  mappings:{tag_type:string;source:string;target:string}[];
};
export type Settings={version:number;general:{natural_sort:boolean;show_directory_in_title:boolean};files:{restrict_extensions:string;folder_separator:string};directories:{subfolders:boolean};tags:TagOptions;pages:Record<string,Record<string,unknown>>};
export type SettingsResponse={settings:Settings;defaults:Settings;warning:string|null};
/** Values edited in the dialog: persisted settings plus the per-device appearance choice. */
export type OptionsDraft=Settings&{appearance:string};
/** The optional flag protects authored option text; ordinary choices remain UI labels. */
export type SelectChoice=[value:string,label:string,verbatim?:boolean];
export type OptionControl=
  |{kind:'checkbox';path:string;label:string;enabledWhen?:(draft:OptionsDraft)=>boolean}
  |{kind:'radio';path:string;label:string;choices:[string,string][];enabledWhen?:(draft:OptionsDraft)=>boolean}
  |{kind:'select';path:string;label:string;choices:SelectChoice[];missingChoiceLabel?:(value:string)=>string;matchesChoice?:(stored:unknown,value:string)=>boolean}
  |{kind:'text';path:string;label:string;placeholder?:string;maxLength?:number;enabledWhen?:(draft:OptionsDraft)=>boolean}
  /** Whole numbers; the value is stored as a number. */
  |{kind:'number';path:string;label:string;min:number;max:number;suffix?:string}
  |{kind:'group';label:string;controls:OptionControl[]}
  |{kind:'note';text:string}
  /** Runs `run` when pressed, for example to open a dedicated editor. */
  |{kind:'button';id:string;label:string;run:(draft:OptionsDraft)=>void|Promise<void>}
  /** Free-form markup bound by the page; `paths` are restored by Restore page defaults. */
  |{kind:'custom';id:string;paths:string[];render:(draft:OptionsDraft)=>string;bind:(root:HTMLElement,draft:OptionsDraft,refresh:()=>void)=>void};
export type OptionsPage={id:string;label:string;parent?:string;order?:number;description?:string;controls:OptionControl[];validate?:(draft:OptionsDraft)=>string|undefined;
  /** Default values of `pages.<id>.*`, used while a value has never been saved. */
  defaults?:Record<string,unknown>};
export type OptionsHost={
  load:()=>Promise<SettingsResponse>;save:(settings:Settings)=>Promise<SettingsResponse>;
  appearance:{get:()=>string;set:(value:string)=>void};
  applied:(next:Settings,previous:Settings)=>void|Promise<void>;
};

export const defaultSettings=():Settings=>({version:1,general:{natural_sort:true,show_directory_in_title:true},files:{restrict_extensions:'',folder_separator:'_'},directories:{subfolders:true},
  tags:{mpeg:{read_id3v1:true,read_id3v2:true,read_ape:true,write_id3v1:true,write_id3v2:true,write_ape:false,id3v2_only_if_needed:false,id3v2_format:'v23_utf16',remove_id3v1:true,remove_id3v2:true,remove_ape:true},
    ape:{read_id3v1:true,read_ape:true,write_id3v1:false,write_ape:true,remove_id3v1:true,remove_id3v2:false,remove_ape:true},
    advanced:{id3v2_comment_language:'eng',popularimeter_ratings:false,matroska_video_mapping:true,list_chapters:false,mp4_single_atom:true,reuse_mp4_atoms:false,reuse_id3v2_frames:false},preserve_modified_time:false,mappings:[]},pages:{}});

/** pages.general: General options beyond the window title and sorting. */
export const GENERAL_DEFAULTS={auto_select_next:false,autocomplete:true,select_added:false,keep_moved:true,tray_icon:false,minimize_to_tray:false};
// Notification area options work on Windows and Linux; the app sets this at start-up.
let notificationArea=true;
export function setNotificationAreaSupport(supported:boolean){notificationArea=supported;}
/** pages.tags: saving when moving to another file and the folder-image fallback of the cover pane. */
export const TAGS_PAGE_DEFAULTS={save_on_move:false,no_folder_image:false,extended_save_mode:'pending'};
/** pages.appearance: scaling in percent and the file list grid. */
export const APPEARANCE_DEFAULTS={toolbar_scale:'100',ui_scale:'100',grid:'off'};
const SCALES:[string,string][]=['100','110','125','150','175','200'].map(v=>[v,`${v}%`]);

const pageId=/^[a-z0-9][a-z0-9_-]*$/;
export function getPath(value:unknown,path:string):unknown{return path.split('.').reduce<unknown>((current,key)=>current&&typeof current==='object'?(current as Record<string,unknown>)[key]:undefined,value);}
export function setPath<T extends object>(value:T,path:string,next:unknown):T{
  const keys=path.split('.'),last=keys.pop()!;let current=value as Record<string,unknown>;
  for(const key of keys){if(!current[key]||typeof current[key]!=='object')current[key]={};current=current[key] as Record<string,unknown>;}
  current[last]=next;return value;
}

const registry=new Map<string,OptionsPage>();
/** Add an Options page. Values under `pages.<id>.*` persist without backend changes. */
export function registerOptionsPage(page:OptionsPage){
  if(!pageId.test(page.id))throw new Error(`Invalid options page identifier: ${page.id}`);
  if(registry.has(page.id))throw new Error(`Options page ${page.id} is already registered`);
  registry.set(page.id,page);
}
/** Values of a registered page with its defaults filled in. */
export function pageValues(settings:Pick<Settings,'pages'>,id:string):Record<string,unknown>{
  return{...structuredClone(registry.get(id)?.defaults??{}),...(settings.pages[id] as Record<string,unknown>|undefined)};
}
/** Settings with every registered page's defaults filled in. */
export function withPageDefaults<T extends Pick<Settings,'pages'>>(settings:T,pages:OptionsPage[]=[...registry.values()]):T{
  for(const page of pages)if(page.defaults)settings.pages[page.id]={...structuredClone(page.defaults),...settings.pages[page.id]};
  return settings;
}
/** Pages in display order with their tree depth. Unknown parents become top-level pages. */
export function pageTree(pages:OptionsPage[]=[...registry.values()]):{page:OptionsPage;depth:number}[]{
  const known=new Set(pages.map(p=>p.id)),output:{page:OptionsPage;depth:number}[]=[];
  const children=(parent:string|undefined)=>pages.map((page,index)=>({page,index})).filter(({page})=>(page.parent&&known.has(page.parent)?page.parent:undefined)===parent)
    .sort((a,b)=>(a.page.order??1000)-(b.page.order??1000)||a.index-b.index).map(({page})=>page);
  const visit=(parent:string|undefined,depth:number)=>children(parent).forEach(page=>{output.push({page,depth});visit(page.id,depth+1);});
  visit(undefined,0);return output;
}

export function parseExtensions(value:string):string[]|null{
  const output:string[]=[];
  for(const item of value.split(/[;,\s]+/).filter(Boolean)){
    const extension=item.replace(/^\*?\./,'').toLowerCase();
    if(!/^[a-z0-9+]+$/.test(extension))throw new Error(`Invalid file extension: ${item}`);
    if(!output.includes(extension))output.push(extension);
  }
  return output.length?output:null;
}
/** Mirror of `tag_core::validate_folder_separator`: the replacement for \ and / in tag values must not itself be a separator or an invalid filename character. */
export function folderSeparatorProblem(value:string){
  if([...value].length>16)return'The folder separator replacement is limited to 16 characters.';
  return/[\\/:*?"<>|\u0000-\u001f]/.test(value)?'The folder separator replacement cannot contain \\ / : * ? " < > | or control characters.':undefined;
}
/** Mirror of the backend validation, reported with the page that needs attention. */
export function validateOptions(draft:OptionsDraft,pages:OptionsPage[]=[...registry.values()]):{page:string;message:string}|undefined{
  const {mpeg,ape,advanced}=draft.tags;
  if(!/^[A-Za-z]{3}$/.test(advanced.id3v2_comment_language))return{page:'tags-advanced',message:'The ID3v2 comment language must be a three-letter ISO-639-2 code.'};
  if(!mpeg.write_id3v1&&!mpeg.write_id3v2&&!mpeg.write_ape)return{page:'tags-mpeg',message:'Enable at least one tag type to write for MPEG files.'};
  if(!ape.write_id3v1&&!ape.write_ape)return{page:'tags-ape',message:'Enable at least one tag type to write for APE, MPC and WavPack files.'};
  try{parseExtensions(draft.files.restrict_extensions);}catch(error){return{page:'files',message:String((error as Error).message)};}
  const separatorProblem=folderSeparatorProblem(draft.files.folder_separator??'_');if(separatorProblem)return{page:'files',message:separatorProblem};
  for(const page of pages){const message=page.validate?.(draft);if(message)return{page:page.id,message};}
  return undefined;
}
export function naturalCollator(settings:Settings){return new Intl.Collator(undefined,{numeric:settings.general.natural_sort,sensitivity:'base'});}
export function windowTitle(sourceLocation:string,settings:Settings,presentation:{isData?:boolean;translate?:(label:string)=>string}={}){
  if(!settings.general.show_directory_in_title||!sourceLocation||!presentation.isData&&sourceLocation==='No folder loaded')return'TagGroove';
  const location=presentation.isData?sourceLocation:presentation.translate?.(sourceLocation)??sourceLocation;
  return`TagGroove — ${location}`;
}

const tagRead=(prefix:string,kinds:[string,string][]):OptionControl=>({kind:'group',label:'Read',controls:kinds.map(([key,label])=>({kind:'checkbox',path:`tags.${prefix}.read_${key}`,label}))});
const tagRemove=(prefix:string):OptionControl=>({kind:'group',label:'Remove',controls:[...[['id3v1','ID3v1'],['id3v2','ID3v2'],['ape','APE']].map(([key,label]):OptionControl=>({kind:'checkbox',path:`tags.${prefix}.remove_${key}`,label})),
  {kind:'note',text:'File → Remove tag (⌘/Ctrl R) removes these tag types. Saving also removes a checked type that is not written.'}]});
for(const page of [
  {id:'general',label:'General',order:10,defaults:{...GENERAL_DEFAULTS},controls:[
    {kind:'checkbox',path:'general.show_directory_in_title',label:'Show current directory in window title'},
    {kind:'checkbox',path:'pages.general.auto_select_next',label:'Automatically select next file when editing in file view'},
    {kind:'checkbox',path:'pages.general.autocomplete',label:'Auto-completion at selection boxes'},
    {kind:'checkbox',path:'general.natural_sort',label:'Use natural sorting (1, 2, 11, …)'},
    {kind:'checkbox',path:'pages.general.select_added',label:'Select files automatically when they are added to the list'},
    {kind:'checkbox',path:'pages.general.keep_moved',label:'Keep moved files in the file list'},
    {kind:'note',text:'Auto-completion completes tag panel fields and file list cells from their drop-down values. Moved files that are no longer below a listed folder leave the list unless they have pending changes.'},
    {kind:'group',label:'Notification area',controls:[
      {kind:'checkbox',path:'pages.general.tray_icon',label:'Always show icon in notification area',enabledWhen:()=>notificationArea},
      {kind:'checkbox',path:'pages.general.minimize_to_tray',label:'Minimize to notification area',enabledWhen:()=>notificationArea},
      {kind:'note',text:'Minimizing then hides the window and its task bar entry; click the icon or choose Show TagGroove in its menu to bring it back. Windows and Linux only (Linux needs libayatana-appindicator); macOS keeps minimized windows in the Dock.'}]}]},
  {id:'files',label:'Files',order:20,controls:[
    {kind:'group',label:'Associated files',controls:[
      {kind:'checkbox',path:'pages.files.associated_files',label:'Move matching .lrc and .cdg files with audio'},
      {kind:'note',text:'Off by default. Applies to Tag → Filename and Move files to. Review the companion paths before applying. Only files with exactly the same basename and a .lrc or .cdg extension follow the audio. Copy, other converters and action-driven renames do not use this option.'}]},
    {kind:'group',label:'Read files',controls:[{kind:'text',path:'files.restrict_extensions',label:'Restrict incoming files to',placeholder:'All supported formats (for example *.mp3;*.flac)'},
      {kind:'note',text:'Applies to folders. Files you add individually are always read.'}]},
    {kind:'group',label:'Rename files from tags',controls:[{kind:'text',path:'files.folder_separator',label:'Replace folder separator in tag values with',placeholder:'Nothing: remove separators',maxLength:16},
      {kind:'note',text:'Replaces \\ and / inside tag values when Tag → Filename, Format Value, Guess Values or Tag → Tag rename files, so a value such as Rock\\Classic does not create a folder. Leave empty to remove them. Separators typed in the format string itself still create folders.'}]},
    {kind:'group',label:'Write files',controls:[{kind:'checkbox',path:'tags.preserve_modified_time',label:'Preserve timestamps of modified files'}]}]},
  {id:'directories',label:'Directories',order:30,controls:[{kind:'checkbox',path:'directories.subfolders',label:'Read subfolders'},
    {kind:'group',label:'Cover art directory',controls:[{kind:'text',path:'pages.directories.cover_art',label:'Default folder for Add cover',placeholder:'Folder of the selected file'},
      {kind:'note',text:'A full path, or a path relative to the first selected file’s folder; placeholders such as %artist% work. If it is empty or does not exist, Add cover opens the file’s folder when it holds an image, else the working folder.'}]}]},
  {id:'appearance',label:'Appearance',order:40,defaults:{...APPEARANCE_DEFAULTS},controls:[{kind:'select',path:'appearance',label:'Color mode',choices:[['system','System'],['light','Light'],['dark','Dark']]},
    {kind:'note',text:'Color mode is remembered on this device and also available in the title bar.'},
    {kind:'select',path:'pages.appearance.toolbar_scale',label:'Toolbar scaling',choices:SCALES},
    {kind:'select',path:'pages.appearance.ui_scale',label:'User interface scaling',choices:SCALES},
    {kind:'radio',path:'pages.appearance.grid',label:'Display grid lines in file view',choices:[['off','Off'],['alternate','Alternate row colors'],['lines','Grid lines']]}]},
  {id:'tags',label:'Tags',order:50,description:'Choose which tag types are read, written and removed for each format family.',defaults:{...TAGS_PAGE_DEFAULTS},controls:[
    {kind:'checkbox',path:'pages.tags.save_on_move',label:'Save tags when using arrow keys or a single mouse click'},
    {kind:'select',path:'pages.tags.extended_save_mode',label:'Save mode at extended tag dialog',choices:[['pending','Keep changes pending'],['ask','Ask'],['ignore','Ignore changes'],['save','Save changes']]},
    {kind:'checkbox',path:'pages.tags.no_folder_image',label:'Don’t display first image from file directory as cover art'},
    {kind:'note',text:'Saving on selection change writes pending tag panel edits when you move to another file with the mouse, the arrow keys in the file list or Select next file. Without embedded cover art, the cover pane shows the first image of the file’s folder; that image is not embedded. The save mode decides what happens to changed fields when << or >> in Extended tags moves to another file: they stay pending until Save tags, TagGroove asks, they are discarded, or the file is saved.'},
    {kind:'note',text:'Mpeg applies to MP3, AAC and TTA files. Ape, Mpc applies to Monkey’s Audio, Musepack, WavPack, TAK and OptimFROG files. Other formats always use their native tag, and Remove tag removes all of their tags.'}]},
  {id:'tags-mpeg',parent:'tags',label:'Mpeg',order:10,description:'Only one tag type is displayed: APE before ID3v2 before ID3v1, among the types enabled for reading.',controls:[
    tagRead('mpeg',[['id3v1','ID3v1'],['id3v2','ID3v2'],['ape','APE']]),
    {kind:'group',label:'Write',controls:[
      {kind:'checkbox',path:'tags.mpeg.write_id3v1',label:'ID3v1'},
      {kind:'checkbox',path:'tags.mpeg.write_id3v2',label:'ID3v2'},
      {kind:'checkbox',path:'tags.mpeg.id3v2_only_if_needed',label:'ID3v2 only if ID3v1 too small',enabledWhen:d=>d.tags.mpeg.write_id3v1&&d.tags.mpeg.write_id3v2},
      {kind:'radio',path:'tags.mpeg.id3v2_format',label:'ID3v2 version',enabledWhen:d=>d.tags.mpeg.write_id3v2,choices:[['v24_utf8','ID3v2.4 UTF-8'],['v23_utf16','ID3v2.3 UTF-16'],['v23_latin1','ID3v2.3 ISO-8859-1']]},
      {kind:'checkbox',path:'tags.mpeg.write_ape',label:'APEv2 (also updates an existing ID3v1 tag)'}]},
    tagRemove('mpeg')]},
  {id:'tags-ape',parent:'tags',label:'Ape, Mpc',order:20,description:'APE is displayed before ID3v1 when both are read.',controls:[
    tagRead('ape',[['id3v1','ID3v1'],['ape','APE']]),
    {kind:'group',label:'Write',controls:[{kind:'checkbox',path:'tags.ape.write_id3v1',label:'ID3v1'},{kind:'checkbox',path:'tags.ape.write_ape',label:'APEv2 (also updates an existing ID3v1 tag)'}]},
    tagRemove('ape')]},
  {id:'tags-advanced',parent:'tags',label:'Advanced',order:40,controls:[
    {kind:'text',path:'tags.advanced.id3v2_comment_language',label:'ID3v2 comment language code (ISO-639-2)',maxLength:3},
    {kind:'checkbox',path:'tags.advanced.popularimeter_ratings',label:'Always display POPULARIMETER values for ID3v2 ratings'},
    {kind:'checkbox',path:'tags.advanced.matroska_video_mapping',label:'Use recommended mapping for Matroska video'},
    {kind:'checkbox',path:'tags.advanced.list_chapters',label:'List chapters as separate files'},
    {kind:'checkbox',path:'tags.advanced.mp4_single_atom',label:'Use single MP4 atom for multiple values'},
    {kind:'checkbox',path:'tags.advanced.reuse_mp4_atoms',label:'Reuse unmodified MP4 atoms'},
    {kind:'checkbox',path:'tags.advanced.reuse_id3v2_frames',label:'Reuse unmodified ID3v2 frames'},
    {kind:'note',text:'Reload files to see changed rating fields and Matroska mappings, and reload the directory to list or hide chapters.'}]},
] as OptionsPage[])registerOptionsPage(page);

const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const controlId=(path:string,suffix='')=>`option-${path.replace(/[^a-z0-9]+/gi,'-')}${suffix}`;
export function controlMarkup(control:OptionControl,draft:OptionsDraft):string{
  const disabled=(c:{enabledWhen?:(d:OptionsDraft)=>boolean})=>c.enabledWhen&&!c.enabledWhen(draft)?'disabled':'';
  switch(control.kind){
  case 'checkbox':return`<label class="action-check"><input type="checkbox" id="${controlId(control.path)}" data-option="${html(control.path)}" ${getPath(draft,control.path)?'checked':''} ${disabled(control)}> ${html(control.label)}</label>`;
  case 'radio':return`<fieldset class="options-choices" ${disabled(control)}><legend>${html(control.label)}</legend>${control.choices.map(([value,label])=>`<label class="action-check"><input type="radio" name="${controlId(control.path)}" data-option="${html(control.path)}" value="${html(value)}" ${getPath(draft,control.path)===value?'checked':''}> ${html(label)}</label>`).join('')}</fieldset>`;
  case 'select':{
    const stored=getPath(draft,control.path),matches=(value:string)=>control.matchesChoice?.(stored,value)??stored===value;
    const missing=typeof stored==='string'&&stored&&!control.choices.some(([value])=>matches(value))&&control.missingChoiceLabel;
    const unavailable=missing?`<option value="${html(stored)}" selected>${html(control.missingChoiceLabel!(stored))}</option>`:'';
    return`<label class="field"><span>${html(control.label)}</span><select id="${controlId(control.path)}" data-option="${html(control.path)}">${unavailable}${control.choices.map(([value,label,verbatim])=>`<option value="${html(value)}" ${verbatim?'translate="no" ':''}${matches(value)?'selected':''}>${html(label)}</option>`).join('')}</select></label>`;
  }
  case 'text':return`<label class="field"><span>${html(control.label)}</span><input id="${controlId(control.path)}" data-option="${html(control.path)}" value="${html(getPath(draft,control.path)??'')}" ${control.placeholder?`placeholder="${html(control.placeholder)}"`:''} ${control.maxLength?`maxlength="${control.maxLength}"`:''} autocomplete="off" spellcheck="false" ${disabled(control)}></label>`;
  case 'number':return`<label class="field"><span>${html(control.label)}</span><span class="options-number"><input type="number" id="${controlId(control.path)}" data-option="${html(control.path)}" data-number value="${html(getPath(draft,control.path)??'')}" min="${control.min}" max="${control.max}" step="1">${control.suffix?` ${html(control.suffix)}`:''}</span></label>`;
  case 'group':return`<fieldset class="numbering-section"><legend>${html(control.label)}</legend>${control.controls.map(c=>controlMarkup(c,draft)).join('')}</fieldset>`;
  case 'note':return`<p>${html(control.text)}</p>`;
  case 'button':return`<p><button type="button" id="option-button-${html(control.id)}" data-option-button="${html(control.id)}">${html(control.label)}</button></p>`;
  case 'custom':return`<div class="options-custom" data-option-custom="${html(control.id)}">${control.render(draft)}</div>`;
  }
}
const pathsOf=(controls:OptionControl[]):string[]=>controls.flatMap(c=>c.kind==='group'?pathsOf(c.controls):c.kind==='custom'?c.paths:'path' in c?[c.path]:[]);
const flatControls=(controls:OptionControl[]):OptionControl[]=>controls.flatMap(c=>c.kind==='group'?flatControls(c.controls):[c]);

export class OptionsUI {
  private showing=false;
  private current:Settings=defaultSettings();
  constructor(private host:OptionsHost){}
  get settings():Settings{return this.current;}
  /** Saved values of a registered page, with its defaults. */
  page(id:string):Record<string,unknown>{return pageValues(this.current,id);}
  /** Persist values of one page outside the dialog (for example a remembered format string). */
  async updatePage(id:string,values:Record<string,unknown>){
    const response=await this.host.load(),next={...response.settings,pages:{...response.settings.pages,[id]:{...response.settings.pages[id],...values}}};
    this.current=(await this.host.save(next)).settings;
  }
  async initialize(){const response=await this.host.load();this.current=response.settings;return response;}
  async open(initialPage='general'){
    if(this.showing)return;this.showing=true;
    let response:SettingsResponse;
    try{response=await this.host.load();}catch(error){this.showing=false;throw error;}
    this.current=response.settings;
    const draft:OptionsDraft=withPageDefaults({...structuredClone(response.settings),appearance:this.host.appearance.get()});
    const tree=pageTree();let active=tree.some(({page})=>page.id===initialPage)?initialPage:tree[0].page.id,saving=false;
    const dialog=document.createElement('dialog');dialog.className='options-dialog';dialog.setAttribute('aria-labelledby','options-title');document.body.append(dialog);
    const error=(message:string)=>{let p=dialog.querySelector<HTMLElement>('.dialog-error');if(!p){p=document.createElement('p');p.className='dialog-error';p.setAttribute('role','alert');dialog.querySelector('.options-page')!.append(p);}p.textContent=message;};
    const renderPage=(focusTree=false)=>{
      const page=tree.find(({page})=>page.id===active)!.page;
      dialog.querySelectorAll<HTMLButtonElement>('[role="treeitem"]').forEach(item=>{const selected=item.dataset.page===active;item.setAttribute('aria-selected',String(selected));item.tabIndex=selected?0:-1;if(selected&&focusTree)item.focus();});
      dialog.querySelector<HTMLSelectElement>('#options-page-picker')!.value=active;
      const section=dialog.querySelector<HTMLElement>('.options-page')!;
      section.innerHTML=`<h2 id="options-page-title">${html(page.label)}</h2>${page.description?`<p>${html(page.description)}</p>`:''}${page.controls.map(c=>controlMarkup(c,draft)).join('')}`;
      for(const control of flatControls(page.controls)){
        if(control.kind==='button')section.querySelector(`[data-option-button="${CSS.escape(control.id)}"]`)?.addEventListener('click',async()=>{try{await control.run(draft);}catch(e){error(String(e));}});
        if(control.kind==='custom'){const root=section.querySelector<HTMLElement>(`[data-option-custom="${CSS.escape(control.id)}"]`);if(root){const refresh=()=>{root.innerHTML=control.render(draft);dialog.querySelector('.options-page .dialog-error')?.remove();control.bind(root,draft,refresh);};control.bind(root,draft,refresh);}}
      }
    };
    const select=(id:string,focusTree=false)=>{active=id;renderPage(focusTree);};
    dialog.innerHTML=`<header id="options-title">Options</header><div class="dialog-content options-layout">${response.warning?`<p class="dialog-error options-warning" role="alert">${html(response.warning)}</p>`:''}
      <label class="field options-page-select"><span>Options page</span><select id="options-page-picker">${tree.map(({page,depth})=>`<option value="${html(page.id)}">${depth?'— ':''}${html(page.label)}</option>`).join('')}</select></label>
      <div class="options-tree" role="tree" aria-label="Options pages">${tree.map(({page,depth})=>`<button type="button" role="treeitem" aria-level="${depth+1}" data-page="${html(page.id)}">${html(page.label)}</button>`).join('')}</div>
      <section class="options-page" aria-labelledby="options-page-title" aria-live="polite"></section></div>
      <div class="dialog-footer"><button id="options-defaults" type="button">Restore page defaults</button><span class="spacer"></span><button id="options-cancel" type="button">Cancel</button><button id="options-ok" class="primary" type="button">OK</button></div>`;
    const items=()=>[...dialog.querySelectorAll<HTMLButtonElement>('[role="treeitem"]')];
    dialog.querySelector('.options-tree')!.addEventListener('click',event=>{const item=(event.target as Element).closest<HTMLButtonElement>('[role="treeitem"]');if(item)select(item.dataset.page!,true);});
    dialog.querySelector('.options-tree')!.addEventListener('keydown',event=>{
      const key=(event as KeyboardEvent).key,list=items(),index=list.findIndex(item=>item.dataset.page===active);
      const next=key==='ArrowDown'?index+1:key==='ArrowUp'?index-1:key==='Home'?0:key==='End'?list.length-1:-2;
      if(next===-2)return;event.preventDefault();const target=list[Math.max(0,Math.min(list.length-1,next))];select(target.dataset.page!,true);
    });
    dialog.querySelector('#options-page-picker')!.addEventListener('change',event=>select((event.target as HTMLSelectElement).value));
    const page=dialog.querySelector<HTMLElement>('.options-page')!;
    const update=(event:Event)=>{
      const input=event.target as HTMLInputElement|HTMLSelectElement,path=input.dataset.option;if(!path)return;
      if(input instanceof HTMLInputElement&&input.type==='radio'&&!input.checked)return;
      setPath(draft,path,input instanceof HTMLInputElement&&input.type==='checkbox'?input.checked:input.dataset.number!==undefined?(input.value.trim()===''?NaN:Number(input.value)):input.value);
      dialog.querySelector('.options-page .dialog-error')?.remove();
      if(event.type==='change'&&!(input instanceof HTMLInputElement&&(input.type==='text'||input.type==='number'))){const focused=input.id||undefined,value=input.value;renderPage();(focused?dialog.querySelector<HTMLElement>(`#${CSS.escape(focused)}`):dialog.querySelector<HTMLElement>(`input[name="${CSS.escape((input as HTMLInputElement).name)}"][value="${CSS.escape(value)}"]`))?.focus();}
    };
    page.addEventListener('input',update);page.addEventListener('change',update);
    dialog.querySelector('#options-defaults')!.addEventListener('click',()=>{
      const current=tree.find(({page})=>page.id===active)!.page,defaults:OptionsDraft=withPageDefaults({...structuredClone(response.defaults),appearance:'system'});
      for(const path of pathsOf(current.controls))setPath(draft,path,structuredClone(getPath(defaults,path)));renderPage();
    });
    dialog.querySelector('#options-cancel')!.addEventListener('click',()=>dialog.close());
    dialog.querySelector('#options-ok')!.addEventListener('click',async()=>{
      if(saving)return;
      const problem=validateOptions(draft);if(problem){select(problem.page);error(problem.message);return;}
      const {appearance,...settings}=draft,previous=this.current;saving=true;
      dialog.querySelectorAll<HTMLButtonElement>('.dialog-footer button').forEach(b=>b.disabled=true);
      try{
        const saved=await this.host.save(settings);this.current=saved.settings;this.host.appearance.set(appearance);
        dialog.close();await this.host.applied(saved.settings,previous);
      }catch(e){if(dialog.open)error(String(e));}
      finally{saving=false;dialog.querySelectorAll<HTMLButtonElement>('.dialog-footer button').forEach(b=>b.disabled=false);}
    });
    dialog.addEventListener('cancel',event=>{if(saving)event.preventDefault();});
    dialog.addEventListener('close',()=>{this.showing=false;dialog.remove();});
    renderPage();dialog.showModal();items().find(item=>item.dataset.page===active)?.focus();
    if(getComputedStyle(dialog.querySelector('.options-tree')!).display==='none')dialog.querySelector<HTMLSelectElement>('#options-page-picker')!.focus();
  }
}
