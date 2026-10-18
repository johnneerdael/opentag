export type Startup='empty'|'previous'|'favorite';
/** Recent and favorite folders (directories.json). Read subfolders and the startup choice are Options settings. */
export type DirectoryState={version:number;recent:string[];favorites:string[]};
export type DirectoryPreferences={subfolders:boolean;startup:Startup;associatedFiles?:boolean};
export type LaunchRequest={directories:string[];files:string[];add:boolean};
export type FileOperation='dir-add'|'dir-up'|'dir-back'|'dir-forward'|'dir-favorite'|'dir-manage'|'file-delete'|'file-delete-permanent'|'file-copy'|'file-move'|'file-reveal'|'file-play';
type FileError={path:string;error:string};
type PathChange={from:string;to:string};
export type TransferResult={entries:unknown[];errors:FileError[];renamed:PathChange[];copied:PathChange[]};
type KeyInput={key:string;code?:string;ctrlKey:boolean;metaKey:boolean;shiftKey:boolean;altKey:boolean};

export const defaultDirectoryState=():DirectoryState=>({version:1,recent:[],favorites:[]});
/** Parent folder of a local path, or undefined at a filesystem or drive root. */
export function parentDirectory(path:string):string|undefined {
  const trimmed=path.length>1?path.replace(/[\\/]+$/,''):path,index=Math.max(trimmed.lastIndexOf('/'),trimmed.lastIndexOf('\\'));
  if(index<0||trimmed===''||/^[A-Za-z]:$/.test(trimmed))return undefined;
  const parent=trimmed.slice(0,index);
  return parent===''?(trimmed==='/'?undefined:'/'):/^[A-Za-z]:$/.test(parent)?`${parent}${trimmed[index]}`:parent;
}
/** Working-directory back/forward history; visiting a folder clears the forward list. */
export class DirectoryHistory {
  private back:string[]=[];private forward:string[]=[];current:string|undefined;
  /** `changed` hears every new working directory (for `%_workingdir%`/`%_workingpath%`). */
  constructor(private changed?:(path:string)=>void){}
  visit(path:string){if(path===this.current)return;if(this.current)this.back.push(this.current);this.forward=[];this.current=path;this.changed?.(path);}
  get canBack(){return this.back.length>0;}
  get canForward(){return this.forward.length>0;}
  peek(direction:'back'|'forward'){const list=direction==='back'?this.back:this.forward;return list[list.length-1];}
  move(direction:'back'|'forward'){
    const [from,to]=direction==='back'?[this.back,this.forward]:[this.forward,this.back],next=from.pop();
    if(next!==undefined){if(this.current)to.push(this.current);this.current=next;this.changed?.(next);}return next;
  }
}
/** Map a key press to its File/Edit menu operation. Mp3tag uses Ctrl; macOS uses ⌘ except where the system reserves it (⌘H, ⌘M). */
export function fileShortcut(event:KeyInput,typing:boolean,mac:boolean):'folder'|FileOperation|undefined {
  const key=event.key.toLowerCase(),primary=mac?event.metaKey:event.ctrlKey,other=mac?event.ctrlKey:event.metaKey;
  if(event.altKey&&!primary&&!event.ctrlKey&&!event.metaKey&&!typing&&(key==='arrowleft'||key==='arrowright'))return key==='arrowleft'?'dir-back':'dir-forward';
  if(event.altKey)return;
  if(!primary&&!event.ctrlKey&&!event.metaKey){if(key==='f4'&&!event.shiftKey)return 'file-play';return;}
  if(event.ctrlKey&&!event.metaKey&&!event.shiftKey&&!(mac&&typing)){if(key==='h')return 'dir-add';if(key==='m')return 'dir-favorite';}
  if(!primary||other)return;
  if(key==='d')return event.shiftKey?'dir-up':'folder';
  if(event.shiftKey&&key==='t')return 'file-play';
  if(typing)return;
  if(event.shiftKey&&key==='c')return 'file-copy';
  if(event.shiftKey&&key==='v')return 'file-move';
  if(!event.shiftKey&&(key==='delete'||(mac&&key==='backspace')))return 'file-delete';
}
/** A short, bounded list of names for confirmations. */
export function nameList(items:string[],limit=5){
  const names=items.slice(0,limit).join(', ');
  return items.length>limit?`${names} and ${items.length-limit} more`:names;
}
export function transferStatus(result:TransferResult,mode:'copy'|'move',destination:string){
  const done=mode==='copy'?result.copied.length:result.renamed.length,verb=mode==='copy'?'Copied':'Moved';
  return `${verb} ${done} file${done===1?'':'s'} to ${destination}${result.errors.length?`, ${result.errors.length} error${result.errors.length===1?'':'s'}`:''}`;
}
/** Folder used as the working directory for a launch request. */
/** File → Options → Directories → Startup. Values persist under `pages.directories-startup`. */
export const startupOptionsPage={id:'directories-startup',parent:'directories',label:'Startup',order:10,controls:[
  {kind:'select' as const,path:'pages.directories-startup.startup',label:'Startup directory',choices:[['empty','Empty file list'],['previous','Previous directory'],['favorite','First favorite directory']] as [string,string][]},
  {kind:'note' as const,text:'Manage favorites with File → Favorite directories…. The first favorite opens with Ctrl M.'}]};
