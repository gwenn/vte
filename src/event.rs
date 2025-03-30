/// Reading user input
use crate::{Params, Perform};

/// User input handler
pub trait Handler {
    fn ss3(&mut self, _c: char) {}
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
    fn ss3(&mut self, c: char) {
        log::info!(target: "vte", "[ss3] c={c}");
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
struct Performer<'h, H: Handler> {
    handler: &'h mut H,
    /// https://en.wikipedia.org/wiki/C0_and_C1_control_codes#C1_control_codes_for_general_use
    /// https://en.wikipedia.org/wiki/ISO/IEC_2022#Shift_functions
    ss3: bool,
    csi_bracket: bool,
    osc_dispatch: bool,
}

pub fn new<'h, H: Handler>(h: &'h mut H) -> impl Perform + use<'h, H> {
    Performer { handler: h, ss3: false, csi_bracket: false, osc_dispatch: false }
}

impl<'h, H: Handler> Perform for Performer<'h, H> {
    fn print(&mut self, c: char) {
        if self.ss3 {
            self.ss3 = false;
            self.handler.ss3(c);
        } else if self.csi_bracket {
            self.csi_bracket = false;
            self.csi_dispatch(&Params::default(), &[], false, c);
        } else if c == '\x7F' {
            self.handler.execute(c as u8);
        } else {
            self.handler.print(c);
        }
    }

    fn execute(&mut self, b: u8) {
        self.handler.execute(b);
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        if c == '[' /*&& params.is_empty()*/&& intermediates.is_empty() && !ignore {
            self.csi_bracket = true;
        } else {
            self.handler.csi_dispatch(params, intermediates, ignore, c);
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, b: u8) {
        if b == b'O' {
            self.ss3 = true;
        } else if self.osc_dispatch && b == b'\\' {
            self.osc_dispatch = false;
        } else {
            self.handler.esc_dispatch(intermediates, ignore, b);
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        if !bell_terminated {
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
        self.handler.dcs_unhook()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Params, Parser};

    fn parse<H: Handler>(h: &mut H, bytes: &[u8]) {
        let mut x = Performer { handler: h, ss3: false, csi_bracket: false, osc_dispatch: false };
        let mut p = Parser::new();
        let n = p.advance_until_terminated(&mut x, bytes);
        assert_eq!(n, bytes.len());
        assert!(!x.ss3);
        assert!(!x.csi_bracket);
        assert!(!x.osc_dispatch);
    }

    #[test]
    fn test_ss3() {
        struct H(char);
        impl Handler for H {
            fn ss3(&mut self, c: char) {
                self.0 = c;
            }
        }
        let mut h = H('\0');
        // F1 on Mac / Windows terminal with ENABLE_VIRTUAL_TERMINAL_INPUT
        parse(&mut h, b"\x1BOA");
        assert_eq!('A', h.0);
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
                assert!(params.is_empty());
                assert!(intermediates.is_empty());
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
        parse(&mut h, b"\x1BP+q544e;524742;687061\x1B\\");
        parse(&mut h, b"\x1BP>|iTerm2[version]\x1B\\");
    }
}
