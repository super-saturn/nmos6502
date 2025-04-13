pub struct ProcessorStatus {
    byte: u8
}

impl From<u8> for ProcessorStatus {
    fn from(b:u8) -> Self {
        ProcessorStatus { byte: b }
    }
}

// (N)eg | o(V)erflow | b0 | b1 | (D)ecimal | (I)nterrupt | (Z)ero | (C)arry 
impl ProcessorStatus {
    pub fn update_flags_with_compare(&mut self, regval:u8, cmp_val:u8) {
        // Compare sets flags as if a subtraction had been carried out.
        // If regval is equal or greater than the compared value, then (C)arry will be set.
        // The zero (Z) and negative (N) flags will be set based on equality or
        // lack thereof and the sign (i.e. A>=$80) of the first value.

        if regval == cmp_val {
            self.clr_negative();
            self.set_zero();
            self.set_carry();
            return;
        }

        self.clr_zero();
        
        if regval > cmp_val {
            self.set_carry();
        } else {
            self.clr_carry();
        }

        if (regval as i8).wrapping_sub(cmp_val as i8) < 0 {
            self.set_negative();
        } else {
            self.clr_negative();
        }
    }

    // Sets (Z)ero and (N)egative flags
    pub fn update_zero_neg_flags(&mut self, val:u8) {
        if val == 0 {
            self.set_zero();
        } else {
            self.clr_zero();
        }
        if (val as i8) < 0 {
            self.set_negative();
        } else {
            self.clr_negative();
        }
    }
    pub fn set_carry(&mut self) {
        self.byte |= 0b0000_0001;
    }
    pub fn clr_carry(&mut self) {
        self.byte &= 0b1111_1110;
    }
    pub fn carry(&self) -> bool {
        (self.byte & 0b0000_0001) > 0
    }
    pub fn set_zero(&mut self) {
        self.byte |= 0b0000_0010;
    } 
    pub fn clr_zero(&mut self) {
        self.byte &= 0b1111_1101;
    }
    pub fn zero(&self) -> bool {
        (self.byte & 0b0000_0010) > 0
    }
    pub fn set_interrupt_disable(&mut self) {
        self.byte |= 0b0000_0100;
    }
    pub fn clr_interrupt_disable(&mut self) {
        self.byte &= 0b1111_1011;
    }
    pub fn interrupt_disable(&self) -> bool {
        (self.byte & 0b0000_0100) > 0
    }
    pub fn set_decimal(&mut self) {
        self.byte |= 0b0000_1000;
    }
    pub fn clr_decimal(&mut self) {
        self.byte &= 0b1111_0111;
    }
    pub fn decimal(&self) -> bool {
        (self.byte & 0b0000_1000) > 0
    }
    pub fn set_overflow(&mut self) {
        self.byte |= 0b0100_0000;
    }
    pub fn clr_overflow(&mut self) {
        self.byte &= 0b1011_1111;
    }
    pub fn overflow(&self) -> bool {
        (self.byte & 0b0100_0000) > 0
    }
    pub fn set_negative(&mut self) {
        self.byte |= 0b1000_0000;
    }
    pub fn clr_negative(&mut self) {
        self.byte &= 0b0111_1111;
    }
    pub fn negative(&self) -> bool {
        (self.byte & 0b1000_0000) > 0
    }
    pub fn as_byte(&self) -> u8 {
        self.byte
    }
}