export function startupChoice(value:unknown):Startup{return value==='previous'||value==='favorite'?value:'empty';}
export function launchDirectory(request:LaunchRequest){return request.directories[0]??(request.files[0]?parentDirectory(request.files[0]):undefined);}

type Selection={path:string;name:string};
type LoadOptions={replace:boolean;recursive:boolean;location:string;locationIsData?:boolean};
export type FileOpsHost={
  invoke:<T>(command:string,args?:Record<string,unknown>)=>Promise<T>;
  pickDirectories:(title:string,multiple:boolean)=>Promise<string[]>;
  confirm:(message:string|{parts:string[];separator:string},title:string)=>Promise<boolean>;
  selection:()=>Selection[];pendingCount:()=>number;
  /** Scan paths into the file list. Replacing happens only after a successful scan. */
  load:(paths:string[],options:LoadOptions)=>Promise<boolean>;
  removeEntries:(paths:string[])=>void;moved:(result:TransferResult,status:string)=>void;
  run:<T>(message:string,job:()=>Promise<T>)=>Promise<T|undefined>;
  report:(errors:FileError[],status:string)=>void;
  /** Why folders cannot be opened here, or an empty string. */
  openReason:()=>string;changed:()=>void;mac:boolean;
  /** True while another operation runs; launch requests wait for it instead of being dropped. */
  busy?:()=>boolean;
  preferences:()=>DirectoryPreferences;
};
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const plural=(count:number,noun='file')=>`${count} ${noun}${count===1?'':'s'}`;
const basename=(path:string)=>path.replace(/[\\/]+$/,'').split(/[\\/]/).pop()||path;

