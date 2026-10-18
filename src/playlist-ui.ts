// Playlists (https://docs.mp3tag.de/playlists/): File → Load Playlist/Cuesheet,
// File → Playlist (all files / selected files), File → Generate Playlists and
// Options → Playlist. Options → Export (https://docs.mp3tag.de/customization/options/export/)
// lives here too because both pages configure written output files.
import type { OptionsPage } from './options-ui';

type FileError={path:string;error:string};
export type PlannedPlaylist={path:string;files:number};
export type PlaylistPlan={playlists:PlannedPlaylist[];errors:FileError[]};
export type PlaylistContents={files:string[];errors:FileError[]};
export type ExpandedPaths={paths:string[];errors:FileError[];playlist?:string};
export type PlaylistScope='all'|'selected';

export const PLAYLIST_EXTENSIONS=['m3u','m3u8','pls','cue','xspf','wpl','asx'];
/** Whether a dropped, opened or command-line path is a playlist or cue sheet. */
export function isPlaylistPath(path:string){const match=/\.([^.\\/]+)$/.exec(path);return !!match&&PLAYLIST_EXTENSIONS.includes(match[1].toLowerCase());}

export const PLAYLIST_DEFAULTS={extended:true,extinf_format:'%artist% - %title%',relative:true,auto_create:false,filename:'%_directory%.m3u8',confirm_filename:true,write_bom:false,dont_create_directories:false,
  generator_filename:'%_folderpath%%_directory%.m3u8',generator_partition:'%_folderpath%'};
const check=(key:string,label:string)=>({kind:'checkbox' as const,path:`pages.playlist.${key}`,label});
export const playlistOptionsPage:OptionsPage={id:'playlist',label:'Playlist',order:64,defaults:{...PLAYLIST_DEFAULTS},
  description:'How File → Playlist (all files), Playlist (selected files) and Generate Playlists write playlists.',controls:[
    check('extended','Write extended info'),
    {kind:'text',path:'pages.playlist.extinf_format',label:'Extended info format string',placeholder:PLAYLIST_DEFAULTS.extinf_format},
    check('relative','Entries relative to working directory (if possible)'),
    check('auto_create','Create playlist automatically after saving tags or renaming files'),
    {kind:'text',path:'pages.playlist.filename',label:'Filename of playlist',placeholder:PLAYLIST_DEFAULTS.filename},
    check('confirm_filename','Show confirm filename dialog'),
    check('write_bom','Write BOM for UTF-8 playlists (.m3u8, .pls)'),
    check('dont_create_directories','Don\'t create directories'),
    {kind:'note',text:'Relative file names are created in the working directory. The extension chooses the format: .m3u8 is UTF-8, .m3u uses the Windows ANSI codepage (UTF-8 on other systems) and .pls writes a PLS playlist. %_playlist_filename%, %_playlist_filename_ext% and %_playlist_folderpath% describe the loaded playlist.'},
  ],
  validate:draft=>{const value=draft.pages.playlist?.filename;return typeof value==='string'&&!value.trim()?'Enter a filename of playlist.':undefined;}};

export const EXPORT_DEFAULTS={show_dialog:true,write_bom:false,dont_create_directories:false};
export const exportOptionsPage:OptionsPage={id:'export',label:'Export',order:62,defaults:{...EXPORT_DEFAULTS},
  description:'Options of File → Export. Export configurations are edited in the Export dialog.',controls:[
    {kind:'checkbox',path:'pages.export.show_dialog',label:'Show selection dialog'},
    {kind:'checkbox',path:'pages.export.dont_create_directories',label:'Don\'t create directories'},
    {kind:'checkbox',path:'pages.export.write_bom',label:'Write BOM (UTF-8 and UTF-16 exports)'},
    {kind:'note',text:'Without the selection dialog, File → Export runs the active configuration at once. Hold Shift (⌘/Ctrl ⇧ E) to create one export file per directory. Options → Messages decides whether TagGroove offers to open the exported file.'},
  ]};

