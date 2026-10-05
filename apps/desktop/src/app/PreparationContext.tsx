import type {LcuSession} from '@olc/shared';
import {createContext,useContext} from 'react';
import {initialPreparation,initialState,type PreparationState} from './state';
/** La consultation appartient à la navigation, pas au panneau démonté au retour. */
export const PreparationContext=createContext<{rankReady?:boolean;defaultRank?:string;session?:LcuSession;value:PreparationState;update:(patch:Partial<PreparationState>)=>void}>({session:initialState.session,value:initialPreparation,update:()=>{}});
export const usePreparation=()=>useContext(PreparationContext);
