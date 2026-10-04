import type {ApiAccessError} from '@olc/shared';
const fr={
 path:'Application / Connexion',title:'Accès à l’API',
 description:'Adresse du service et jeton d’accès utilisés par les builds communautaires et les profils joueurs. Le jeton est gardé dans le trousseau du système et n’est plus jamais affiché.',
 url:'Adresse du service',urlPlaceholder:'https://…',token:'Jeton d’accès',tokenPlaceholder:'Collez le jeton reçu',tokenKept:'Un jeton est enregistré. Saisissez-en un nouveau pour le remplacer.',
 save:'Enregistrer',saving:'Enregistrement…',clear:'Retirer le jeton',loading:'Lecture du trousseau…',
 desktop:'L’accès à l’API se configure dans l’application desktop.',
 none:'Non configuré : les builds communautaires et les profils joueurs restent indisponibles.',
 keychain:'Configuré depuis le trousseau du système.',
 environment:'Configuré par les variables d’environnement OLC_API_URL et OLC_API_TOKEN, prioritaires sur ce réglage.',
 saved:'Accès enregistré dans le trousseau du système.',cleared:'Jeton retiré du trousseau du système.',
 errors:{
  invalid_configuration:'Adresse ou jeton refusé. L’adresse doit être en HTTPS (HTTP seulement en local), sans chemin ni paramètre.',
  too_long:'Ce jeton est trop long pour le trousseau du système.',
  storage_unavailable:'Le trousseau du système est inaccessible sur cet appareil.',
  read_failed:'Le trousseau du système n’a pas pu être lu.',
  write_failed:'Le trousseau du système n’a pas pu être modifié.',
 } satisfies Record<ApiAccessError,string>,
};
const en:typeof fr={
 path:'Application / Connection',title:'API access',
 description:'Service address and access token used by community builds and player profiles. The token is kept in the system keychain and is never shown again.',
 url:'Service address',urlPlaceholder:'https://…',token:'Access token',tokenPlaceholder:'Paste the token you received',tokenKept:'A token is saved. Enter a new one to replace it.',
 save:'Save',saving:'Saving…',clear:'Remove token',loading:'Reading the keychain…',
 desktop:'API access is configured in the desktop application.',
 none:'Not configured: community builds and player profiles remain unavailable.',
 keychain:'Configured from the system keychain.',
 environment:'Configured by the OLC_API_URL and OLC_API_TOKEN environment variables, which take priority over this setting.',
 saved:'Access saved in the system keychain.',cleared:'Token removed from the system keychain.',
 errors:{
  invalid_configuration:'Address or token rejected. The address must use HTTPS (HTTP only for local addresses), with no path or parameters.',
  too_long:'This token is too long for the system keychain.',
  storage_unavailable:'The system keychain is not available on this device.',
  read_failed:'The system keychain could not be read.',
  write_failed:'The system keychain could not be updated.',
 },
};
export const apiAccessCopy={fr,en};
