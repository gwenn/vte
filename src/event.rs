/// Reading user input
use crate::{Params, Perform};

/// User input handler
pub trait Handler {
    fn ss3_dispatch(&mut self, _param: u16, _c: char) {}
    /// printable character pressed
    fn print(&mut self, _c: char) {}
    /// control character pressed (Tab / Ctrl-I, Enter / Ctrl-M, Backspace /
    /// Ctrl-H, ...)
    fn execute(&mut self, _b: u8) {}
    /// Alt + character pressed, BackTab / Shift-Tab
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _b: u8) {}
    fn csi_dispatch(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn dcs_hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn dcs_put(&mut self, _byte: u8) {}
    fn dcs_unhook(&mut self) {}
}

bitflags::bitflags! {
    /// The set of modifier keys that were triggered along with a key press.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct Modifiers: u8 {
        /// Control modifier
        const CTRL  = 1<<3;
        /// Escape or Alt modifier (Meta/Opt)
        const ALT  = 1<<2;
        /// Shift modifier
        const SHIFT = 1<<1;
        /// Super modifier (Cmd/Win)
        const SUPER = 1<<4;

        /// No modifier
        const NONE = 0;
        /// Ctrl + Shift
        const CTRL_SHIFT = Self::CTRL.bits() | Self::SHIFT.bits();
        /// Alt + Shift
        const ALT_SHIFT = Self::ALT.bits() | Self::SHIFT.bits();
        /// Ctrl + Alt
        const CTRL_ALT = Self::CTRL.bits() | Self::ALT.bits();
        /// Ctrl + Alt + Shift
        const CTRL_ALT_SHIFT = Self::CTRL.bits() | Self::ALT.bits() | Self::SHIFT.bits();
    }
}

/// Input key pressed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KeyCode {
    /// Unsupported escape sequence (on unix platform)
    UnknownEscSeq,
    /// ⌫ or Ctrl-H
    Backspace,
    /// ⇤ (usually Shift-Tab)
    BackTab,
    /// Paste (on unix platform)
    BracketedPasteStart,
    /// Paste (on unix platform)
    BracketedPasteEnd,
    /// Single char
    Char(char),
    /// ⌦
    Delete,
    /// ↓ arrow key
    Down,
    /// ⇲
    End,
    /// ↵ or Ctrl-M
    Enter,
    /// Escape or Ctrl-[
    Esc,
    /// Function key
    F(u8),
    /// ⇱
    Home,
    /// Insert key
    Insert,
    /// ← arrow key
    Left,
    /// \0
    Null,
    /// ⇟
    PageDown,
    /// ⇞
    PageUp,
    /// → arrow key
    Right,
    /// ⇥ or Ctrl-I
    Tab,
    /// ↑ arrow key
    Up,
    // TODO CapsLock,Menu,NumLock,Pause,PrintScreen,ScrollLock
}

pub struct EventHandler {
    pub key: KeyCode,
    pub modifiers: Modifiers,
    // TODO state: keypad | num_lock | caps_lock
}

