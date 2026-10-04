//! Files de jeu identifiées par le collecteur (#97).
//!
//! Une file est « identifiée » quand son format est connu et contrôlable : taille des
//! camps, vainqueur, sous-équipes. Les autres identifiants restent stockés bruts mais
//! sont isolés des agrégats (exclusion `unknown_queue`) : leur sens n'est pas vérifiable
//! hors ligne. Le catalogue Data Dragon ne sert pas de référence ici : il ignore les
//! variantes Arena 1740 et 1750 pourtant observées (voir la recette multirégion).

/// Formats à deux camps de cinq : Faille, ARAM, Swiftplay, coop contre IA, modes rotatifs.
pub const STANDARD: &[i32] = &[
    400, 420, 430, 440, 450, 480, 490, 700, 720, 830, 840, 850, 870, 880, 890, 900, 1020, 1300,
    1400, 1900, 2300, 2400,
];
/// Arena : sous-équipes de 2 (1700, 1710) ou de 3 (1740, 1750 observées).
pub const ARENA: &[i32] = &[1700, 1710, 1740, 1750];
/// Swarm : un à quatre joueurs contre l'IA, le nombre de joueurs suit l'identifiant.
pub const SWARM: &[i32] = &[1810, 1820, 1830, 1840];

pub fn is_identified(queue_id: i32) -> bool {
    STANDARD.contains(&queue_id) || ARENA.contains(&queue_id) || SWARM.contains(&queue_id)
}
