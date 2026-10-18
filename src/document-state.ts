import type { PendingCover } from './tag-clipboard';

type Fields = Record<string, string[]>;
export type DocumentState = {
  uri: string; name: string; writable: boolean; persisted: boolean;
  state: 'ready'|'recovery_required'; error: string|null;
};
export type DocumentAccess = { states: Record<string, DocumentState>; refreshError: string|null };

/** Keep publication policy separate from draft editing policy. Unknown access fails closed. */
export function documentSelection(paths: string[], access: DocumentAccess, documentPlatform: boolean) {
  const managed = paths.filter(path => !!access.states[path]);
  const provider = documentPlatform || managed.length > 0;
  let mutationReason = '', reloadReason = '';
  if(provider && paths.length) {
    if(access.refreshError || managed.length !== paths.length) mutationReason = 'Source access is unknown. Refresh source access before saving.';
    else if(managed.some(path => access.states[path].state !== 'ready')) mutationReason = 'Recovery required. Save an edited copy or the original backup.';
    else if(managed.some(path => !access.states[path].writable)) mutationReason = 'Selection includes a read-only source. Keep editing and use Save a copy.';
    if(access.refreshError || managed.length !== paths.length) reloadReason = 'Source access is unknown. Refresh source access before reloading.';
    else if(managed.some(path => access.states[path].state !== 'ready')) reloadReason = 'Recovery copies are retained. Use Save a copy before reopening the source.';
  }
  return {
    mutationReason, reloadReason,
    unsupportedReason: provider && paths.length ? 'Actions, renames, text imports and exports are not available for provider documents yet.' : '',
    copyReason: paths.length !== 1 ? 'Select one provider document to save a copy.' : !access.states[paths[0]] ? 'Save a copy requires a known provider document.' : '',
  };
}

/** Refreshing access must never replace retained source errors when the refresh itself fails. */
export async function refreshDocumentAccess(previous: DocumentAccess, load: () => Promise<Record<string, DocumentState>>): Promise<DocumentAccess> {
  try { return { states: await load(), refreshError: null }; }
  catch(error) { return { states: previous.states, refreshError: String(error) }; }
}
export function copyRequest(path: string, version: 'edited'|'original', draft: Fields = {}, covers?: PendingCover[]) {
  return { path, version, updates: version === 'edited' ? structuredClone(draft) : {},
    ...(version === 'edited' && covers !== undefined ? { covers: structuredClone(covers) } : {}) };
}
export function copyOutcome(batch: {entries: unknown[]; errors: unknown[]}) {
  return batch.errors.length ? 'failed' : batch.entries.length ? 'saved' : 'cancelled';
}
export function sourcePresentation(state: DocumentState|undefined, stale = false) {
  return {
    name: state?.name || 'Provider document', location: state?.uri || 'Source location unavailable',
    label: !state || stale ? 'Source access unknown' : state.state === 'recovery_required' ? 'Recovery required' : state.writable ? 'Source writable' : 'Source read-only',
    detail: !state ? 'Refresh source access before saving.' : `${stale || state.state === 'recovery_required' ? 'Previous source details: ' : ''}${state.persisted ? 'Access retained' : 'Access for this session'} · ${state.state === 'recovery_required' ? 'Edited and original copies retained' : 'Save a copy available'}`,
  };
}
export function publicDocumentMessage(error: unknown, states: Record<string, DocumentState>): string {
  let message = String(error);
  for(const [path, state] of Object.entries(states)) message = message.split(path).join(state.name);
  // Transport errors can refer to a record not yet returned by the picker or recovery command.
  if(/\/(?:data\/(?:user|data)|(?:storage\/emulated\/\d+\/Android|sdcard\/Android)\/data)\//.test(message))
    return 'The document operation failed. Private working files were retained.';
  return message;
}

export function reconcileDrafts(drafts: Map<string, Fields>, successfulPaths: string[], clear: boolean) {
  if(clear)successfulPaths.forEach(path=>drafts.delete(path));
}