// TODO
// - Alt-O (\x1BO)
// - Alt-P (\x1BP)
// - ESC ESC... => Alt- ...
impl Handler for EventHandler {
    fn ss3_dispatch(&mut self, param: u16, c: char) {
        let ss3_m = |param: u16| match param {
            5 => Modifiers::SHIFT,
            _ => Modifiers::NONE,
        };
        let ss3_f = |param: u16| match param {
            0 => 0,
            2 | 12 => 12,
            3 | 13 => 48,
            4 | 14 => 60,
            5 | 15 => 24,
            6 | 16 => 36,
            _ => 0,
        };
        match c {
            'A' => self.key = KeyCode::Up,
            'B' => self.key = KeyCode::Down,
            'C' => (self.key, self.modifiers) = (KeyCode::Right, ss3_m(param)),
            'D' => (self.key, self.modifiers) = (KeyCode::Left, ss3_m(param)),
            //'E' => self.key = KeyCode::Home, // kbeg,kb2
            'F' => self.key = KeyCode::End,
            'M' => self.key = KeyCode::Enter,
            'H' => self.key = KeyCode::Home,
            'P' => self.key = KeyCode::F(1 + ss3_f(param)),
            'Q' => self.key = KeyCode::F(2 + ss3_f(param)),
            'R' => self.key = KeyCode::F(3 + ss3_f(param)),
            'S' => self.key = KeyCode::F(4 + ss3_f(param)),
            'T' | 't' => self.key = KeyCode::F(5),
            'U' => self.key = KeyCode::F(6),
            'V' | 'v' => self.key = KeyCode::F(7),
            'W' | 'l' => self.key = KeyCode::F(8),
            'X' => self.key = KeyCode::F(9),
            'Y' | 'x' => self.key = KeyCode::F(10),
            'a' => (self.key, self.modifiers) = (KeyCode::Up, Modifiers::CTRL),
            'b' => (self.key, self.modifiers) = (KeyCode::Down, Modifiers::CTRL),
            'c' => (self.key, self.modifiers) = (KeyCode::Right, Modifiers::CTRL),
            'd' => (self.key, self.modifiers) = (KeyCode::Left, Modifiers::CTRL),
            'n' => self.key = KeyCode::Delete, // kc3
            'p' => self.key = KeyCode::Insert, // kc1
            'q' => self.key = KeyCode::End,    // ka1,kc1*
            //'r' => self.key = KeyCode::Char('5'), // kb2
            's' => self.key = KeyCode::PageDown, // ka3,kc3*
            //'u' => self.key = KeyCode::, // kbeg,kb2*,kf6
            'w' => self.key = KeyCode::Home,   // ka1*,kf9
            'y' => self.key = KeyCode::PageUp, // ka3*,kf0
            _ => {},
        };
    }

    fn print(&mut self, c: char) {
        self.key = KeyCode::Char(c)
    }