export class FileOpsUI {
  state=defaultDirectoryState();
  readonly history=new DirectoryHistory(path=>{Promise.resolve().then(()=>this.host.invoke('set_working_directory',{path})).catch(()=>{/* The placeholders stay empty. */});});
  private showing=false;
  private launching:Promise<unknown>=Promise.resolve();
  constructor(private host:FileOpsHost){}
  get fileManager(){return this.host.mac?'Finder':/Win/.test(globalThis.navigator?.platform||'')?'Explorer':'file manager';}
  /** Load preferences, then open queued command-line requests or the startup folder. */
  async initialize(){
    this.state=await this.host.invoke<DirectoryState>('load_directory_state');this.host.changed();
    if(await this.takeLaunches())return;
    const startup=this.host.preferences().startup,start=startup==='previous'?this.state.recent[0]:startup==='favorite'?this.state.favorites[0]:undefined;
    if(start&&!this.host.openReason())await this.openDirectory(start,{replace:true});
  }
  /** Open queued command-line, second-instance and Finder requests in arrival order, one at a time. */
  takeLaunches(){
    const next=this.launching.then(async()=>{
      const requests=await this.host.invoke<LaunchRequest[]>('take_launch_requests');
      for(const request of requests)await this.launch(request);
      return requests.length>0;
    });
    this.launching=next.catch(()=>{/* The caller reports the error; later requests still run. */});
    return next;
  }
  async launch(request:LaunchRequest){
    const paths=[...request.directories,...request.files],directory=launchDirectory(request);
    while(this.host.busy?.())await new Promise(resolve=>setTimeout(resolve,100));
    if(!paths.length||this.host.openReason())return;
    const singleDirectory=request.directories.length===1&&!request.files.length,sharedDirectory=!request.directories.length&&request.files.every(file=>parentDirectory(file)===directory);
    const location=singleDirectory?request.directories[0]:sharedDirectory?directory!:`${paths.length} added locations`;
    await this.open(paths,{replace:!request.add,directories:request.directories,location,locationIsData:singleDirectory||sharedDirectory,working:request.add?undefined:directory});
  }
  reason(op:FileOperation|'folder'):string {
    const selected=this.host.selection().length;
    if(op==='dir-up'&&!this.history.current)return 'Change to a directory first.';
    if(op==='dir-up'&&!parentDirectory(this.history.current!))return 'This is the top folder.';
    if(op==='dir-back'&&!this.history.canBack)return 'No previous directory.';
    if(op==='dir-forward'&&!this.history.canForward)return 'No next directory.';
    if(op==='dir-favorite'&&!this.state.favorites.length)return 'Add a favorite in File → Favorite directories.';
    if(op==='file-reveal'&&!selected&&!this.history.current)return 'Select files or change to a directory first.';
    if(['file-delete','file-delete-permanent','file-copy','file-move','file-play'].includes(op)&&!selected)return 'Select files first.';
    return '';
  }
  /** Recent and favorite folders for the File menu. */
  menuHTML(){
    const disabled=this.host.openReason()?'disabled':'';
    const list=(kind:string,label:string,paths:string[])=>paths.length?`<details class="group-submenu file-dir-menu"><summary>${label}</summary><div>${paths.map((path,index)=>`<button data-dir-${kind}="${index}" title="${html(path)}" ${disabled} translate="no"><span>${html(basename(path))}</span><small>${html(path)}</small></button>`).join('')}</div></details>`:'';
    return `${list('recent','Recent directories',this.state.recent)}${list('favorite','Favorite directories',this.state.favorites)}`;
  }
  bindMenu(root:HTMLElement){
    const close=()=>root.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(d=>d.open=false);
    root.querySelectorAll<HTMLButtonElement>('[data-dir-recent],[data-dir-favorite]').forEach(button=>button.addEventListener('click',()=>{
      close();const path=button.dataset.dirRecent!==undefined?this.state.recent[Number(button.dataset.dirRecent)]:this.state.favorites[Number(button.dataset.dirFavorite)];
      if(path)void this.openDirectory(path,{replace:true}).catch(error=>this.host.report([{path,error:String(error)}],'Directory could not be opened'));
    }));
  }
  async operate(op:FileOperation|'folder'){
    if(op==='dir-manage'){await this.manage();return;}
    if(this.reason(op))return;
    if(op==='folder'||op==='dir-add'){
      const replace=op==='folder';if(replace&&!await this.confirmDiscard())return;
      const picked=await this.host.pickDirectories(replace?'Change directory':'Add directory',!replace);
      if(picked.length)await this.open(picked,{replace,directories:picked,location:picked.length===1?picked[0]:`${picked.length} added locations`,locationIsData:picked.length===1,working:replace?picked[0]:undefined,confirmed:true});
      return;
    }
    if(op==='dir-up'){await this.openDirectory(parentDirectory(this.history.current!)!,{replace:true});return;}
    if(op==='dir-back'||op==='dir-forward'){
      const direction=op==='dir-back'?'back':'forward',target=this.history.peek(direction);
      if(target&&await this.confirmDiscard()&&await this.open([target],{replace:true,directories:[target],location:target,confirmed:true}))this.history.move(direction);
      return;
    }
    if(op==='dir-favorite'){await this.openDirectory(this.state.favorites[0],{replace:true});return;}
    const selection=this.host.selection(),paths=selection.map(item=>item.path),names=nameList(selection.map(item=>item.name));
    if(op==='file-reveal'){await this.host.run(`Opening ${this.fileManager}…`,async()=>{if(paths.length)await this.host.invoke('reveal_files',{paths});else await this.host.invoke('open_directory',{path:this.history.current});this.host.report([],paths.length?`Showed ${plural(paths.length)} in ${this.fileManager}`:`Opened ${this.history.current}`);});return;}
    if(op==='file-play'){
      if(paths.length>10&&!await this.host.confirm(`Open ${plural(paths.length)} with the default player?`,'Play'))return;
      await this.host.run('Opening the default player…',async()=>{await this.host.invoke('play_files',{paths});this.host.report([],`Playing ${plural(paths.length)}`);});return;
    }
    if(op==='file-delete'){
      if(!await this.host.confirm(`Move ${plural(paths.length)} to the ${this.host.mac?'Trash':'Recycle Bin or trash'}?\n\n${names}\n\nThe files are removed from the list. Restore them from the ${this.host.mac?'Trash':'Recycle Bin or trash'} if needed.`,'Delete files'))return;
      await this.host.run(`Deleting ${plural(paths.length)}…`,async()=>{
        const result=await this.host.invoke<{removed:string[];errors:FileError[]}>('trash_files',{paths});
        this.host.removeEntries(result.removed);
        this.host.report(result.errors,`Moved ${plural(result.removed.length)} to the ${this.host.mac?'Trash':'trash'}${result.errors.length?`, ${plural(result.errors.length,'error')}`:''}`);
      });
      return;
    }
    if(op==='file-delete-permanent'){
      // Confirmed like Delete files (Options → Messages); the text says there is no way back.
      if(!await this.host.confirm(`Permanently delete ${plural(paths.length)}?\n\n${names}\n\nThe files are removed from the disk without using the ${this.host.mac?'Trash':'Recycle Bin or trash'} and cannot be restored.`,'Delete permanently'))return;
      await this.host.run(`Deleting ${plural(paths.length)}…`,async()=>{
        const result=await this.host.invoke<{removed:string[];errors:FileError[]}>('delete_files_permanently',{paths});
        this.host.removeEntries(result.removed);
        this.host.report(result.errors,`Deleted ${plural(result.removed.length)} permanently${result.errors.length?`, ${plural(result.errors.length,'error')}`:''}`);
      });
      return;
    }
    if(op==='file-copy'||op==='file-move'){
      const mode=op==='file-copy'?'copy':'move',[destination]=await this.host.pickDirectories(mode==='copy'?'Copy files to':'Move files to',false);
      if(!destination)return;
      const reviewed=mode==='move'&&this.host.preferences?.().associatedFiles
        ?await this.host.run('Checking associated files…',()=>this.host.invoke<{plan_token:string;associated:PathChange[]}>('preview_transfer',{paths,destination})):undefined;
      if(mode==='move'&&this.host.preferences?.().associatedFiles&&!reviewed)return;
      const confirmation=`${mode==='copy'?'Copy':'Move'} ${plural(paths.length)} to ${destination}?\n\n${names}\n\nExisting files are never replaced. If any name is already taken, nothing is ${mode==='copy'?'copied':'moved'}.`;
      const reviewMessage=reviewed?{parts:[confirmation,reviewed.associated.length?`Associated .lrc/.cdg files: ${reviewed.associated.length}`:'Associated .lrc/.cdg files: none found.',...reviewed.associated.map(item=>`Associated file: ${item.from} → ${item.to}`)],separator:'\n\n'}:confirmation;
      if(!await this.host.confirm(reviewMessage,mode==='copy'?'Copy files':'Move files'))return;
      await this.host.run(`${mode==='copy'?'Copying':'Moving'} ${plural(paths.length)}…`,async()=>{
        const result=await this.host.invoke<TransferResult>('transfer_files',{paths,destination,mode,...(reviewed?{planToken:reviewed.plan_token}:{})}),status=transferStatus(result,mode,destination);
        if(mode==='move')this.host.moved(result,status);else this.host.report(result.errors,status);
      });
    }
  }
  /** A plain drop on the window replaces the File List with the dropped files, folders and playlists. */
  async replaceWith(paths:string[]){
    if(!paths.length||this.host.openReason())return false;
    return this.open(paths,{replace:true,directories:[],location:paths.length===1?paths[0]:`${paths.length} dropped locations`,locationIsData:paths.length===1});
  }
  /** Open one folder as the working directory. */
  async openDirectory(path:string,options:{replace:boolean}){
    return this.open([path],{...options,directories:[path],location:path,working:options.replace?path:undefined});
  }
  private async confirmDiscard(){
    const pending=this.host.pendingCount();
    return !pending||this.host.confirm(`Change directory and discard pending tag changes in ${plural(pending)}?`,'Unsaved changes');
  }
  private async open(paths:string[],options:{replace:boolean;directories:string[];location:string;locationIsData?:boolean;working?:string;confirmed?:boolean}){
    if(options.replace&&!options.confirmed&&!await this.confirmDiscard())return false;
    const loaded=await this.host.load(paths,{replace:options.replace,recursive:this.host.preferences().subfolders,location:options.location,...(options.locationIsData===false?{locationIsData:false}:{})});
    if(!loaded)return false;
    if(options.working)this.history.visit(options.working);
    if(options.directories.length)try{this.state=await this.host.invoke<DirectoryState>('remember_directories',{paths:options.directories});}catch{/* Recent folders are a convenience; loading already succeeded. */}
    this.host.changed();return true;
  }
  private async save(next:DirectoryState){
    this.state=await this.host.invoke<DirectoryState>('save_directory_state',{state:next});this.host.changed();
  }
  /** Favorite directories: add, order, remove and open favorites, and clear the recent list. */
  async manage(){
    if(this.showing)return;this.showing=true;
    let draft=structuredClone(this.state),current=0,working=false;
    const dialog=document.createElement('dialog');dialog.className='directories-dialog';dialog.setAttribute('aria-label','Favorite directories');document.body.append(dialog);
    const error=(message:unknown)=>{const p=dialog.querySelector<HTMLElement>('.dialog-error')!;p.hidden=false;p.textContent=String(message);};
    const render=()=>{
      current=Math.min(current,Math.max(0,draft.favorites.length-1));
      dialog.innerHTML=`<header>Favorite directories</header><div class="dialog-content">
      <label class="field"><span>Favorite directories</span><select id="dir-favorites" size="5" aria-label="Favorite directories">${draft.favorites.map((path,index)=>`<option value="${index}" translate="no" ${index===current?'selected':''}>${index===0?'★ ':''}${html(path)}</option>`).join('')}</select></label>
      <p>${draft.favorites.length?'The first favorite opens with File → Favorite directory (Ctrl M) and at startup.':'No favorites yet. Add the current directory or choose a folder.'}</p>
      <div class="group-buttons"><button id="dir-add-current" ${this.history.current&&!draft.favorites.includes(this.history.current)?'':'disabled'}>Add current directory</button><button id="dir-add-folder">Add folder…</button><button id="dir-first" ${current>0?'':'disabled'}>Make first</button><button id="dir-remove" ${draft.favorites.length?'':'disabled'}>Remove</button><button id="dir-open" ${draft.favorites.length&&!this.host.openReason()?'':'disabled'}>Open</button></div>
      <p>Read subfolders and the startup directory are in File → Options → Directories.</p>
      <div class="group-buttons"><button id="dir-clear-recent" ${draft.recent.length?'':'disabled'}>Clear recent directories (${draft.recent.length})</button></div>
      <p class="dialog-error" role="alert" hidden></p></div>
      <div class="dialog-footer"><button id="dir-cancel">Cancel</button><button class="primary" id="dir-save">Save</button></div>`;
      const on=(id:string,handler:()=>void|Promise<void>)=>dialog.querySelector(`#${id}`)!.addEventListener('click',()=>{if(!working)void handler();});
      dialog.querySelector('#dir-favorites')!.addEventListener('change',event=>{current=Number((event.target as HTMLSelectElement).value);render();});
      on('dir-add-current',()=>{draft.favorites.push(this.history.current!);current=draft.favorites.length-1;render();});
      on('dir-add-folder',async()=>{working=true;try{for(const path of await this.host.pickDirectories('Add favorite directory',true))if(!draft.favorites.includes(path))draft.favorites.push(path);current=draft.favorites.length-1;}catch(e){error(e);}finally{working=false;if(dialog.open)render();}});
      on('dir-first',()=>{draft.favorites.unshift(...draft.favorites.splice(current,1));current=0;render();});
      on('dir-remove',()=>{draft.favorites.splice(current,1);render();});
      on('dir-clear-recent',()=>{draft.recent=[];render();});
      on('dir-cancel',()=>dialog.close());
      const persist=async()=>{working=true;try{await this.save(draft);draft=structuredClone(this.state);return true;}catch(e){error(e);return false;}finally{working=false;}};
      on('dir-save',async()=>{if(await persist())dialog.close();});
      on('dir-open',async()=>{const path=draft.favorites[current];if(path&&await persist()){dialog.close();await this.openDirectory(path,{replace:true});}});
    };
    dialog.addEventListener('close',()=>{this.showing=false;dialog.remove();});
    render();dialog.showModal();
  }
}
