#[derive(Debug, PartialEq, Eq)]
pub enum Presentation {
    Hidden,
    Preview,
    Game,
}
pub fn presentation(
    available: bool,
    enabled: bool,
    exclusive: bool,
    ready: bool,
    foreground: bool,
    preview: bool,
) -> Presentation {
    if !available || exclusive {
        Presentation::Hidden
    } else if preview {
        Presentation::Preview
    } else if enabled && !exclusive && ready && foreground {
        Presentation::Game
    } else {
        Presentation::Hidden
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partie_focus_activation_et_arret() {
        assert_eq!(
            presentation(true, true, false, true, true, false),
            Presentation::Game
        );
        for (available, enabled, exclusive, ready, foreground) in [
            (false, true, false, true, true),
            (true, false, false, true, true),
            (true, true, true, true, true),
            (true, true, false, false, true),
            (true, true, false, true, false),
        ] {
            assert_eq!(
                presentation(available, enabled, exclusive, ready, foreground, false),
                Presentation::Hidden
            );
        }
    }
    #[test]
    fn apercu_independant_du_jeu_mais_jamais_si_initialisation_echouee() {
        assert_eq!(
            presentation(true, false, false, false, false, true),
            Presentation::Preview
        );
        assert_eq!(
            presentation(false, true, false, true, true, true),
            Presentation::Hidden
        );
        assert_eq!(
            presentation(true, true, true, true, true, true),
            Presentation::Hidden
        );
        assert_eq!(
            presentation(true, true, false, false, false, false),
            Presentation::Hidden
        );
    }
}
