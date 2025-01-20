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
}

/// Adapter from `crate::Perform` to `Handler`
struct Performer<'h, H: Handler> {
    handler: &'h mut H,
    /// https://en.wikipedia.org/wiki/C0_and_C1_control_codes#C1_control_codes_for_general_use
    /// https://en.wikipedia.org/wiki/ISO/IEC_2022#Shift_functions
    ss3: bool,
    csi_bracket: bool,
}

pub fn new<'h, H: Handler>(h: &'h mut H) -> impl Perform + use<'h, H> {
    Performer { handler: h, ss3: false, csi_bracket: false }
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
        } else {
            self.handler.esc_dispatch(intermediates, ignore, b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Params, Parser};

    fn parse<'h, H: Handler>(h: &mut H, bytes: &[u8]) {
        let mut x = new(h);
        let mut p = Parser::new();
        p.advance(&mut x, bytes);
        assert!(p.is_ground());
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
        parse(&mut h, &[0x1B, b'O', b'A']);
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
        parse(&mut h, &[0x7F]);
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
        parse(&mut h, &[0x1B, b'[', b'[', b'A']);
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
        parse(&mut h, &[0x1B, 0x0D]);
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
        parse(&mut h, &[0x1B, 0x7F]);
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
        parse(&mut h, &[0x1B, 0x09]);
        assert_eq!(0x09, h.0);
    }
}