    fn execute(&mut self, b: u8) {
        (self.key, self.modifiers) = match b {
            b'\x00' => (KeyCode::Char('@'), Modifiers::CTRL), // '\0'
            b'\x01' => (KeyCode::Char('A'), Modifiers::CTRL),
            b'\x02' => (KeyCode::Char('B'), Modifiers::CTRL),
            b'\x03' => (KeyCode::Char('C'), Modifiers::CTRL),
            b'\x04' => (KeyCode::Char('D'), Modifiers::CTRL),
            b'\x05' => (KeyCode::Char('E'), Modifiers::CTRL),
            b'\x06' => (KeyCode::Char('F'), Modifiers::CTRL),
            b'\x07' => (KeyCode::Char('G'), Modifiers::CTRL), // '\a',bel
            b'\x08' => (KeyCode::Backspace, Modifiers::NONE), // '\b',kbs
            b'\x09' => (KeyCode::Tab, Modifiers::NONE),       // '\t',ht
            b'\x0a' => (KeyCode::Char('J'), Modifiers::CTRL), // '\n' (10)
            b'\x0b' => (KeyCode::Char('K'), Modifiers::CTRL),
            b'\x0c' => (KeyCode::Char('L'), Modifiers::CTRL), // clear
            b'\x0d' => (KeyCode::Enter, Modifiers::NONE),     // '\r' (13),cr
            b'\x0e' => (KeyCode::Char('N'), Modifiers::CTRL),
            b'\x0f' => (KeyCode::Char('O'), Modifiers::CTRL),
            b'\x10' => (KeyCode::Char('P'), Modifiers::CTRL),
            b'\x11' => (KeyCode::Char('Q'), Modifiers::CTRL),
            b'\x12' => (KeyCode::Char('R'), Modifiers::CTRL),
            b'\x13' => (KeyCode::Char('S'), Modifiers::CTRL),
            b'\x14' => (KeyCode::Char('T'), Modifiers::CTRL),
            b'\x15' => (KeyCode::Char('U'), Modifiers::CTRL),
            b'\x16' => (KeyCode::Char('V'), Modifiers::CTRL),
            b'\x17' => (KeyCode::Char('W'), Modifiers::CTRL),
            b'\x18' => (KeyCode::Char('X'), Modifiers::CTRL),
            b'\x19' => (KeyCode::Char('Y'), Modifiers::CTRL),
            b'\x1a' => (KeyCode::Char('Z'), Modifiers::CTRL), // kspd
            b'\x1b' => (KeyCode::Esc, Modifiers::NONE),       // Ctrl-[, '\e'
            b'\x1c' => (KeyCode::Char('\\'), Modifiers::CTRL),
            b'\x1d' => (KeyCode::Char(']'), Modifiers::CTRL),
            b'\x1e' => (KeyCode::Char('^'), Modifiers::CTRL),
            b'\x1f' => (KeyCode::Char('_'), Modifiers::CTRL),
            b'\x7f' => (KeyCode::Backspace, Modifiers::NONE), // Rubout, Ctrl-?,kbs
            _ => (KeyCode::Null, Modifiers::NONE),
        };
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, b: u8) {
        if b.is_ascii_control() {
            self.execute(b);
        } else if b.is_ascii_alphanumeric() {
            self.key = KeyCode::Char(b as char);
        }
        self.modifiers |= Modifiers::ALT;
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, c: char) {
        fn csi_m(params: &Params) -> Modifiers {
            let mut params_iter = params.iter();
            let mut next_param_or = |default: u16| match params_iter.next() {
                Some(&[param, ..]) if param != 0 => param,
                _ => default,
            };
            let _ = next_param_or(1);
            match next_param_or(1) {
                2 => Modifiers::SHIFT,
                3 => Modifiers::ALT,
                4 => Modifiers::ALT_SHIFT,
                5 => Modifiers::CTRL,
                6 => Modifiers::CTRL_SHIFT,
                7 => Modifiers::CTRL_ALT,
                8 => Modifiers::CTRL_ALT_SHIFT,
                _ => Modifiers::NONE,
            }
        }
        fn csi_f(params: &Params) -> u8 {
            let mut params_iter = params.iter();
            let mut next_param_or = |default: u16| match params_iter.next() {
                Some(&[param, ..]) if param != 0 => param,
                _ => default,
            };
            let _ = next_param_or(1);
            match next_param_or(1) {
                2 => 12,
                5 => 24,
                6 => 36,
                3 => 48,
                4 => 60,
                _ => 3,
            }
        }
        match c {
            'A' => {
                if intermediates == &[b'['] {
                    (self.key, self.modifiers) = (KeyCode::F(1), Modifiers::NONE)
                } else {
                    (self.key, self.modifiers) = (KeyCode::Up, csi_m(params)) // kcuu1,kri
                }
            },
            'B' => {
                if intermediates == &[b'['] {
                    (self.key, self.modifiers) = (KeyCode::F(2), Modifiers::NONE)
                } else {
                    (self.key, self.modifiers) = (KeyCode::Down, csi_m(params)) // kcud1,kind
                }
            },
            'C' => {
                if intermediates == &[b'['] {
                    (self.key, self.modifiers) = (KeyCode::F(3), Modifiers::NONE)
                } else {
                    (self.key, self.modifiers) = (KeyCode::Right, csi_m(params))
                    // kcuf1,kRIT
                }
            },
            'D' => {
                if intermediates == &[b'['] {
                    (self.key, self.modifiers) = (KeyCode::F(4), Modifiers::NONE)
                } else {
                    (self.key, self.modifiers) = (KeyCode::Left, csi_m(params)) // kcub1,kLFT
                }
            },
            'E' => {
                if intermediates == &[b'['] {
                    (self.key, self.modifiers) = (KeyCode::F(5), Modifiers::NONE)
                } else {
                    (self.key, self.modifiers) = (KeyCode::Home, csi_m(params)) // kb2,kbeg,kBEG
                }
            },
            'F' => (self.key, self.modifiers) = (KeyCode::End, csi_m(params)), // kend,kEND
            'H' => (self.key, self.modifiers) = (KeyCode::Home, csi_m(params)), // khome,kHOM
            'I' => (self.key, self.modifiers) = (KeyCode::PageUp, Modifiers::NONE), // kpp
            'L' => (self.key, self.modifiers) = (KeyCode::Insert, Modifiers::NONE), // kich1
            'M' => (self.key, self.modifiers) = (KeyCode::F(1), Modifiers::NONE), // kf1
            'N' => (self.key, self.modifiers) = (KeyCode::F(2), Modifiers::NONE), // kf2
            'O' => (self.key, self.modifiers) = (KeyCode::F(3), Modifiers::NONE), // kf3
            'P' => (self.key, self.modifiers) = (KeyCode::F(1 + csi_f(params)), Modifiers::NONE), /* kf4 */
            'Q' => (self.key, self.modifiers) = (KeyCode::F(2 + csi_f(params)), Modifiers::NONE), /* kf5 */
            'R' => (self.key, self.modifiers) = (KeyCode::F(3 + csi_f(params)), Modifiers::NONE), /* kf6 */
            'S' => (self.key, self.modifiers) = (KeyCode::F(4 + csi_f(params)), Modifiers::NONE), /* kf7 */
            'T' => (self.key, self.modifiers) = (KeyCode::F(8), Modifiers::NONE), // kf8
            'U' => (self.key, self.modifiers) = (KeyCode::PageDown, Modifiers::NONE), // kf9,knp
            'V' => (self.key, self.modifiers) = (KeyCode::PageUp, Modifiers::NONE), // kf10,kpp
            'W' => (self.key, self.modifiers) = (KeyCode::F(11), Modifiers::NONE), // kf11
            'X' => (self.key, self.modifiers) = (KeyCode::F(12), Modifiers::NONE), // kf12
            'Y' => (self.key, self.modifiers) = (KeyCode::End, Modifiers::NONE),  // kend,kf13
            'Z' => (self.key, self.modifiers) = (KeyCode::BackTab, Modifiers::NONE), // kcbt,kf14
            'a' => (self.key, self.modifiers) = (KeyCode::Up, Modifiers::SHIFT),  // kind,kf15
            'b' => (self.key, self.modifiers) = (KeyCode::Down, Modifiers::SHIFT), // kri,kf16
            'c' if intermediates.is_empty() => {
                (self.key, self.modifiers) = (KeyCode::Right, Modifiers::SHIFT)
            }, // kRIT,kf17
            'd' => (self.key, self.modifiers) = (KeyCode::Left, Modifiers::SHIFT), // kLFT,kf18
            _ => {},
        };
    }
}