const str=(value:unknown,fallback:string)=>typeof value==='string'?value:fallback;
const bool=(value:unknown,fallback:boolean)=>typeof value==='boolean'?value:fallback;
export function playlistValues(values:Record<string,unknown>){
  return{filename:str(values.filename,PLAYLIST_DEFAULTS.filename),confirm:bool(values.confirm_filename,PLAYLIST_DEFAULTS.confirm_filename),auto:bool(values.auto_create,PLAYLIST_DEFAULTS.auto_create),
    generatorFilename:str(values.generator_filename,PLAYLIST_DEFAULTS.generator_filename),generatorPartition:str(values.generator_partition,PLAYLIST_DEFAULTS.generator_partition)};
}

export type PlaylistRequest={paths:string[];filename_template:string;partition_template?:string|null;base_directory?:string|null;output_path?:string|null;playlist?:string|null;columns?:unknown};
export type PlaylistHost={
  invoke:<T>(command:string,args?:Record<string,unknown>)=>Promise<T>;
  /** Options → Playlist values (`pages.playlist`). */
  values:()=>Record<string,unknown>;
  remember:(values:Record<string,unknown>)=>Promise<void>;
  /** File List paths in list order. */
  files:(scope:PlaylistScope)=>string[];
  /** Working directory for relative playlist names. */
  workingDirectory:()=>string|undefined;
  loadedPlaylist:()=>string|undefined;
  columns:(paths:string[])=>unknown;
  modal:(title:string,content:string,buttons:string)=>HTMLDialogElement;
  modalError:(dialog:HTMLDialogElement,error:unknown)=>void;
  run:<T>(message:string,job:()=>Promise<T>)=>Promise<T|undefined>;
  /** Report the result; `interactive` offers to open a single written file. */
  done:(plan:PlaylistPlan,interactive:boolean)=>Promise<void>|void;
};
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const plural=(count:number,noun='file')=>`${count} ${noun}${count===1?'':'s'}`;

