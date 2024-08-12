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
    3.50, 0.25, 0.25, 1.50, 2.50,    2.50, 1.50, 0.25, 0.25, 3.50, 4.50,
    1.25, 0.00, 0.00, 0.00, 1.50,    1.50, 0.00, 0.00, 0.00, 1.25, 4.00,
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
    // An extra 10 points if the jump is between top and bottom rows. An extra
    // 5 points for each outer key. Note: the penalty for consecutive index
    // finger usage is significantly more nuanced because some patterns (e.g.
    // G->R on Qwerty) can be typed easily by moving the middle finger over to
    // the index finger's place. See the weights.xlsx file ("New--Same Finger"
    // sheet) for details.
    penalties.push(KeyPenalty {
        name: "same finger",
    });

    // 3. Penalize some points for using certain finger combinations (but not
    // the same finger) on the same hand. The actual penalty is nuanced and is
    // based on the amount of stretching or motion involved. See the
    // weights.xlsx file ("New--Stretch" sheet) for details.
    penalties.push(KeyPenalty {
        name: "stretch",
    });

    // 4. Penalize 0.1 points for using the same hand four times in a row.
    penalties.push(KeyPenalty {
        name: "same hand",
    });

    // 5. Penalize 20 points for reversing a roll at the end of the hand, i.e.
    // using the ring, pinky, then middle finger of the same hand, or the
    // middle, pinky, then ring of the same hand.
    penalties.push(KeyPenalty {
        name: "roll reversal",
    });

    // 6. Penalize 0.125 points for rolling outwards.
    penalties.push(KeyPenalty {
        name: "roll out",
    });

    // 7. Award 0.125 points for rolling inwards.
    penalties.push(KeyPenalty {
        name: "roll in",
    });

    // 8. Penalize 3 points for jumping from top to bottom row or from bottom
    // to top row on the same finger with a keystroke in between.
    penalties.push(KeyPenalty {
        name: "long jump sandwich",
    });

    // 9. Penalize 10 points for three consecutive keystrokes going up or down
    // the three rows of the keyboard in a roll.
    penalties.push(KeyPenalty {
        name: "twist",
    });

    // 10. Penalize 15 point for pinky/ring alternation on the same hand. For
    // example POP or SAS on Qwerty.
    penalties.push(KeyPenalty {
        name: "pinky/ring alternation",
    });

    // 11. Penalize a few points for repeatedly pressing the same key for
    // certain fingers. This is similar to the "same finger" penalty, but for
    // the same key. For example, double-tapping using pinkies is harder than
    // double-tapping using index fingers.
    penalties.push(KeyPenalty {
        name: "same key",
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

        // 3: Stretch.
        if curr.finger != old1.finger {
            let penalty = calculate_stretch_penalty(curr, old1);
            let penalty = penalty * count;
            if detailed && penalty > 0. {
                *result[3].high_keys.entry(slice2).or_insert(0.0) += penalty;
                result[3].total += penalty;
            }
            total += penalty;
        }

        // 6: Roll out.
        if is_roll_out(curr.finger, old1.finger) {
            let penalty = 0.125 * count;
            if detailed {
                *result[6].high_keys.entry(slice2).or_insert(0.0) += penalty;
                result[6].total += penalty;
            }
            total += penalty;
        }

        // 7: Roll in.
        if is_roll_in(curr.finger, old1.finger) {
            let penalty = -0.125 * count;
            if detailed {
                *result[7].high_keys.entry(slice2).or_insert(0.0) += penalty;
                result[7].total += penalty;
            }
            total += penalty;
        }

        // 11. Same key.
        if curr.pos == old1.pos {
            let penalty = calculate_same_key_penalty(curr, old1);
            let penalty = penalty * count;
            if detailed && penalty > 0. {
                *result[11].high_keys.entry(slice2).or_insert(0.0) += penalty;
                result[11].total += penalty;
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

        // 5: Roll reversal.
        if (curr.finger == Finger::Middle &&
            old1.finger == Finger::Pinky &&
            old2.finger == Finger::Ring) ||
           (curr.finger == Finger::Ring &&
            old1.finger == Finger::Pinky &&
            old2.finger == Finger::Middle) {
            let penalty = 20.0 * count;
            if detailed {
                *result[5].high_keys.entry(slice3).or_insert(0.0) += penalty;
                result[5].total += penalty;
            }
            total += penalty;
        }

        // 9: Twist.
        if ((curr.row == Row::Top && old1.row == Row::Home && old2.row == Row::Bottom) ||
            (curr.row == Row::Bottom && old1.row == Row::Home && old2.row == Row::Top)) &&
           ((is_roll_out(curr.finger, old1.finger) && is_roll_out(old1.finger, old2.finger)) ||
               (is_roll_in(curr.finger, old1.finger) && is_roll_in(old1.finger, old2.finger))) {
            let penalty = 10.0 * count;
            if detailed {
                *result[9].high_keys.entry(slice3).or_insert(0.0) += penalty;
                result[9].total += penalty;
            }
            total += penalty;
        }

        // 10: Pinky/ring alternation.
        if (curr.finger == Finger::Ring &&
            old1.finger == Finger::Pinky &&
            old2.finger == Finger::Ring) ||
           (curr.finger == Finger::Pinky &&
            old1.finger == Finger::Ring &&
            old2.finger == Finger::Pinky) {
            let penalty = 15.0 * count;
            if detailed {
                *result[10].high_keys.entry(slice3).or_insert(0.0) += penalty;
                result[10].total += penalty;
            }
            total += penalty;
        }
    }

    // 8: Long jump sandwich.
    if curr.hand == old2.hand && curr.finger == old2.finger {
        if curr.row == Row::Top && old2.row == Row::Bottom ||
           curr.row == Row::Bottom && old2.row == Row::Top {
            let slice3 = &string[(len - 3)..len];
            let penalty = 3.0 * count;
            if detailed {
                *result[8].high_keys.entry(slice3).or_insert(0.0) += penalty;
                result[8].total += penalty;
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
        // 4: Same hand.
        let slice4 = &string[(len - 4)..len];
        let penalty = 0.1 * count;
        if detailed {
            *result[4].high_keys.entry(slice4).or_insert(0.0) += penalty;
            result[4].total += penalty;
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

        // fg/gf/hj/jh
        if curr.pos == 14 && old1.pos == 15 ||
           curr.pos == 15 && old1.pos == 14 ||
           curr.pos == 16 && old1.pos == 17 ||
           curr.pos == 17 && old1.pos == 16 {
            return 0.;
        }
        // gr/rg/hu/uh
        if curr.pos == 15 && old1.pos == 3 ||
           curr.pos == 3 && old1.pos == 15 ||
           curr.pos == 16 && old1.pos == 6 ||
           curr.pos == 6 && old1.pos == 16 {
            return 0.;
        }
        // bf/fb/nj/jn
        if curr.pos == 26 && old1.pos == 14 ||
           curr.pos == 14 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 17 ||
           curr.pos == 17 && old1.pos == 27 {
            return 1.;
        }
        // rt/tr/yu/uy
        if curr.pos == 3 && old1.pos == 4 ||
           curr.pos == 4 && old1.pos == 3 ||
           curr.pos == 5 && old1.pos == 6 ||
           curr.pos == 6 && old1.pos == 5 {
            return 3.;
        }
        // vf/fv/mj/jm
        if curr.pos == 25 && old1.pos == 14 ||
           curr.pos == 14 && old1.pos == 25 ||
           curr.pos == 28 && old1.pos == 17 ||
           curr.pos == 17 && old1.pos == 28 {
            return 3.;
        }
        // fr/rf/ju/uj
        if curr.pos == 14 && old1.pos == 3 ||
           curr.pos == 3 && old1.pos == 14 ||
           curr.pos == 17 && old1.pos == 6 ||
           curr.pos == 6 && old1.pos == 17 {
            return 4.;
        }
        // br/rb/nu/un
        if curr.pos == 26 && old1.pos == 3 ||
           curr.pos == 3 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 6 ||
           curr.pos == 6 && old1.pos == 27 {
            return 6.;
        }
        // bv/vb/nm/mn
        if curr.pos == 26 && old1.pos == 25 ||
           curr.pos == 25 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 28 ||
           curr.pos == 28 && old1.pos == 27 {
            return 7.;
        }
        // vr/rv/mu/um
        if curr.pos == 25 && old1.pos == 3 ||
           curr.pos == 3 && old1.pos == 25 ||
           curr.pos == 28 && old1.pos == 6 ||
           curr.pos == 6 && old1.pos == 28 {
            return 8.;
        }
        // ft/tf/jy/yj
        if curr.pos == 14 && old1.pos == 4 ||
           curr.pos == 4 && old1.pos == 14 ||
           curr.pos == 17 && old1.pos == 5 ||
           curr.pos == 5 && old1.pos == 17 {
            return 10.;
        }
        // vg/gv/mh/hm
        if curr.pos == 25 && old1.pos == 15 ||
           curr.pos == 15 && old1.pos == 25 ||
           curr.pos == 28 && old1.pos == 16 ||
           curr.pos == 16 && old1.pos == 28 {
            return 10.;
        }
        // bg/gb/nh/hn
        if curr.pos == 26 && old1.pos == 15 ||
           curr.pos == 15 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 16 ||
           curr.pos == 16 && old1.pos == 27 {
            return 15.;
        }
        // gt/tg/hy/yh
        if curr.pos == 15 && old1.pos == 4 ||
           curr.pos == 4 && old1.pos == 15 ||
           curr.pos == 16 && old1.pos == 5 ||
           curr.pos == 5 && old1.pos == 16 {
            return 15.;
        }
        // bt/tb/ny/yn
        if curr.pos == 26 && old1.pos == 4 ||
           curr.pos == 4 && old1.pos == 26 ||
           curr.pos == 27 && old1.pos == 5 ||
           curr.pos == 5 && old1.pos == 27 {
            return 20.;
        }
        // vt/tv/my/ym
        if curr.pos == 25 && old1.pos == 4 ||
           curr.pos == 4 && old1.pos == 25 ||
           curr.pos == 28 && old1.pos == 5 ||
           curr.pos == 5 && old1.pos == 28 {
            return 25.;
        }

        assert!(false, "All index finger pairs must be covered by now");
    }

    assert!(!curr.center,
            "All center column key presses must be covered by now.");

    let long_jump = (curr.row == Row::Top && old1.row == Row::Bottom) ||
                    (curr.row == Row::Bottom && old1.row == Row::Top);

    // Long jumping is painful: 15 points; else 5 points.
    0.0 + if long_jump { 15.0 } else { 5.0 }
        + if curr.outer { 5.0 } else { 0.0 }
        + if old1.outer { 5.0 } else { 0.0 }
}

fn calculate_stretch_penalty(curr: &KeyPress, old1: &KeyPress)
-> f64 {
    // This penalty should only be calculated if we use different fingers on
    // the same hand.
    assert!(curr.hand == old1.hand);
    assert!(curr.finger != old1.finger);

    // In the following comments, all letter combinations are on Qwerty.

    // 1 point penalties.

    // ve ev mi im
    if curr.pos == 25 && old1.pos == 2 ||
       curr.pos == 2 && old1.pos == 25 ||
       curr.pos == 28 && old1.pos == 7 ||
       curr.pos == 7 && old1.pos == 28 {
        return 1.;
    }

    // vw wv mo om
    if curr.pos == 25 && old1.pos == 1 ||
       curr.pos == 1 && old1.pos == 25 ||
       curr.pos == 28 && old1.pos == 8 ||
       curr.pos == 8 && old1.pos == 28 {
        return 1.;
    }

    // ba ab n; ;n
    if curr.pos == 26 && old1.pos == 11 ||
       curr.pos == 11 && old1.pos == 26 ||
       curr.pos == 27 && old1.pos == 20 ||
       curr.pos == 20 && old1.pos == 27 {
        return 1.;
    }

    // gq qg hp ph
    if curr.pos == 15 && old1.pos == 0 ||
       curr.pos == 0 && old1.pos == 15 ||
       curr.pos == 16 && old1.pos == 9 ||
       curr.pos == 9 && old1.pos == 16 {
        return 1.;
    }

    // bz zb n/ /n
    if curr.pos == 26 && old1.pos == 22 ||
       curr.pos == 22 && old1.pos == 26 ||
       curr.pos == 27 && old1.pos == 31 ||
       curr.pos == 31 && old1.pos == 27 {
        return 1.;
    }

    // ga ag h; ;h
    if curr.pos == 15 && old1.pos == 11 ||
       curr.pos == 11 && old1.pos == 15 ||
       curr.pos == 16 && old1.pos == 20 ||
       curr.pos == 20 && old1.pos == 16 {
        return 1.;
    }

    // tq qt yp py
    if curr.pos == 4 && old1.pos == 0 ||
       curr.pos == 0 && old1.pos == 4 ||
       curr.pos == 5 && old1.pos == 9 ||
       curr.pos == 9 && old1.pos == 5 {
        return 1.;
    }

    // i' 'i
    if curr.pos == 7 && old1.pos == 21 ||
       curr.pos == 21 && old1.pos == 7 {
        return 1.;
    }

    // u' 'u
    if curr.pos == 6 && old1.pos == 21 ||
       curr.pos == 21 && old1.pos == 6 {
        return 1.;
    }

    // ta at y; ;y
    if curr.pos == 4 && old1.pos == 11 ||
       curr.pos == 11 && old1.pos == 4 ||
       curr.pos == 5 && old1.pos == 20 ||
       curr.pos == 20 && old1.pos == 5 {
        return 1.;
    }

    // gz zg h/ /h
    if curr.pos == 15 && old1.pos == 22 ||
       curr.pos == 22 && old1.pos == 15 ||
       curr.pos == 16 && old1.pos == 31 ||
       curr.pos == 31 && old1.pos == 16 {
        return 1.;
    }

    // bs sb nl ln
    if curr.pos == 26 && old1.pos == 12 ||
       curr.pos == 12 && old1.pos == 26 ||
       curr.pos == 27 && old1.pos == 19 ||
       curr.pos == 19 && old1.pos == 27 {
        return 1.;
    }

    // gw wg ho oh
    if curr.pos == 15 && old1.pos == 1 ||
       curr.pos == 1 && old1.pos == 15 ||
       curr.pos == 16 && old1.pos == 8 ||
       curr.pos == 8 && old1.pos == 16 {
        return 1.;
    }

    // bx xb n. .n
    if curr.pos == 26 && old1.pos == 23 ||
       curr.pos == 23 && old1.pos == 26 ||
       curr.pos == 27 && old1.pos == 30 ||
       curr.pos == 30 && old1.pos == 27 {
        return 1.;
    }

    // gs sg hl lh
    if curr.pos == 15 && old1.pos == 12 ||
       curr.pos == 12 && old1.pos == 15 ||
       curr.pos == 16 && old1.pos == 19 ||
       curr.pos == 19 && old1.pos == 16 {
        return 1.;
    }

    // tw wt yo oy
    if curr.pos == 4 && old1.pos == 1 ||
       curr.pos == 1 && old1.pos == 4 ||
       curr.pos == 5 && old1.pos == 8 ||
       curr.pos == 8 && old1.pos == 5 {
        return 1.;
    }

    // j\ \j
    if curr.pos == 17 && old1.pos == 10 ||
       curr.pos == 10 && old1.pos == 17 {
        return 1.;
    }

    // m' 'm
    if curr.pos == 28 && old1.pos == 21 ||
       curr.pos == 21 && old1.pos == 28 {
        return 1.;
    }

    // o' 'o
    if curr.pos == 8 && old1.pos == 21 ||
       curr.pos == 21 && old1.pos == 8 {
        return 1.;
    }

    // 2 point penalties.

    // ez ze i/ /i
    if curr.pos == 2 && old1.pos == 22 ||
       curr.pos == 22 && old1.pos == 2 ||
       curr.pos == 7 && old1.pos == 31 ||
       curr.pos == 31 && old1.pos == 7 {
        return 2.;
    }

    // rz zr u/ /u
    if curr.pos == 3 && old1.pos == 22 ||
       curr.pos == 22 && old1.pos == 3 ||
       curr.pos == 6 && old1.pos == 31 ||
       curr.pos == 31 && old1.pos == 6 {
        return 2.;
    }

    // bd db nk kn
    if curr.pos == 26 && old1.pos == 13 ||
       curr.pos == 13 && old1.pos == 26 ||
       curr.pos == 27 && old1.pos == 18 ||
       curr.pos == 18 && old1.pos == 27 {
        return 2.;
    }

    // ge eg hi ih
    if curr.pos == 15 && old1.pos == 2 ||
       curr.pos == 2 && old1.pos == 15 ||
       curr.pos == 16 && old1.pos == 7 ||
       curr.pos == 7 && old1.pos == 16 {
        return 2.;
    }

    // bc cb n, ,n
    if curr.pos == 26 && old1.pos == 24 ||
       curr.pos == 24 && old1.pos == 26 ||
       curr.pos == 27 && old1.pos == 29 ||
       curr.pos == 29 && old1.pos == 27 {
        return 2.;
    }

    // gd dg hk kh
    if curr.pos == 15 && old1.pos == 13 ||
       curr.pos == 13 && old1.pos == 15 ||
       curr.pos == 16 && old1.pos == 18 ||
       curr.pos == 18 && old1.pos == 16 {
        return 2.;
    }

    // te et yi iy
    if curr.pos == 4 && old1.pos == 2 ||
       curr.pos == 2 && old1.pos == 4 ||
       curr.pos == 5 && old1.pos == 7 ||
       curr.pos == 7 && old1.pos == 5 {
        return 2.;
    }

    // xa ax .; ;.
    if curr.pos == 23 && old1.pos == 11 ||
       curr.pos == 11 && old1.pos == 23 ||
       curr.pos == 30 && old1.pos == 20 ||
       curr.pos == 20 && old1.pos == 30 {
        return 2.;
    }

    // sq qs lp pl
    if curr.pos == 12 && old1.pos == 0 ||
       curr.pos == 0 && old1.pos == 12 ||
       curr.pos == 19 && old1.pos == 9 ||
       curr.pos == 9 && old1.pos == 19 {
        return 2.;
    }

    // vq qv mp pm
    if curr.pos == 25 && old1.pos == 0 ||
       curr.pos == 0 && old1.pos == 25 ||
       curr.pos == 28 && old1.pos == 9 ||
       curr.pos == 9 && old1.pos == 28 {
        return 2.;
    }

    // 3 point penalties.

    // bw wb no on
    if curr.pos == 26 && old1.pos == 1 ||
       curr.pos == 1 && old1.pos == 26 ||
       curr.pos == 27 && old1.pos == 8 ||
       curr.pos == 8 && old1.pos == 27 {
        return 3.;
    }

    // gx xg h. .h
    if curr.pos == 15 && old1.pos == 23 ||
       curr.pos == 23 && old1.pos == 15 ||
       curr.pos == 16 && old1.pos == 30 ||
       curr.pos == 30 && old1.pos == 16 {
        return 3.;
    }

    // ts st yl ly
    if curr.pos == 4 && old1.pos == 12 ||
       curr.pos == 12 && old1.pos == 4 ||
       curr.pos == 5 && old1.pos == 19 ||
       curr.pos == 19 && old1.pos == 5 {
        return 3.;
    }

    // rx xr u. .u
    if curr.pos == 3 && old1.pos == 23 ||
       curr.pos == 23 && old1.pos == 3 ||
       curr.pos == 6 && old1.pos == 30 ||
       curr.pos == 30 && old1.pos == 6 {
        return 3.;
    }

    // m\ \m
    if curr.pos == 28 && old1.pos == 10 ||
       curr.pos == 10 && old1.pos == 28 {
        return 3.;
    }

    // bq qb np pn
    if curr.pos == 26 && old1.pos == 0 ||
       curr.pos == 0 && old1.pos == 26 ||
       curr.pos == 27 && old1.pos == 9 ||
       curr.pos == 9 && old1.pos == 27 {
        return 3.;
    }

    // k\ \k
    if curr.pos == 18 && old1.pos == 10 ||
       curr.pos == 10 && old1.pos == 18 {
        return 3.;
    }

    // ,' ',
    if curr.pos == 29 && old1.pos == 21 ||
       curr.pos == 21 && old1.pos == 29 {
        return 3.;
    }

    // .' '.
    if curr.pos == 30 && old1.pos == 21 ||
       curr.pos == 21 && old1.pos == 30 {
        return 3.;
    }

    // l\ \l
    if curr.pos == 19 && old1.pos == 10 ||
       curr.pos == 10 && old1.pos == 19 {
        return 3.;
    }

    // y\ \y
    if curr.pos == 5 && old1.pos == 10 ||
       curr.pos == 10 && old1.pos == 5 {
        return 3.;
    }

    // h' 'h
    if curr.pos == 16 && old1.pos == 21 ||
       curr.pos == 21 && old1.pos == 16 {
        return 3.;
    }

    // 4 point penalties.

    // tz zt y/ /y
    if curr.pos == 4 && old1.pos == 22 ||
       curr.pos == 22 && old1.pos == 4 ||
       curr.pos == 5 && old1.pos == 31 ||
       curr.pos == 31 && old1.pos == 5 {
        return 4.;
    }

    // td dt yk ky
    if curr.pos == 4 && old1.pos == 13 ||
       curr.pos == 13 && old1.pos == 4 ||
       curr.pos == 5 && old1.pos == 18 ||
       curr.pos == 18 && old1.pos == 5 {
        return 4.;
    }

    // gc cg h, ,h
    if curr.pos == 15 && old1.pos == 24 ||
       curr.pos == 24 && old1.pos == 15 ||
       curr.pos == 16 && old1.pos == 29 ||
       curr.pos == 29 && old1.pos == 16 {
        return 4.;
    }

    // ex xe i. .i
    if curr.pos == 2 && old1.pos == 23 ||
       curr.pos == 23 && old1.pos == 2 ||
       curr.pos == 7 && old1.pos == 30 ||
       curr.pos == 30 && old1.pos == 7 {
        return 4.;
    }

    // rc cr u, ,u
    if curr.pos == 3 && old1.pos == 24 ||
       curr.pos == 24 && old1.pos == 3 ||
       curr.pos == 6 && old1.pos == 29 ||
       curr.pos == 29 && old1.pos == 6 {
        return 4.;
    }

    // cw wc ,o o,
    if curr.pos == 24 && old1.pos == 1 ||
       curr.pos == 1 && old1.pos == 24 ||
       curr.pos == 29 && old1.pos == 8 ||
       curr.pos == 8 && old1.pos == 29 {
        return 4.;
    }

    // n' 'n
    if curr.pos == 27 && old1.pos == 21 ||
       curr.pos == 21 && old1.pos == 27 {
        return 4.;
    }

    // h\ \h
    if curr.pos == 16 && old1.pos == 10 ||
       curr.pos == 10 && old1.pos == 16 {
        return 4.;
    }

    // 5 point penalties.

    // y' 'y
    if curr.pos == 5 && old1.pos == 21 ||
       curr.pos == 21 && old1.pos == 5 {
        return 5.;
    }

    // tx xt y. .y
    if curr.pos == 4 && old1.pos == 23 ||
       curr.pos == 23 && old1.pos == 4 ||
       curr.pos == 5 && old1.pos == 30 ||
       curr.pos == 30 && old1.pos == 5 {
        return 5.
    }

    // 6 point penalties.

    // tc ct y, ,y
    if curr.pos == 4 && old1.pos == 24 ||
       curr.pos == 24 && old1.pos == 4 ||
       curr.pos == 5 && old1.pos == 29 ||
       curr.pos == 29 && old1.pos == 5 {
        return 6.;
    }

    // 7 point penalties.

    // be eb ni in
    if curr.pos == 26 && old1.pos == 2 ||
       curr.pos == 2 && old1.pos == 26 ||
       curr.pos == 27 && old1.pos == 7 ||
       curr.pos == 7 && old1.pos == 27 {
        return 7.;
    }

    // cq qc ,p p,
    if curr.pos == 24 && old1.pos == 0 ||
       curr.pos == 0 && old1.pos == 24 ||
       curr.pos == 29 && old1.pos == 9 ||
       curr.pos == 9 && old1.pos == 29 {
        return 7.;
    }

    // 8 point penalties.

    // n\ \n
    if curr.pos == 27 && old1.pos == 10 ||
       curr.pos == 10 && old1.pos == 27 {
        return 8.;
    }

    // 9 point penalties.

    // xq qx .p p.
    if curr.pos == 23 && old1.pos == 0 ||
       curr.pos == 0 && old1.pos == 23 ||
       curr.pos == 30 && old1.pos == 9 ||
       curr.pos == 9 && old1.pos == 30 {
        return 9.;
    }

    // .\ \.
    if curr.pos == 30 && old1.pos == 10 ||
       curr.pos == 10 && old1.pos == 30 {
        return 9.;
    }

    // ,\ \,
    if curr.pos == 29 && old1.pos == 10 ||
       curr.pos == 10 && old1.pos == 29 {
        return 9.;
    }

    // 10 point penalties.

    // wz zw o/ /o
    if curr.pos == 1 && old1.pos == 22 ||
       curr.pos == 22 && old1.pos == 1 ||
       curr.pos == 8 && old1.pos == 31 ||
       curr.pos == 31 && old1.pos == 8 {
        return 10.;
    }

    0.
}

fn calculate_same_key_penalty(curr: &KeyPress, old1: &KeyPress)
-> f64 {

    // This penalty should only be calculated if we consecutively use the same
    // key.
    assert!(curr.hand == old1.hand);
    assert!(curr.finger == old1.finger);
    assert!(curr.pos == old1.pos);

    match curr.finger {
        Finger::Pinky  => 3.0,
        _ => 0.
    }
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
