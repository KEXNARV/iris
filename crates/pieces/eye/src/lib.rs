//! The built-in eye as a recipe part: where the pupil goes and how big it is, published for the
//! body to draw. The same arithmetic as `src/core/pieces/eye.rs`, so a recipe with this one in
//! place of `builtin:eye` draws the same dots.

use iris_piece::export_part;
use iris_piece::part::*;

struct Eye;

impl Part for Eye {
    const INPUTS: &'static [&'static str] = &["body.x", "body.y", "gaze.x", "gaze.y", "pupil", "dilate", "red", "yawn", "lens", "blink", "ink.body"];
    const OUTPUTS: &'static [&'static str] = &["pupil.x", "pupil.y", "pupil.r", "pupil.blink", "pupil.ink"];
    const SETTINGS: &'static [Setting] = &[
        Setting::new("radius", 0.085, 0.02, 0.3),
        // Never goes below this many dots of radius.
        Setting::new("min_dots", 2.5, 0.0, 10.0),
        // How much it contracts on an error and when yawning; how much it grows under the magnifier.
        Setting::new("error", 0.45, 0.0, 1.0),
        Setting::new("yawn", 0.8, 0.0, 1.0),
        Setting::new("magnifier", 1.5, 1.0, 3.0),
    ];

    fn new(_io: &Io, _rand: &mut Rand) -> Self {
        Eye
    }

    fn layout(&self, io: &mut Io) {
        let v = |i: usize| io.input(i);
        let (pupil, dilate, red, yawn, lens) = (v(4), v(5), v(6), v(7), v(8));
        let (radius, min_dots, error, yawn_k, magnifier) = (io.setting(0), io.setting(1), io.setting(2), io.setting(3), io.setting(4));
        let pr = if pupil > 0.01 { (radius * dilate * (1.0 - red * error)).max(min_dots * io.du()) * pupil * (1.0 - yawn * yawn_k) } else { 0.0 };
        let r = pr * if lens > 0.5 { magnifier } else { 1.0 };
        let (x, y, blink, ink) = (v(0) + v(2), v(1) + v(3), v(9), v(10));
        for (i, o) in [x, y, r, blink, ink].into_iter().enumerate() {
            io.put(i, o);
        }
    }
}

export_part!(Eye);
