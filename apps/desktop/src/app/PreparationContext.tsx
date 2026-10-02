import {createContext,useContext} from 'react';
import {initialPreparation,type PreparationState} from './state';
/** La consultation appartient à la navigation, pas au panneau démonté au retour. */
export const PreparationContext=createContext<{value:PreparationState;update:(patch:Partial<PreparationState>)=>void}>({value:initialPreparation,update:()=>{}});
export const usePreparation=()=>useContext(PreparationContext);