#[cfg(feature = "log")]
pub struct Log;
#[cfg(feature = "log")]
fn show(bs: &[u8]) -> String {
    let mut visible = String::new();
    for &b in bs {
        let part: Vec<u8> = std::ascii::escape_default(b).collect();
        visible.push_str(str::from_utf8(&part).unwrap());
    }
    visible
}
#[cfg(feature = "log")]
impl Handler for Log {
    fn ss3_dispatch(&mut self, param: u16, c: char) {
        log::info!(target: "vte", "[ss3] param={param} c={c}");
    }

    fn print(&mut self, c: char) {
        log::info!(target: "vte", "[print] c={c}");
    }

    fn execute(&mut self, b: u8) {
        log::info!(target: "vte", "[execute] b={}", std::ascii::escape_default(b));
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, b: u8) {
        log::info!(target: "vte", "[esc_dispatch] intermediates={}, ignore={ignore}, b={}", show(intermediates), std::ascii::escape_default(b));
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        log::info!(target: "vte", "[csi_dispatch] params={params:?} intermediates={}, ignore={ignore}, c={}", show(intermediates), c);
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        log::info!(target: "vte", "[osc_dispatch] params={:?} bell_terminated={bell_terminated}", params.iter().map(|s| show(s)).collect::<Vec<String>>());
    }

    fn dcs_hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        log::info!(target: "vte", "[dcs_hook] params={params:?} intermediates={}, ignore={ignore}, action={}", show(intermediates), action);
    }

    fn dcs_put(&mut self, byte: u8) {
        log::info!(target: "vte", "[dcs_put] byte={}", std::ascii::escape_default(byte));
    }

    fn dcs_unhook(&mut self) {
        log::info!(target: "vte", "[dcs_unhook]");
    }
}

