import { menuSegments, mnemonicMarkup } from './action-ui';

/** One `[SearchBy]` field: its label and the format string that fills it from the first selected file. */
export type SearchField = {label:string;format:string;default_label?:boolean};
/** A `.src` file in the sources folder. `menu` is its file name without extension: `#` makes submenus, `&` marks access keys. */
export type WebSourceInfo = {id:string;menu:string;name:string;search_by:SearchField[];direct:boolean};
export type WebSettingsInfo = {id:string;menu:string};
export type WebListing = {folder:string;sources:WebSourceInfo[];settings:WebSettingsInfo[];problems:{file:string;error:string}[]};
export type WebResult = {url:string;cover:string|null;preview:string|null;values:string[]};
export type WebSearchPage = {columns:string[];results:WebResult[];direct:boolean};
/** A `.settings` schema as documented for Mp3tag tag sources. */
export type SettingSpec = {type:string;key:string;title?:string;description?:string;choices?:string[];default?:unknown};
export type SettingsSchema = {key:string;title?:string;settings:SettingSpec[];localizations?:Record<string,{key:string;title?:string;description?:string}[]>};
export type SettingsForm = {schema:SettingsSchema;values:Record<string,unknown>};

const html = (value:unknown) => String(value ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
let settingsTitleId = 0;
/** The menu path without access-key markers, for dialog titles. */
export const menuLabel = (menu:string) => menuSegments(menu).map(segment => segment.label).join(' › ');

type Leaf = {menu:string;markup:(raw:string)=>string};
/** Nested submenus for `#` segments, in the order given; leaves render with `markup`. */
export function menuTree(leaves:Leaf[], prefix:string[] = []):string {
  const slots:({leaf:Leaf;raw:string}|{branch:string;raw:string})[] = [], branches = new Set<string>();
  for (const leaf of leaves) {
    const segments = menuSegments(leaf.menu), raw = leaf.menu.split('#')[prefix.length];
    if (!prefix.every((part,i) => segments[i]?.label === part)) continue;
    const segment = segments[prefix.length];
    if (!segment) continue;
    if (segments.length === prefix.length+1) slots.push({leaf,raw});
    else if (!branches.has(segment.label)) { branches.add(segment.label); slots.push({branch:segment.label,raw}); }
  }
  return slots.map(slot => 'branch' in slot
    ? `<details class="group-submenu web-submenu"><summary translate="no">${mnemonicMarkup(slot.raw)}</summary><div>${menuTree(leaves,[...prefix,slot.branch])}</div></details>`
    : slot.leaf.markup(slot.raw)).join('');
}

/** Localized title and description of a schema entry (ISO 639-1 language). */
export function localized(schema:SettingsSchema, key:string, language:string) {
  const entries = schema.localizations?.[language.slice(0,2).toLowerCase()] || [];
  return entries.find(entry => entry.key === key);
}

/** Read the dialog's controls back into typed values. */
export function readSettings(schema:SettingsSchema, root:ParentNode, language='en'):Record<string,unknown> {
  const values:Record<string,unknown> = {};
  schema.settings.forEach((setting,index) => {
    const control = root.querySelector<HTMLInputElement|HTMLSelectElement>(`[data-setting="${index}"]`);
    if (!control) return;
    if (setting.type === 'bool') values[setting.key] = (control as HTMLInputElement).checked;
    else if (setting.type === 'number') {
      const number = Number(control.value);
      if (control.value.trim() === '' || !Number.isFinite(number)) {const title=localized(schema,setting.key,language)?.title || setting.title || setting.key;throw new Error(`${title} needs a number.`);}
      values[setting.key] = number;
    } else values[setting.key] = control.value;
  });
  return values;
}

export function settingsMarkup(form:SettingsForm, language:string) {
  const {schema,values} = form;
  return schema.settings.map((setting,index) => {
    const text = localized(schema,setting.key,language), title = text?.title || setting.title || setting.key, description = text?.description || setting.description || '';
    const help = description ? `<small translate="no">${html(description)}</small>` : '';
    const value = values[setting.key] ?? setting.default;
    switch (setting.type) {
      case 'heading': return `<h3 class="ws-heading" translate="no">${html(title)}</h3>`;
      case 'separator': return `<hr class="ws-separator">${setting.title || text?.title ? `<p class="ws-note" translate="no">${html(title)}</p>` : ''}`;
      case 'bool': return `<label class="action-check"><input type="checkbox" data-setting="${index}" ${value === true ? 'checked' : ''}> <span translate="no">${html(title)}</span></label>${help}`;
      case 'number': return `<label class="field"><span translate="no">${html(title)}</span><input type="number" data-setting="${index}" value="${html(value)}"></label>${help}`;
      case 'string': return setting.choices?.length
        ? `<label class="field"><span translate="no">${html(title)}</span><select data-setting="${index}">${setting.choices.map(choice => `<option value="${html(choice)}" translate="no" ${choice === value ? 'selected' : ''}>${html(choice)}</option>`).join('')}</select></label>${help}`
        : `<label class="field"><span translate="no">${html(title)}</span><input data-setting="${index}" value="${html(value)}" autocomplete="off" spellcheck="false"></label>${help}`;
      default: return '';
    }
  }).join('');
}

type Invoke = <T>(command:string, args?:Record<string,unknown>) => Promise<T>;
type WebSourcesHost = {
  invoke:Invoke;
  /** Why tag sources cannot run now, or an empty string. */
  reason:()=>string;
  open:(source:WebSourceInfo)=>Promise<void>;
  /** The id of the last used web source (Ctrl+I), if any. */
  last:()=>string|null;
  shortcut:string;
  changed:()=>void;
  pickFiles:(mode?:'files'|'folder')=>Promise<string[]|null>;
  report:(message:string)=>void;
  fail:(error:unknown)=>void;
};

export class WebSourcesUI {
  listing:WebListing = {folder:'',sources:[],settings:[],problems:[]};
  private mnemonicPrefix:string[] = [];
  private showing = false;
  constructor(private host:WebSourcesHost) {}

  /** Rescan the sources folder; the menu is rebuilt only when something changed. */
  async refresh() {
    try {
      const next = await this.host.invoke<WebListing>('web_sources_list');
      if (JSON.stringify(next) !== JSON.stringify(this.listing)) { this.listing = next; this.host.changed(); }
    } catch (error) { this.host.fail(error); }
  }
  source(id:string) { return this.listing.sources.find(source => source.id === id); }

  /** Web sources and their configuration entries for the Tag Sources menu. */
  menuHTML() {
    const reason = this.host.reason(), last = this.host.last();
    const sources = menuTree(this.listing.sources.map((source,index) => ({menu:source.menu,markup:raw => `<button data-web-source="${index}" ${reason ? 'disabled' : `data-verbatim="${html(source.name)}"`} title="${html(reason || source.name)}"><span translate="no">${mnemonicMarkup(raw)}…</span>${source.id === last ? `<small>${this.host.shortcut}</small>` : ''}</button>`})));
    const settings = menuTree(this.listing.settings.map((entry,index) => ({menu:entry.menu,markup:raw => `<button data-web-settings="${index}"><span translate="no">${mnemonicMarkup(raw)}…</span></button>`})));
    const problems = this.listing.problems.length ? `<p class="menu-note" title="${html(this.listing.problems.map(p => `${p.file}: ${p.error}`).join('\n'))}">${this.listing.problems.length} tag source file${this.listing.problems.length === 1 ? '' : 's'} could not be read.</p>` : '';
    return `${sources ? `<hr>${sources}` : ''}${problems}${settings ? `<hr>${settings}` : ''}<hr>`;
  }
  bindMenu(root:HTMLElement) {
    // Submenu hover and placement come from ActionUI.bindMenu (.group-submenu).
    const close = () => root.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(d => d.open = false);
    root.querySelectorAll<HTMLButtonElement>('[data-web-source]').forEach(button => button.addEventListener('click',() => { close(); void this.run(this.listing.sources[Number(button.dataset.webSource)]); }));
    root.querySelectorAll<HTMLButtonElement>('[data-web-settings]').forEach(button => button.addEventListener('click',() => { close(); void this.settings(this.listing.settings[Number(button.dataset.webSettings)]); }));
  }
  async run(source:WebSourceInfo|undefined) {
    if (!source) return;
    try { await this.host.open(source); } catch (error) { this.host.fail(error); }
  }

  /** Typing in the open Tag Sources menu follows `&` access keys through submenus. */
  beginMnemonic() { this.mnemonicPrefix = []; }
  mnemonic(key:string) {
    const all = [...this.listing.sources.map(source => ({source,menu:source.menu})),...this.listing.settings.map(entry => ({entry,menu:entry.menu}))];
    const candidates = all.map(item => ({...item,segments:menuSegments(item.menu)})).filter(item => this.mnemonicPrefix.every((part,i) => item.segments[i]?.label === part));
    const match = candidates.find(item => item.segments[this.mnemonicPrefix.length]?.mnemonic === key.toLocaleLowerCase());
    if (!match) return false;
    if (match.segments.length === this.mnemonicPrefix.length+1) {
      this.mnemonicPrefix = [];
      document.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(d => d.open = false);
      if ('source' in match) void this.run(match.source); else if ('entry' in match) void this.settings(match.entry);
    } else {
      this.mnemonicPrefix.push(match.segments[this.mnemonicPrefix.length].label);
      const label = this.mnemonicPrefix[this.mnemonicPrefix.length-1];
      const submenu = [...document.querySelectorAll<HTMLDetailsElement>('#tag-sources-menu .web-submenu')].find(d => d.querySelector('summary')?.textContent === label);
      if (submenu) { submenu.open = true; submenu.querySelector<HTMLElement>(':scope > summary')?.focus(); }
    }
    return true;
  }

  /** Tag Sources → Install tag sources…: copy chosen files into the sources folder. */
  async install() {
    if (this.showing) return;
    this.showing = true;
    const mode = await new Promise<'files'|'folder'|null>(resolve => {
      const dialog = document.createElement('dialog');
      dialog.className = 'tag-source-dialog';
      dialog.setAttribute('aria-label','Install tag sources');
      dialog.innerHTML = '<header>Install tag sources</header><div class="dialog-content"><p>Choose a complete downloaded folder or ZIP, or select all source files and their companions together. The installation is checked before replacing existing files.</p><p>Individual-file updates can reuse installed companion files.</p></div><div class="dialog-footer"><button data-cancel>Cancel</button><button data-folder>Folder…</button><button data-files>Files or ZIP…</button></div>';
      let chosen:'files'|'folder'|null = null;
      dialog.querySelector('[data-folder]')!.addEventListener('click',()=>{chosen='folder';dialog.close();});
      dialog.querySelector('[data-files]')!.addEventListener('click',()=>{chosen='files';dialog.close();});
      dialog.querySelector('[data-cancel]')!.addEventListener('click',()=>dialog.close());
      dialog.addEventListener('close',()=>{dialog.remove();resolve(chosen);},{once:true});
      document.body.append(dialog);dialog.showModal();
    });
    if (!mode) { this.showing = false; return; }
    try {
      const paths = await this.host.pickFiles(mode);
      if (!paths?.length) return;
      const listing = await this.host.invoke<WebListing>('web_sources_install',{paths});
      this.listing = listing; this.host.changed();
      const names = paths.map(path => path.split(/[\\/]/).pop()).join(', ');
      this.host.report(listing.problems.length ? `Installed ${names}. ${listing.problems.length} tag source file${listing.problems.length === 1 ? '' : 's'} could not be read: ${listing.problems.map(p => `${p.file}: ${p.error}`).join('; ')}` : `Installed ${names}`);
    } catch (error) { this.host.fail(error); }
    finally { this.showing = false; }
  }
  async openFolder() { try { await this.host.invoke('web_sources_open_folder'); } catch (error) { this.host.fail(error); } }

  /** The configuration dialog generated from a `.settings` schema. */
  async settings(entry:WebSettingsInfo|undefined) {
    if (!entry || this.showing) return;
    let form:SettingsForm;
    try { form = await this.host.invoke<SettingsForm>('web_source_settings',{id:entry.id}); } catch (error) { this.host.fail(error); return; }
    this.showing = true;
    // The UI language (Options → Language) picks the source's own translations.
    const language = document.documentElement.lang || navigator.language || 'en', title = localized(form.schema,form.schema.key,language)?.title || form.schema.title || menuLabel(entry.menu);
    const dialog = document.createElement('dialog');
    dialog.className = 'tag-source-dialog web-settings-dialog';
    const titleId=`ws-settings-title-${++settingsTitleId}`;
    dialog.setAttribute('aria-labelledby',titleId);
    dialog.innerHTML = `<header id="${titleId}" translate="no">${html(title)}</header><div class="dialog-content">${settingsMarkup(form,language) || '<p>This tag source has no settings.</p>'}</div><div class="dialog-footer"><button data-close>Cancel</button><button class="primary" id="ws-save">Save</button></div>`;
    document.body.append(dialog);
    const error = (message:unknown) => { let p = dialog.querySelector('.dialog-error'); if (!p) { p = document.createElement('p'); p.className = 'dialog-error'; p.setAttribute('role','alert'); dialog.querySelector('.dialog-content')!.append(p); } p.textContent = String(message).replace(/^Error: /,''); };
    dialog.querySelector('[data-close]')!.addEventListener('click',() => dialog.close());
    dialog.addEventListener('close',() => { this.showing = false; dialog.remove(); });
    dialog.querySelector('#ws-save')!.addEventListener('click',async () => {
      try { await this.host.invoke('save_web_source_settings',{id:entry.id,values:readSettings(form.schema,dialog,language)}); dialog.close(); }
      catch (e) { error(e); }
    });
    dialog.showModal();
    dialog.querySelector<HTMLElement>('[data-setting]')?.focus();
  }
}
