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
struct Performer<'h, H: Handler> {
    handler: &'h mut H,
    osc_dispatch: bool,
    dcs_unhook: bool,
}

pub fn new<'h, H: Handler>(h: &'h mut H) -> impl Perform + use<'h, H> {
    Performer { handler: h, osc_dispatch: false, dcs_unhook: false }
}

impl<'h, H: Handler> Perform for Performer<'h, H> {
    fn print(&mut self, c: char) {
        if c == '\x7F' {
            self.handler.execute(c as u8);
        } else {
            self.handler.print(c);
        }
    }

    fn execute(&mut self, b: u8) {
        self.handler.execute(b);
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        self.handler.csi_dispatch(params, intermediates, ignore, c);
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, b: u8) {
        if self.osc_dispatch && b == b'\\' {
            self.osc_dispatch = false;
        } else if self.dcs_unhook && b == b'\\' {
            self.dcs_unhook = false;
        } else {
            self.handler.esc_dispatch(intermediates, ignore, b);
        }
    }

    fn ss3_dispatch(&mut self, param: u16, c: char) {
        self.handler.ss3_dispatch(param, c);
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
        self.dcs_unhook = true;
        self.handler.dcs_unhook()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Params, Parser};

    fn parse<H: Handler>(h: &mut H, bytes: &[u8]) {
        let mut x = Performer { handler: h, osc_dispatch: false, dcs_unhook: false };
        let mut p = Parser::new();
        let n = p.advance_until_terminated(&mut x, bytes);
        assert_eq!(n, bytes.len());
        assert!(!x.osc_dispatch);
        assert!(!x.dcs_unhook);
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
        parse(&mut h, b"\x1B[11\x1E");
        // FIXME (rxvt)
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