/// Adapter from `crate::Perform` to `Handler`
pub struct Performer<'h, H: Handler> {
    handler: &'h mut H,
    osc_dispatch: bool,
    dcs_unhook: bool,
    terminated: bool,
}

pub fn new<H: Handler>(h: &mut H) -> Performer<'_, H> {
    Performer { handler: h, osc_dispatch: false, dcs_unhook: false, terminated: false }
}

impl<H: Handler> Performer<'_, H> {
    pub fn reset(&mut self) -> bool {
        if self.terminated {
            self.terminated = false;
            true
        } else {
            false
        }
    }
}

impl<'h, H: Handler> Perform for Performer<'h, H> {
    fn print(&mut self, c: char) {
        if c == '\x7F' {
            self.handler.execute(c as u8);
        } else {
            self.handler.print(c);
        }
        self.terminated = true;
    }

    fn execute(&mut self, b: u8) {
        self.handler.execute(b);
        self.terminated = true;
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        self.handler.csi_dispatch(params, intermediates, ignore, c);
        self.terminated = true;
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, b: u8) {
        if self.osc_dispatch && b == b'\\' {
            self.osc_dispatch = false;
        } else if self.dcs_unhook && b == b'\\' {
            self.dcs_unhook = false;
        } else {
            self.handler.esc_dispatch(intermediates, ignore, b);
        }
        self.terminated = true;
    }

