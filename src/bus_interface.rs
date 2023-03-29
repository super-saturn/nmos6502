pub trait BusInterface {
    fn get_byte_at(&mut self, addr:u16) -> u8;
    fn set_byte_at(&mut self, addr:u16, byte: u8);
    // fn indirect_x_addr(&mut self, byte:u8, x:u8) -> u16;
    // fn indirect_y_addr(&mut self, byte:u8, y:u8) -> u16;
    // fn zero_page_addr(index: u8, off:u8) -> u16;
    // fn abs_addr(lo:u8, hi:u8, off:u8) -> u16;

    // specifically used for opcode + param retrieval.
    // This is the naive implementation; you may wish to override.
    fn get_pipelined_bytes(&mut self, addr:u16) -> (u8, u8, u8) {
        let opcode = self.get_byte_at(addr);
        let b1 = self.get_byte_at(addr.wrapping_add(1));
        let b2 = self.get_byte_at(addr.wrapping_add(2));
        (opcode, b1, b2)
    }
}