export class PlaylistUI {
  constructor(private host:PlaylistHost){}
  /** Replace playlists and cue sheets among `paths` by their entries, in order. */
  async expand(paths:string[]):Promise<ExpandedPaths>{
    const result:ExpandedPaths={paths:[],errors:[]};
    for(const path of paths){
      if(!isPlaylistPath(path)){result.paths.push(path);continue;}
      const contents=await this.host.invoke<PlaylistContents>('read_playlist',{path});
      result.paths.push(...contents.files);result.errors.push(...contents.errors);result.playlist=path;
    }
    return result;
  }
  private request(paths:string[],filename:string,partition?:string):PlaylistRequest{
    return{paths,filename_template:filename,partition_template:partition??null,base_directory:this.host.workingDirectory()??null,playlist:this.host.loadedPlaylist()??null,columns:this.host.columns(paths)};
  }
  /** File → Playlist (all files / selected files). `automatic` creates it without dialogs. */
  async create(scope:PlaylistScope,automatic=false){
    const paths=this.host.files(scope);if(!paths.length)return;
    const options=playlistValues(this.host.values()),request=this.request(paths,options.filename);
    if(automatic||!options.confirm){await this.write(request,!automatic);return;}
    const plan=await this.host.invoke<PlaylistPlan>('plan_playlists',{request});
    const proposed=plan.playlists[0]?.path??'';
    const dialog=this.host.modal('Playlist',`<p>Create a playlist of ${plural(paths.length)}. Placeholders are not evaluated in this name.</p>
      <label class="field"><span>Playlist file name</span><input id="playlist-filename" value="${html(proposed)}" spellcheck="false" autocomplete="off"></label>`,'<button id="playlist-ok" class="primary">OK</button>');
    dialog.classList.add('playlist-dialog');
    const input=dialog.querySelector<HTMLInputElement>('#playlist-filename')!,button=dialog.querySelector<HTMLButtonElement>('#playlist-ok')!;
    const submit=async()=>{
      if(!input.value.trim()){this.host.modalError(dialog,'Enter a playlist file name.');return;}
      button.disabled=true;
      try{const written=await this.host.invoke<PlaylistPlan>('create_playlists',{request:{...request,output_path:input.value}});dialog.close();await this.host.done(written,true);}
      catch(error){this.host.modalError(dialog,error);}
      finally{button.disabled=false;}
    };
    button.addEventListener('click',()=>void submit());
    input.addEventListener('keydown',event=>{if(event.key==='Enter'){event.preventDefault();void submit();}});
    input.focus();input.select();
  }
  /**
   * File → Playlist per folder (Ctrl+Shift+P): one playlist of all listed files for each
   * folder, written into that folder with the Options → Playlist file name.
   */
  async createPerFolder(){
    const paths=this.host.files('all');if(!paths.length)return;
    const options=playlistValues(this.host.values());
    await this.write({...this.request(paths,options.filename,'%_folderpath%'),base_directory:null},true);
  }
  private async write(request:PlaylistRequest,interactive:boolean){
    const plan=await this.host.run('Creating playlist…',()=>this.host.invoke<PlaylistPlan>('create_playlists',{request}));
    if(plan)await this.host.done(plan,interactive);
  }
  /** File → Generate Playlists: one playlist per partition value, with a preview. */
  generate(){
    const selected=this.host.files('selected'),paths=selected.length?selected:this.host.files('all');if(!paths.length)return;
    const options=playlistValues(this.host.values());
    const dialog=this.host.modal('Generate Playlists',`<p>${plural(paths.length)}${selected.length?' selected':''}. Files with the same partition value form one playlist, in File List order.</p>
      <label class="field"><span>Playlist file name</span><input id="generate-filename" value="${html(options.generatorFilename)}" data-helper="format" spellcheck="false" autocomplete="off"></label>
      <label class="field"><span>Partition format</span><input id="generate-partition" value="${html(options.generatorPartition)}" data-helper="format" spellcheck="false" autocomplete="off"></label>
      <div class="preview" id="generate-preview" aria-live="polite" aria-label="Preview"></div>`,'<button id="generate-ok" class="primary">OK</button>');
    dialog.classList.add('generate-playlists-dialog');
    const filename=dialog.querySelector<HTMLInputElement>('#generate-filename')!,partition=dialog.querySelector<HTMLInputElement>('#generate-partition')!;
    const preview=dialog.querySelector<HTMLElement>('#generate-preview')!,button=dialog.querySelector<HTMLButtonElement>('#generate-ok')!;
    const request=()=>this.request(paths,filename.value,partition.value);
    let generation=0,timer=0;
    const update=()=>{clearTimeout(timer);timer=window.setTimeout(async()=>{
      const current=++generation;
      try{
        const plan=await this.host.invoke<PlaylistPlan>('plan_playlists',{request:request()});if(current!==generation)return;
        preview.innerHTML=`<div><strong>${plural(plan.playlists.length,'playlist')}</strong></div>`+plan.playlists.slice(0,50).map(p=>`<div><span translate="no">${html(p.path)}</span> <small>(${plural(p.files)})</small></div>`).join('')+(plan.playlists.length>50?`<div><small>… ${plan.playlists.length-50} more</small></div>`:'');
        dialog.querySelector('.dialog-error')?.remove();
      }catch(error){if(current===generation){preview.textContent='';this.host.modalError(dialog,error);}}
    },200);};
    filename.addEventListener('input',update);partition.addEventListener('input',update);
    button.addEventListener('click',async()=>{
      button.disabled=true;
      try{
        const plan=await this.host.invoke<PlaylistPlan>('create_playlists',{request:request()});
        await this.host.remember({generator_filename:filename.value,generator_partition:partition.value}).catch(()=>{/* The playlists are written; only the remembered pattern is lost. */});
        dialog.close();await this.host.done(plan,true);
      }catch(error){this.host.modalError(dialog,error);}
      finally{button.disabled=false;}
    });
    update();filename.focus();
  }
}
