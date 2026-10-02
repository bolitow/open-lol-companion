import type {DraftRuneImportError,DraftSession,ImportDraftRunesRequest} from '@olc/shared';
export interface RuneImportSnapshot {pending:boolean;result:{context:string;status:'accepted'}|{context:string;status:'error';error:DraftRuneImportError|'unknown'}|null}
const errors:DraftRuneImportError[]=['clientUnavailable','clientRejected','invalidClientData','invalidRunes','runePageUnavailable','draftContextChanged','unsupportedMode','importBusy','notInChampSelect','invalidSpells','invalidItems','itemSetPriorityUnavailable'];
/** Un seul envoi même si le panneau est remonté ; une réponse ancienne reste invisible. */
export function createImportController<Request>(send:(request:Request)=>Promise<unknown>){
 let snapshot:RuneImportSnapshot={pending:false,result:null},context:string|null=null,generation=0;
 const listeners=new Set<()=>void>();
 const update=(next:RuneImportSnapshot)=>{snapshot=next;listeners.forEach(fn=>fn())};
 return {
  getSnapshot:()=>snapshot,
  subscribe:(fn:()=>void)=>{listeners.add(fn);return()=>{listeners.delete(fn)}},
  activate:(next:string|null)=>{if(next!==context){context=next;generation++;update({...snapshot,result:null})}},
  submit:async(key:string,request:Request)=>{
   if(snapshot.pending||key!==context)return;
   const started=generation;update({pending:true,result:null});
   let result:RuneImportSnapshot['result'];
   try {await send(request);result={context:key,status:'accepted'}}
   catch(error){result={context:key,status:'error',error:typeof error==='string'&&errors.includes(error as DraftRuneImportError)?error as DraftRuneImportError:'unknown'}}
   update({pending:false,result:generation===started&&key===context?result:null});
  },
 };
}
export function importEligibility(native:boolean,draft:DraftSession|null,championId:number,valid:boolean):'desktop'|'draft'|'champion'|'incomplete'|null{
 if(!native)return 'desktop';
 if(!draft?.supported)return 'draft';
 const local=draft.allies.filter(p=>p.local);
 if(!championId||local.length!==1||local[0]?.championId!==championId)return 'champion';
 return valid?null:'incomplete';
}

export const createRuneImportController=createImportController<ImportDraftRunesRequest>;
