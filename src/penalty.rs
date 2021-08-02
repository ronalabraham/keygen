/// Methods for calculating the penalty of a keyboard layout given an input
/// corpus string.

use std::vec::Vec;
use std::ops::Range;
use std::collections::HashMap;
use std::fmt;

use layout::Layout;
use layout::LayoutPosMap;
use layout::KeyMap;
use layout::KeyPress;
use layout::Finger;
use layout::Row;
use layout::KP_NONE;

pub struct KeyPenalty<'a>
{
    name:      &'a str,
}

#[derive(Clone)]
pub struct KeyPenaltyResult<'a>
{
    pub name:  &'a str,
    pub total:     f64,
    pub high_keys: HashMap<&'a str, f64>,
}

pub struct QuartadList<'a>(HashMap<&'a str, usize>);

impl <'a> fmt::Display for KeyPenaltyResult<'a>
{
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.total)
    }
}

// static BASE_PENALTY: KeyMap<f64> = KeyMap([
//     3.0, 1.0, 1.0, 1.5, 3.0,    3.0, 1.5, 1.0, 1.0, 3.0, 4.0,
//     0.5, 0.5, 0.0, 0.0, 1.5,    1.5, 0.0, 0.0, 0.5, 0.5, 2.0,
//     2.0, 2.0, 1.5, 1.5, 2.5,    2.5, 1.5, 1.5, 2.0, 2.0,
//                         0.0,    0.0]);

static BASE_PENALTY: KeyMap<f64> = KeyMap([
    3.50, 0.60, 0.60, 1.50, 2.50,    2.50, 1.50, 0.60, 0.60, 3.50, 4.00,
    0.80, 0.25, 0.00, 0.00, 1.50,    1.50, 0.00, 0.00, 0.25, 0.80, 3.50,
    3.00, 2.00, 1.50, 1.00, 2.00,    2.00, 1.00, 1.50, 2.00, 3.00,
                            0.00,    0.00]);

pub fn init<'a>()
-> Vec<KeyPenalty<'a>>
{
    let mut penalties = Vec::new();

    // Base penalty.
    penalties.push(KeyPenalty {
        name: "base",
    });

    // 1. Penalize 0.5 points for alternating hands three times in a row.
    penalties.push(KeyPenalty {
        name: "alternating hand",
    });

    // 2. Penalize 5 points for using the same finger twice on different keys.
    // An extra 10 points for using the outer right keys. Note: the penalty for
    // consecutive index finger usage is significantly more nuanced because
    // some patterns (e.g. G->R on Qwerty) can be typed easily by moving the
    // middle finger over to the index finger's place. See the weights.xlsx
    // file for details.
    penalties.push(KeyPenalty {
        name: "same finger",
    });

    // 3. Penalize 1 point for jumping from top to bottom row or from bottom to
    // top row on the same hand.
    penalties.push(KeyPenalty {
        name: "long jump hand",
    });

    // 4. Penalize 10 points for jumping from top to bottom row or from bottom
    // to top row on the same finger. Note: there is no penalty for the index
    // finger doing a "long jump" because the difficulty is entirely captured
    // by the corresponding "same finger" penalty. See the weights.xlsx file
    // for details.
    penalties.push(KeyPenalty {
        name: "long jump",
    });

    // 5. Penalize some points for jumping from top to bottom row or from
    // bottom to top row on consecutive fingers. The exact penalty is nuanced;
    // see the weights.xlsx file for details.
    penalties.push(KeyPenalty {
        name: "long jump consecutive",
    });

    // 6. Penalize some points for awkward pinky/ring combination where the
    // pinky reaches above the ring finger, e.g. SQ/QS, XQ/QX on Qwerty. The
    // exact penalty is nuanced; see the weights.xlsx file for details.
    penalties.push(KeyPenalty {
        name: "pinky/ring twist",
    });

    // 7. Penalize 0.1 points for using the same hand four times in a row.
    penalties.push(KeyPenalty {
        name: "same hand",
    });

    // 8. Penalize 20 points for reversing a roll at the end of the hand, i.e.
    // using the ring, pinky, then middle finger of the same hand, or the
    // middle, pinky, then ring of the same hand.
    penalties.push(KeyPenalty {
        name: "roll reversal",
    });

    // 9. Penalize 0.125 points for rolling outwards.
    penalties.push(KeyPenalty {
        name: "roll out",
    });

    // 10. Award 0.125 points for rolling inwards.
    penalties.push(KeyPenalty {
        name: "roll in",
    });

    // 11. Penalize 3 points for jumping from top to bottom row or from bottom
    // to top row on the same finger with a keystroke in between.
    penalties.push(KeyPenalty {
        name: "long jump sandwich",
    });

    // 12. Penalize 10 points for three consecutive keystrokes going up or down
    // the three rows of the keyboard in a roll.
    penalties.push(KeyPenalty {
        name: "twist",
    });

    // 13. Penalize 15 point for pinky/ring alternation on the same hand. For
    // example POP or SAS on Qwerty.
    penalties.push(KeyPenalty {
        name: "pinky/ring alternation",
    });

    penalties
}

