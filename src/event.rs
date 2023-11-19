use crate::{Params, Perform};

trait Handler {
    fn ss3(&mut self, _c: char) {}
    fn print(&mut self, _c: char) {}
    fn execute(&mut self, _b: u8) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _b: u8) {}
    fn csi_dispatch(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {}
}

struct Performer<'h, H: Handler> {
    handler: &'h mut H,
    /// https://en.wikipedia.org/wiki/C0_and_C1_control_codes#C1_control_codes_for_general_use
    /// https://en.wikipedia.org/wiki/ISO/IEC_2022#Shift_functions
    ss3: bool,
}

impl<'h, H: Handler> Perform for Performer<'h, H> {
    fn print(&mut self, c: char) {
        if self.ss3 {
            self.ss3 = false;
            self.handler.ss3(c);
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
        self.handler.csi_dispatch(params, intermediates, ignore, c);
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

    fn parse<'h, H: Handler>(h: &mut H, bytes: &[u8]) {
        let mut x = Performer { handler: h, ss3: false };
        let mut p = crate::Parser::new();
        for b in bytes {
            p.advance(&mut x, *b);
        }
        assert!(p.is_ground());
    }

    #[test]
    fn test_ss3() {
        struct H {
            c: char,
        }
        impl Handler for H {
            fn ss3(&mut self, c: char) {
                self.c = c;
            }
        }
        let mut h = H { c: '\0' };
        let bytes = &[0x1b, b'O', b'A'];
        parse(&mut h, bytes);
        assert_eq!('A', h.c);
    }

    #[test]
    fn test_backspace() {
        struct H {
            b: u8,
        }
        impl Handler for H {
            fn execute(&mut self, b: u8) {
                self.b = b;
            }
        }
        let mut h = H { b: 0 };
        let bytes = &[0x7f];
        parse(&mut h, bytes);
        assert_eq!(0x7f, h.b);
    }

    #[test]
    fn test_backspace() {
        struct H {
            b: u8,
        }
        impl Handler for H {
            fn execute(&mut self, b: u8) {
                self.b = b;
            }
        }
        let mut h = H { b: 0 };
        let bytes = &[0x7f];
        parse(&mut h, bytes);
        assert_eq!(0x7f, h.b);
    }
}
