#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Vrai pour les pixels Windows, faux pour les points macOS.
    pub physical: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementError {
    Frame,
    Position,
    Width,
}

/// Les préférences restent relatives au cadre du jeu, quelle que soit sa résolution.
pub fn placement(frame: Rect, x: f64, y: f64, width_relative: f64) -> Result<Rect, PlacementError> {
    if ![frame.x, frame.y, frame.width, frame.height]
        .iter()
        .all(|value| value.is_finite())
        || frame.width <= 0.0
        || frame.height <= 0.0
        || !(frame.x + frame.width).is_finite()
        || !(frame.y + frame.height).is_finite()
    {
        return Err(PlacementError::Frame);
    }
    if !x.is_finite() || !y.is_finite() {
        return Err(PlacementError::Position);
    }
    if !(0.1..=0.5).contains(&width_relative) {
        return Err(PlacementError::Width);
    }
    let width = frame.width * width_relative;
    // Réserve verticale transparente pour le résumé statique ; aucun bloc ne capte les clics.
    let height = frame.height * 0.5;
    Ok(Rect {
        x: frame.x + frame.width * x.clamp(0.0, 1.0 - width_relative),
        y: frame.y + frame.height * y.clamp(0.0, 0.5),
        width,
        height,
        physical: frame.physical,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(width: f64, height: f64, physical: bool) -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width,
            height,
            physical,
        }
    }

    fn near(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.000_001,
            "{actual} != {expected}"
        );
    }

    #[test]
    fn suit_le_cadre_du_jeu_sur_un_ecran_aux_coordonnees_negatives() {
        let rect = placement(
            Rect {
                x: -1920.0,
                y: -100.0,
                ..frame(1920.0, 1080.0, true)
            },
            0.1,
            0.2,
            0.25,
        )
        .unwrap();
        near(rect.x, -1728.0);
        near(rect.y, 116.0);
        near(rect.width, 480.0);
        near(rect.height, 540.0);
        assert!(rect.physical);
    }

    #[test]
    fn conserve_les_points_logiques_retina_sans_doubler_les_coordonnees() {
        let rect = placement(
            Rect {
                x: 1440.0,
                y: 30.0,
                ..frame(1512.0, 982.0, false)
            },
            0.5,
            0.5,
            0.25,
        )
        .unwrap();
        near(rect.x, 2196.0);
        near(rect.y, 521.0);
        near(rect.width, 378.0);
        near(rect.height, 491.0);
        assert!(!rect.physical);
    }

    #[test]
    fn suit_les_pixels_du_jeu_en_125_et_150_pourcent_et_en_ultrawide() {
        for (width, height, want_x, want_y, want_width, want_height) in [
            (1600.0, 900.0, 160.0, 180.0, 400.0, 450.0),
            (1920.0, 1080.0, 192.0, 216.0, 480.0, 540.0),
            (1366.0, 768.0, 136.6, 153.6, 341.5, 384.0),
            (2560.0, 1440.0, 256.0, 288.0, 640.0, 720.0),
            (3440.0, 1440.0, 344.0, 288.0, 860.0, 720.0),
        ] {
            let rect = placement(frame(width, height, true), 0.1, 0.2, 0.25).unwrap();
            near(rect.x, want_x);
            near(rect.y, want_y);
            near(rect.width, want_width);
            near(rect.height, want_height);
            assert!(rect.physical);
        }
    }

    #[test]
    fn maintient_le_panneau_entier_dans_la_fenetre() {
        let rect = placement(frame(1000.0, 500.0, true), 2.0, 1.0, 0.5).unwrap();
        near(rect.x, 500.0);
        near(rect.y, 250.0);
        near(rect.width, 500.0);
        near(rect.height, 250.0);
        let rect = placement(frame(1000.0, 500.0, true), -1.0, -2.0, 0.1).unwrap();
        near(rect.x, 0.0);
        near(rect.y, 0.0);
        near(rect.width, 100.0);
    }

    #[test]
    fn refuse_les_dimensions_et_preferences_invalides() {
        for invalid in [
            frame(0.0, 1080.0, true),
            frame(1920.0, -1.0, true),
            frame(f64::INFINITY, 1080.0, true),
            Rect {
                x: f64::NAN,
                ..frame(1920.0, 1080.0, true)
            },
            Rect {
                x: f64::MAX,
                ..frame(f64::MAX, 1080.0, true)
            },
        ] {
            assert_eq!(
                placement(invalid, 0.0, 0.0, 0.25),
                Err(PlacementError::Frame)
            );
        }
        for (x, y) in [(f64::NAN, 0.0), (0.0, f64::INFINITY)] {
            assert_eq!(
                placement(frame(1920.0, 1080.0, true), x, y, 0.25),
                Err(PlacementError::Position)
            );
        }
        for width in [0.09, 0.51, f64::NAN, f64::INFINITY] {
            assert_eq!(
                placement(frame(1920.0, 1080.0, true), 0.0, 0.0, width),
                Err(PlacementError::Width)
            );
        }
    }
}

/// La hauteur provient du contenu web en unités logiques ; rien n'est persisté en pixels.
pub fn fit_content(
    frame: Rect,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale: f64,
) -> Result<Rect, PlacementError> {
    if !(48.0..=1200.0).contains(&height) || !(0.5..=8.0).contains(&scale) {
        return Err(PlacementError::Frame);
    }
    let mut rect = placement(frame, x, y, width)?;
    rect.height = (height * scale).min(frame.height * 0.8);
    rect.y = frame.y + (frame.height * y).clamp(0.0, frame.height - rect.height);
    Ok(rect)
}

#[cfg(test)]
mod content_tests {
    use super::*;
    #[test]
    fn ajuste_la_hauteur_au_contenu_sans_depasser_le_jeu() {
        let frame = Rect {
            x: -1920.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
            physical: true,
        };
        let rect = fit_content(frame, 0.1, 0.9, 0.25, 240.0, 1.5).unwrap();
        assert_eq!(rect.height, 360.0);
        assert_eq!(rect.y, 720.0);
        assert_eq!(rect.x, -1728.0);
        let mac = fit_content(
            Rect {
                physical: false,
                ..frame
            },
            0.1,
            0.1,
            0.25,
            240.0,
            1.0,
        )
        .unwrap();
        assert_eq!(mac.height, 240.0);
        assert!(fit_content(frame, 0.1, 0.1, 0.2, f64::NAN, 1.0).is_err());
    }
}