pub fn prepare_quartad_list<'a>(
    string:       &'a str,
    position_map: &'a LayoutPosMap)
-> QuartadList<'a>
{
    let mut range: Range<usize> = 0..0;
    let mut quartads: HashMap<&str, usize> = HashMap::new();
    for (i, c) in string.chars().enumerate() {
        match *position_map.get_key_position(c) {
            Some(_) => {
                range.end = i + 1;
                if range.end > 3 && range.start < range.end - 4 {
                    range.start = range.end - 4;
                }
                let quartad = &string[range.clone()];
                let entry = quartads.entry(quartad).or_insert(0);
                *entry += 1;
            },
            None => {
                range = (i + 1)..(i + 1);
            }
        }
    }

    QuartadList(quartads)
}

pub fn calculate_penalty<'a>(
    quartads:  &   QuartadList<'a>,
    len:           usize,
    layout:    &   Layout,
    penalties: &'a Vec<KeyPenalty>,
    detailed:      bool)
-> (f64, f64, Vec<KeyPenaltyResult<'a>>)
{
    let QuartadList(ref quartads) = *quartads;
    let mut result: Vec<KeyPenaltyResult> = Vec::new();
    let mut total = 0.0;

    if detailed {
        for penalty in penalties {
            result.push(KeyPenaltyResult {
                name: penalty.name,
                total: 0.0,
                high_keys: HashMap::new(),
            });
        }
    }

    let position_map = layout.get_position_map();
    for (string, count) in quartads {
        total += penalty_for_quartad(string, *count, &position_map, &mut result, detailed);
    }

    (total, total / (len as f64), result)
}

fn penalty_for_quartad<'a, 'b>(
    string:       &'a str,
    count:            usize,
    position_map: &'b LayoutPosMap,
    result:       &'b mut Vec<KeyPenaltyResult<'a>>,
    detailed:         bool)
-> f64
{
    let mut chars = string.chars().into_iter().rev();
    let opt_curr = chars.next();
    let opt_old1 = chars.next();
    let opt_old2 = chars.next();
    let opt_old3 = chars.next();

    let curr = match opt_curr {
        Some(c) => match position_map.get_key_position(c) {
            &Some(ref kp) => kp,
            &None => { return 0.0 }
        },
        None => panic!("unreachable")
    };
    let old1 = match opt_old1 {
        Some(c) => position_map.get_key_position(c),
        None => &KP_NONE
    };
    let old2 = match opt_old2 {
        Some(c) => position_map.get_key_position(c),
        None => &KP_NONE
    };
    let old3 = match opt_old3 {
        Some(c) => position_map.get_key_position(c),
        None => &KP_NONE
    };

    penalize(string, count, &curr, old1, old2, old3, result, detailed)
}

fn penalize<'a, 'b>(
    string: &'a     str,
    count:          usize,
    curr:   &              KeyPress,
    old1:   &       Option<KeyPress>,
    old2:   &       Option<KeyPress>,
    old3:   &       Option<KeyPress>,
    result: &'b mut Vec<KeyPenaltyResult<'a>>,
    detailed:       bool)
-> f64
{
    let len = string.len();
    let count = count as f64;
    let mut total = 0.0;

    // One key penalties.
    let slice1 = &string[(len - 1)..len];

    // 0: Base penalty.
    let base = BASE_PENALTY.0[curr.pos] * count;
    if detailed {
        *result[0].high_keys.entry(slice1).or_insert(0.0) += base;
        result[0].total += base;
    }
    total += base;

    // Two key penalties.
    let old1 = match *old1 {
        Some(ref o) => o,
        None => { return total }
    };

    if curr.hand == old1.hand {
        let slice2 = &string[(len - 2)..len];

        // 2: Same finger.
        if curr.finger == old1.finger && curr.pos != old1.pos {
            let penalty = calculate_same_finger_penalty(curr, old1);
            let penalty = penalty * count;
            if detailed && penalty > 0. {
                *result[2].high_keys.entry(slice2).or_insert(0.0) += penalty;
                result[2].total += penalty;
            }
            total += penalty;
        }

        // 3: Long jump hand.
        if curr.row == Row::Top && old1.row == Row::Bottom ||
           curr.row == Row::Bottom && old1.row == Row::Top {
            let penalty = count;
            if detailed {
                *result[3].high_keys.entry(slice2).or_insert(0.0) += penalty;
                result[3].total += penalty;
            }
            total += penalty;
        }

        // 4: Long jump.
        if curr.finger == old1.finger && curr.finger != Finger::Index {
            if curr.row == Row::Top && old1.row == Row::Bottom ||
               curr.row == Row::Bottom && old1.row == Row::Top {
                let penalty = 10.0 * count;
                if detailed {
                    *result[4].high_keys.entry(slice2).or_insert(0.0) += penalty;
                    result[4].total += penalty;
                }
                total += penalty;
            }
        }

        // 5: Long jump consecutive.
        if curr.row == Row::Top && old1.row == Row::Bottom ||
           curr.row == Row::Bottom && old1.row == Row::Top {
            let penalty = calculate_long_jump_consecutive_penalty(curr, old1);
            let penalty = penalty * count;
            if detailed && penalty > 0. {
                *result[5].high_keys.entry(slice2).or_insert(0.0) += penalty;
                result[5].total += penalty;
            }
            total += penalty;
        }

        // 6: Pinky/ring twist.
        if (curr.finger == Finger::Ring && old1.finger == Finger::Pinky) ||
           (curr.finger == Finger::Pinky && old1.finger == Finger::Ring) {
            let penalty = calculate_pinky_ring_twist(curr, old1);
            let penalty = penalty * count;
            if detailed && penalty > 0. {
                *result[6].high_keys.entry(slice2).or_insert(0.0) += penalty;
                result[6].total += penalty;
            }
            total += penalty;
        }

        // 9: Roll out.
        if is_roll_out(curr.finger, old1.finger) {
            let penalty = 0.125 * count;
            if detailed {
                *result[9].high_keys.entry(slice2).or_insert(0.0) += penalty;
                result[9].total += penalty;
            }
            total += penalty;
        }

        // 10: Roll in.
        if is_roll_in(curr.finger, old1.finger) {
            let penalty = -0.125 * count;
            if detailed {
                *result[10].high_keys.entry(slice2).or_insert(0.0) += penalty;
                result[10].total += penalty;
            }
            total += penalty;
        }
    }

    // Three key penalties.
    let old2 = match *old2 {
        Some(ref o) => o,
        None => { return total },
    };

    if curr.hand == old1.hand && old1.hand == old2.hand {
        let slice3 = &string[(len - 3)..len];

        // 8: Roll reversal.
        if (curr.finger == Finger::Middle &&
            old1.finger == Finger::Pinky &&
            old2.finger == Finger::Ring) ||
           (curr.finger == Finger::Ring &&
            old1.finger == Finger::Pinky &&
            old2.finger == Finger::Middle) {
            let penalty = 20.0 * count;
            if detailed {
                *result[8].high_keys.entry(slice3).or_insert(0.0) += penalty;
                result[8].total += penalty;
            }
            total += penalty;
        }

        // 12: Twist.
        if ((curr.row == Row::Top && old1.row == Row::Home && old2.row == Row::Bottom) ||
            (curr.row == Row::Bottom && old1.row == Row::Home && old2.row == Row::Top)) &&
           ((is_roll_out(curr.finger, old1.finger) && is_roll_out(old1.finger, old2.finger)) ||
               (is_roll_in(curr.finger, old1.finger) && is_roll_in(old1.finger, old2.finger))) {
            let penalty = 10.0 * count;
            if detailed {
                *result[12].high_keys.entry(slice3).or_insert(0.0) += penalty;
                result[12].total += penalty;
            }
            total += penalty;
        }

        // 13: Pinky/ring alternation.
        if (curr.finger == Finger::Ring &&
            old1.finger == Finger::Pinky &&
            old2.finger == Finger::Ring) ||
           (curr.finger == Finger::Pinky &&
            old1.finger == Finger::Ring &&
            old2.finger == Finger::Pinky) {
            let penalty = 15.0 * count;
            if detailed {
                *result[13].high_keys.entry(slice3).or_insert(0.0) += penalty;
                result[13].total += penalty;
            }
            total += penalty;
        }
    }

    // 11: Long jump sandwich.
    if curr.hand == old2.hand && curr.finger == old2.finger {
        if curr.row == Row::Top && old2.row == Row::Bottom ||
           curr.row == Row::Bottom && old2.row == Row::Top {
            let slice3 = &string[(len - 3)..len];
            let penalty = 3.0 * count;
            if detailed {
                *result[11].high_keys.entry(slice3).or_insert(0.0) += penalty;
                result[11].total += penalty;
            }
            total += penalty;
        }
    }

    // Four key penalties.
    let old3 = match *old3 {
        Some(ref o) => o,
        None => { return total },
    };

    if curr.hand == old1.hand && old1.hand == old2.hand && old2.hand == old3.hand {
        // 7: Same hand.
        let slice4 = &string[(len - 4)..len];
        let penalty = 0.1 * count;
        if detailed {
            *result[7].high_keys.entry(slice4).or_insert(0.0) += penalty;
            result[7].total += penalty;
        }
        total += penalty;
    } else if curr.hand != old1.hand && old1.hand != old2.hand && old2.hand != old3.hand {
        // 1: Alternating hand.
        let slice4 = &string[(len - 4)..len];
        let penalty = 0.5 * count;
        if detailed {
            *result[1].high_keys.entry(slice4).or_insert(0.0) += penalty;
            result[1].total += penalty;
        }
        total += penalty;
    }

    total
}

fn calculate_same_finger_penalty(curr: &KeyPress, old1: &KeyPress)
-> f64 {

    // This penalty should only be calculated if we consecutively use the
    // same finger on the same hand, but for a different key.
    assert!(curr.hand == old1.hand);
    assert!(curr.finger == old1.finger);
    assert!(curr.pos != old1.pos);

    if curr.finger == Finger::Index {
        // In the following comments, all letter combinations are on Qwerty.

        // gr/rg/hu/uh
        if curr.pos == 15 && old1.pos == 3 ||
           curr.pos == 3 && old1.pos == 15 ||
           curr.pos == 16 && old1.pos == 6 ||
           curr.pos == 6 && old1.pos == 16 {
            return 2.;
        }
        // fg/gf/hj/jh
        if curr.pos == 14 && old1.pos == 15 ||
           curr.pos == 15 && old1.pos == 14 ||
           curr.pos == 16 && old1.pos == 17 ||
           curr.pos == 17 && old1.pos == 16 {
            return 3.;
        }
        // fr/rf/ju/uj
        if curr.pos == 14 && old1.pos == 3 ||
           curr.pos == 3 && old1.pos == 14 ||
           curr.pos == 17 && old1.pos == 6 ||
           curr.pos == 6 && old1.pos == 17 {
            return 4.;
        }
        // vf/fv/mj/jm
        if curr.pos == 25 && old1.pos == 14 ||
           curr.pos == 14 && old1.pos == 25 ||
           curr.pos == 28 && old1.pos == 17 ||
           curr.pos == 17 && old1.pos == 28 {
            return 5.;
        }
        // bf/fb/nj/jn
        if curr.pos == 26 && old1.pos == 14 ||
           curr.pos == 14 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 17 ||
           curr.pos == 17 && old1.pos == 27 {
            return 7.;
        }
        // rt/tr/yu/uy
        if curr.pos == 3 && old1.pos == 4 ||
           curr.pos == 4 && old1.pos == 3 ||
           curr.pos == 5 && old1.pos == 6 ||
           curr.pos == 6 && old1.pos == 5 {
            return 8.;
        }
        // bv/vb/nm/mn
        if curr.pos == 26 && old1.pos == 25 ||
           curr.pos == 25 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 28 ||
           curr.pos == 28 && old1.pos == 27 {
            return 11.;
        }
        // ft/tf/jy/yj
        if curr.pos == 14 && old1.pos == 4 ||
           curr.pos == 4 && old1.pos == 14 ||
           curr.pos == 17 && old1.pos == 5 ||
           curr.pos == 5 && old1.pos == 17 {
            return 13.;
        }
        // br/rb/nu/un
        if curr.pos == 26 && old1.pos == 3 ||
           curr.pos == 3 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 6 ||
           curr.pos == 6 && old1.pos == 27 {
            return 14.;
        }
        // vr/rv/mu/um
        if curr.pos == 25 && old1.pos == 3 ||
           curr.pos == 3 && old1.pos == 25 ||
           curr.pos == 28 && old1.pos == 6 ||
           curr.pos == 6 && old1.pos == 28 {
            return 15.;
        }
        // vg/gv/mh/hm
        if curr.pos == 25 && old1.pos == 15 ||
           curr.pos == 15 && old1.pos == 25 ||
           curr.pos == 28 && old1.pos == 16 ||
           curr.pos == 16 && old1.pos == 28 {
            return 17.;
        }
        // bg/gb/nh/hn
        if curr.pos == 26 && old1.pos == 15 ||
           curr.pos == 15 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 16 ||
           curr.pos == 16 && old1.pos == 27 {
            return 18.;
        }
        // gt/tg/hy/yh
        if curr.pos == 15 && old1.pos == 4 ||
           curr.pos == 4 && old1.pos == 15 ||
           curr.pos == 16 && old1.pos == 5 ||
           curr.pos == 5 && old1.pos == 16 {
            return 20.;
        }
        // bt/tb/ny/yn
        if curr.pos == 26 && old1.pos == 4 ||
           curr.pos == 4 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 5 ||
           curr.pos == 5 && old1.pos == 27 {
            return 25.;
        }
        // vt/tv/my/ym
        if curr.pos == 25 && old1.pos == 4 ||
           curr.pos == 4 && old1.pos == 25 ||
           curr.pos == 28 && old1.pos == 5 ||
           curr.pos == 5 && old1.pos == 28 {
            return 28.;
        }

        assert!(false, "All index finger pairs must be covered by now");
    }

    assert!(!curr.center,
            "All center column key presses must be covered by now.");

    // Outer pinky usage is painful; 10 points to Slytherin.
    5.0 + if curr.outer { 10. } else { 0. } + if old1.outer { 10. } else { 0. }
}

fn calculate_long_jump_consecutive_penalty(curr: &KeyPress, old1: &KeyPress)
-> f64 {
    // This penalty should only be calculated if we jump from the bottom to
    // top row (or vice versa) on the same hand.
    assert!(curr.hand == old1.hand);
    assert!(curr.row == Row::Top && old1.row == Row::Bottom ||
            curr.row == Row::Bottom && old1.row == Row::Top);

    // In the following comments, all letter combinations are on Qwerty.

    // Ring <--> Pinky keypresses
    if curr.finger == Finger::Ring  && old1.finger == Finger::Pinky ||
       curr.finger == Finger::Pinky && old1.finger == Finger::Ring {

        // wz zw o/ /o
        if curr.pos == 1 && old1.pos == 22 ||
           curr.pos == 22 && old1.pos == 1 ||
           curr.pos == 8 && old1.pos == 31 ||
           curr.pos == 31 && old1.pos == 8 {
            return 5.;
        }

        // xq qx .p p.
        if curr.pos == 23 && old1.pos == 0 ||
           curr.pos == 0 && old1.pos == 23 ||
           curr.pos == 30 && old1.pos == 9 ||
           curr.pos == 9 && old1.pos == 30 {
            return 5.;
        }

        // .\ \.
        if curr.pos == 30 && old1.pos == 10 ||
           curr.pos == 10 && old1.pos == 30 {
            return 0.;
        }

        assert!(false, "All Ring/Pinky pairs must be covered by now");
    }

    // Middle <--> Ring keypresses
    if curr.finger == Finger::Middle  && old1.finger == Finger::Ring ||
       curr.finger == Finger::Ring && old1.finger == Finger::Middle {

        // ex xe i. .i
        if curr.pos == 2 && old1.pos == 23 ||
           curr.pos == 23 && old1.pos == 2 ||
           curr.pos == 7 && old1.pos == 30 ||
           curr.pos == 30 && old1.pos == 7 {
            return 2.;
        }

        // cw wc ,o o,
        if curr.pos == 24 && old1.pos == 1 ||
           curr.pos == 1 && old1.pos == 24 ||
           curr.pos == 29 && old1.pos == 8 ||
           curr.pos == 8 && old1.pos == 29 {
            return 3.;
        }

        assert!(false, "All Middle/Ring pairs must be covered by now");
    }

    // Index <--> Ring keypresses
    if curr.finger == Finger::Index  && old1.finger == Finger::Ring ||
       curr.finger == Finger::Ring && old1.finger == Finger::Index {
        // vw wv mo om
        if curr.pos == 25 && old1.pos == 1 ||
           curr.pos == 1 && old1.pos == 25 ||
           curr.pos == 28 && old1.pos == 8 ||
           curr.pos == 8 && old1.pos == 28 {
            return 0.;
        }

        // bw wb no on
        if curr.pos == 26 && old1.pos == 1 ||
           curr.pos == 1 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 8 ||
           curr.pos == 8 && old1.pos == 27 {
            return 0.;
        }

        // rx xr u. .u
        if curr.pos == 3 && old1.pos == 23 ||
           curr.pos == 23 && old1.pos == 3 ||
           curr.pos == 6 && old1.pos == 30 ||
           curr.pos == 30 && old1.pos == 6 {
            return 1.;
        }

        // tx xt y. .y
        if curr.pos == 4 && old1.pos == 23 ||
           curr.pos == 23 && old1.pos == 4 ||
           curr.pos == 5 && old1.pos == 30 ||
           curr.pos == 30 && old1.pos == 5 {
            return 8.;
        }

        assert!(false, "All Index/Ring pairs must be covered by now");
    }

    // Index <--> Middle keypresses
    if curr.finger == Finger::Index  && old1.finger == Finger::Middle ||
       curr.finger == Finger::Middle && old1.finger == Finger::Index {
        // ve ev mi im
        if curr.pos == 25 && old1.pos == 2 ||
           curr.pos == 2 && old1.pos == 25 ||
           curr.pos == 28 && old1.pos == 7 ||
           curr.pos == 7 && old1.pos == 28 {
            return 0.;
        }

        // rc cr u, ,u
        if curr.pos == 3 && old1.pos == 24 ||
           curr.pos == 24 && old1.pos == 3 ||
           curr.pos == 6 && old1.pos == 29 ||
           curr.pos == 29 && old1.pos == 6 {
            return 4.;
        }

        // be eb ni in
        if curr.pos == 26 && old1.pos == 2 ||
           curr.pos == 2 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 7 ||
           curr.pos == 7 && old1.pos == 27 {
            return 7.;
        }

        // tc ct y, ,y
        if curr.pos == 4 && old1.pos == 24 ||
           curr.pos == 24 && old1.pos == 4 ||
           curr.pos == 5 && old1.pos == 29 ||
           curr.pos == 29 && old1.pos == 5 {
            return 10.;
        }

        assert!(false, "All Index/Middle pairs must be covered by now");
    }

    0.
}

fn calculate_pinky_ring_twist(curr: &KeyPress, old1: &KeyPress)
-> f64 {
    // This penalty should only be calculated if we alternate between pinky and
    // ring fingers on the same hand.
    assert!(curr.hand == old1.hand);
    assert!(
        (curr.finger == Finger::Ring && old1.finger == Finger::Pinky) ||
        (curr.finger == Finger::Pinky && old1.finger == Finger::Ring)
    );

    // In the following comments, all letter combinations are on Qwerty.

    // xa ax .; ;.
    if curr.pos == 23 && old1.pos == 11 ||
       curr.pos == 11 && old1.pos == 23 ||
       curr.pos == 30 && old1.pos == 20 ||
       curr.pos == 20 && old1.pos == 30 {
        return 0.;
    }

    // .' '.
    if curr.pos == 30 && old1.pos == 21 ||
       curr.pos == 21 && old1.pos == 30 {
        return 2.;
    }

    // l\ \l
    if curr.pos == 19 && old1.pos == 10 ||
       curr.pos == 10 && old1.pos == 19 {
        return 3.;
    }

    // sq qs lp pl
    if curr.pos == 12 && old1.pos == 0 ||
       curr.pos == 0 && old1.pos == 12 ||
       curr.pos == 19 && old1.pos == 9 ||
       curr.pos == 9 && old1.pos == 19 {
        return 10.;
    }

    // xq qx .p p.
    if curr.pos == 23 && old1.pos == 0 ||
       curr.pos == 0 && old1.pos == 23 ||
       curr.pos == 30 && old1.pos == 9 ||
       curr.pos == 9 && old1.pos == 30 {
        return 10.;
    }

    // .\ \.
    if curr.pos == 30 && old1.pos == 10 ||
       curr.pos == 10 && old1.pos == 30 {
        return 10.;
    }

    0.
}

fn is_roll_out(curr: Finger, prev: Finger) -> bool {
    match curr {
        Finger::Thumb  => false,
        Finger::Index  => prev == Finger::Thumb,
        Finger::Middle => prev == Finger::Thumb || prev == Finger::Index,
        Finger::Ring   => prev != Finger::Pinky && prev != Finger::Ring,
        Finger::Pinky  => prev != Finger::Pinky
    }
}

fn is_roll_in(curr: Finger, prev: Finger) -> bool {
    match curr {
        Finger::Thumb  => prev != Finger::Thumb,
        Finger::Index  => prev != Finger::Thumb && prev != Finger::Index,
        Finger::Middle => prev == Finger::Pinky || prev == Finger::Ring,
        Finger::Ring   => prev == Finger::Pinky,
        Finger::Pinky  => false,
    }
}
