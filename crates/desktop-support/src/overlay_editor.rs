//! Géométrie relative des gestes, indépendante du système de fenêtres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panel {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy)]
pub enum Gesture {
    Move,
    Resize,
}
pub fn transform(panel: Panel, gesture: Gesture, dx: f64, dy: f64) -> Option<Panel> {
    if ![panel.x, panel.y, panel.width, panel.height, dx, dy]
        .iter()
        .all(|v| v.is_finite())
        || panel.x < 0.0
        || panel.y < 0.0
        || panel.x > 0.9
        || panel.y > 0.9
        || !(0.1..=0.5).contains(&panel.width)
        || !(0.1..=0.8).contains(&panel.height)
        || panel.x + panel.width > 1.000001
        || panel.y + panel.height > 1.000001
    {
        return None;
    }
    Some(match gesture {
        Gesture::Move => Panel {
            x: (panel.x + dx).clamp(0.0, 1.0 - panel.width),
            y: (panel.y + dy).clamp(0.0, 1.0 - panel.height),
            ..panel
        },
        Gesture::Resize => Panel {
            width: (panel.width + dx).clamp(0.1, 0.5_f64.min(1.0 - panel.x).max(0.1)),
            height: (panel.height + dy).clamp(0.1, 0.8_f64.min(1.0 - panel.y).max(0.1)),
            ..panel
        },
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn panel() -> Panel {
        Panel {
            x: 0.2,
            y: 0.3,
            width: 0.25,
            height: 0.2,
        }
    }
    #[test]
    fn deplacement_borne_sans_modifier_la_taille() {
        assert_eq!(
            transform(panel(), Gesture::Move, 2.0, -2.0),
            Some(Panel {
                x: 0.75,
                y: 0.0,
                ..panel()
            })
        );
    }
    #[test]
    fn redimensionnement_reste_dans_le_cadre() {
        let p = Panel {
            x: 0.8,
            y: 0.7,
            width: 0.1,
            height: 0.2,
        };
        let result = transform(p, Gesture::Resize, 1.0, 1.0).unwrap();
        assert!((result.width - 0.2).abs() < 1e-6);
        assert!((result.height - 0.3).abs() < 1e-6);
    }
    #[test]
    fn refuse_non_fini_et_origine_invalide() {
        assert!(transform(panel(), Gesture::Move, f64::NAN, 0.0).is_none());
        assert!(transform(Panel { x: 2.0, ..panel() }, Gesture::Move, 0.0, 0.0).is_none());
    }
}