    fn ss3_dispatch(&mut self, param: u16, c: char) {
        self.handler.ss3_dispatch(param, c);
        self.terminated = true;
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        if bell_terminated {
            self.terminated = true;
        } else {
            self.osc_dispatch = true;
        }
        self.handler.osc_dispatch(params, bell_terminated);
    }

    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        self.handler.dcs_hook(params, intermediates, ignore, action)
    }

    fn put(&mut self, byte: u8) {
        self.handler.dcs_put(byte)
    }

    fn unhook(&mut self) {
        self.dcs_unhook = true;
        self.handler.dcs_unhook();
    }

    fn terminated(&self) -> bool {
        if self.terminated {
            debug_assert!(!self.osc_dispatch);
            debug_assert!(!self.dcs_unhook);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Params, Parser};

    fn parse<H: Handler>(h: &mut H, bytes: &[u8]) {
        let mut x = new(h);
        let mut p = Parser::new();
        let n = p.advance_until_terminated(&mut x, bytes);
        assert_eq!(n, bytes.len());
        assert!(p.is_ground());
        assert!(x.reset());
    }

    #[test]
    fn test_ss3() {
        struct H(char);
        impl Handler for H {
            fn ss3_dispatch(&mut self, _param: u16, c: char) {
                self.0 = c;
            }
        }
        let mut h = H('\0');
        // F1 on Mac / Windows terminal with ENABLE_VIRTUAL_TERMINAL_INPUT
        parse(&mut h, b"\x1BOA");
        assert_eq!('A', h.0);
        // Alt-O
        // parse(&mut h, b"\x1BO"); // FIXME
        // [ss3] param=1 c=A
        parse(&mut h, b"\x1BO1A");
    }

    #[test]
    fn test_backspace() {
        struct H(u8);
        impl Handler for H {
            fn execute(&mut self, b: u8) {
                self.0 = b;
            }
        }
        let mut h = H(0);
        // Mac / Windows terminal
        parse(&mut h, b"\x7F");
        assert_eq!(0x7F, h.0);
    }

    #[test]
    fn test_csi_bracket() {
        struct H(char);
        impl Handler for H {
            fn csi_dispatch(
                &mut self,
                params: &Params,
                intermediates: &[u8],
                ignore: bool,
                c: char,
            ) {
                assert_eq!(params.iter().next(), Some(&[0u16][..]));
                assert_eq!(intermediates, b"[");
                assert!(!ignore);
                self.0 = c;
            }
        }
        let mut h = H('\0');
        // F1 on Linux console
        parse(&mut h, b"\x1B[[A");
        assert_eq!('A', h.0);
    }

    #[test]
    fn test_alt_enter() {
        struct H(u8);
        impl Handler for H {
            fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, b: u8) {
                assert!(intermediates.is_empty());
                assert!(!ignore);
                self.0 = b;
            }
        }
        let mut h = H(0);
        // Mac / Linux / Windows
        parse(&mut h, b"\x1B\x0D");
        assert_eq!(0x0D, h.0);
    }

    #[test]
    fn test_alt_backspace() {
        struct H(u8);
        impl Handler for H {
            fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, b: u8) {
                assert!(intermediates.is_empty());
                assert!(!ignore);
                self.0 = b;
            }
        }
        let mut h = H(0);
        parse(&mut h, b"\x1B\x7F");
        assert_eq!(0x7F, h.0);
    }

    #[test]
    fn test_shift_tab() {
        struct H(u8);
        impl Handler for H {
            fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, b: u8) {
                assert!(intermediates.is_empty());
                assert!(!ignore);
                self.0 = b;
            }
        }
        let mut h = H(0);
        // Mac / Linux console
        parse(&mut h, b"\x1B\x09");
        assert_eq!(0x09, h.0);
    }

    #[test]
    fn csi() {
        // env_logger::init();
        let mut h = super::Log;
        // Shift-Space
        // [csi_dispatch] params=[32;2] intermediates=, ignore=false, c=u
        parse(&mut h, b"\x1B[32;2u");
        // [csi_dispatch] params=[97;1:3] intermediates=, ignore=false, c=u
        parse(&mut h, b"\x1b[97;1:3u");
        // Mouse
        // [csi_dispatch] params=[35;1;1] intermediates=<, ignore=false, c=m
        parse(&mut h, b"\x1B[<35;1;1m");
        // DSR
        // [csi_dispatch] params=[997;1] intermediates=?, ignore=false, c=n
        parse(&mut h, b"\x1B[?997;1n");
        // [csi_dispatch] params=[0] intermediates=, ignore=false, c=n
        parse(&mut h, b"\x1B[0n");
        // FIXME [execute] b=\x1e
        // parse(&mut h, b"\x1B[11\x1E");
        // [csi_dispatch] params=[3] intermediates=, ignore=false, c=$ (rxvt)
        parse(&mut h, b"\x1B[3$");
        // [csi_dispatch] params=[0] intermediates=, ignore=false, c=A
        parse(&mut h, b"\x1B[A");
        // [csi_dispatch] params=[1;5] intermediates=, ignore=false, c=p
        parse(&mut h, b"\x1B[1;5p");
        // [csi_dispatch] params=[2] intermediates=, ignore=false, c=~
        parse(&mut h, b"\x1B[2~");
        // [csi_dispatch] params=[2;2] intermediates=, ignore=false, c=~
        parse(&mut h, b"\x1B[2;2~");
        // [csi_dispatch] params=[200] intermediates=, ignore=false, c=~
        parse(&mut h, b"\x1B[200~");
    }

    #[test]
    fn osc() {
        // env_logger::init();
        let mut h = super::Log;
        parse(&mut h, b"\x1B]4;2;#ff0000\x1B\\");
        parse(&mut h, b"\x1B]4;-2;rgb:[red]/[green]/[blue]\x1B\\");
        parse(&mut h, b"\x1B]10;#ff0000\x1B\\");
        parse(&mut h, b"\x1B]52;c;b3NjNTIgcGFzdGU=\x1B\\");
        parse(&mut h, b"\x1B]52;c;\x07");
    }

    // RUST_LOG=vte=info cargo test -- --nocapture event::tests::dcs
    #[test]
    fn dcs() {
        // env_logger::init();
        let mut h = super::Log;
        // XTGETTCAP
        parse(&mut h, b"\x1BP+q544e;524742;687061\x1B\\");
        // XTVERSION
        parse(&mut h, b"\x1BP>|iTerm2[version]\x1B\\");
    }
}
