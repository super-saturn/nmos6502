use crate::{opcodes::Opcode, processor_status::ProcessorStatus};
use crate::bus_interface::BusInterface;

pub struct Nmos6502 {
    
    current_opcode: Opcode,
    registers: Registers,
    processor_status: ProcessorStatus,

    pub last_pc_cycles:u8,
    pub irq: bool,
    pub nmi: bool,
    pub halted: bool,

    pub break_flag_ext_debug: bool,
    pub uncaught_opcode_debug: Option<u8>,
    pub last_pc_debug: u16,
    pub num_instructions_executed_debug:u32,
}

enum InterruptType {
    BRK,
    IRQ,
    NMI
}

impl Nmos6502 {

    pub fn new() -> Self {
        Nmos6502 {
            current_opcode: Opcode::CLD,
            registers: Registers {
                program_counter: 0,
                accumulator: 0,
                x: 0, y: 0,
                stack_pointer: 0xFF
            },
            processor_status: 0b0011_0000.into(),
            irq: false,
            nmi: false,
            halted: false,
            break_flag_ext_debug: true,
            uncaught_opcode_debug: None,
            last_pc_debug: 0,
            num_instructions_executed_debug: 0,
            last_pc_cycles: 0
        }
    }

    pub fn reset<T:BusInterface>(&mut self, bus:&mut T) {
        let reset_vec_lo = bus.get_byte_at(0xfffc);
        let reset_vec_hi =  bus.get_byte_at(0xfffd);
        self.registers.program_counter = self.abs_addr(reset_vec_lo, reset_vec_hi, 0);
    }

    fn push_stack_interrupt<T:BusInterface>(&mut self, ir_type:InterruptType, bus:&mut T) {
        let pc_bytes = self.registers.program_counter.to_le_bytes();

        self.push_stack(bus, pc_bytes[1]);
        self.push_stack(bus, pc_bytes[0]);

        let flags_mask = match ir_type { 
            InterruptType::BRK => 0b0011_0000,
            _ => 0b0010_0000 // NMI, IRQ
        };
        let status = self.processor_status.as_byte() | flags_mask;

        self.push_stack(bus, status);
        self.processor_status.set_interrupt_disable();

        let fetch_vec = match ir_type {
            InterruptType::NMI => 0xFFFA,
            InterruptType::BRK => 0xFFFE,
            InterruptType::IRQ => 0xFFFE,
        };

        let reset_vec_lo = bus.get_byte_at(fetch_vec);
        let reset_vec_hi =  bus.get_byte_at(fetch_vec+0x1);

        self.registers.program_counter = self.abs_addr(reset_vec_lo, reset_vec_hi, 0);
    }

    pub fn tick<T:BusInterface>(&mut self, bus:&mut T) {
        if self.halted {
            return;
        }

        if self.nmi {
            self.push_stack_interrupt(InterruptType::NMI, bus);
            return;
        } else if self.irq && !self.processor_status.interrupt_disable() {
            self.push_stack_interrupt(InterruptType::IRQ, bus);
            return;
        }
        
        let (raw_opcode_byte, pipe_byte1, pipe_byte2) = bus.get_pipelined_bytes(self.registers.program_counter);
        let opcode:Opcode = raw_opcode_byte.into();
        self.current_opcode = opcode;

        self.num_instructions_executed_debug = self.num_instructions_executed_debug.wrapping_add(1);
        self.last_pc_cycles = opcode.cycle_inc();
        
        // inc PC after fetch
        self.last_pc_debug = self.registers.program_counter;
        self.registers.program_counter = self.registers.program_counter.wrapping_add(opcode.pc_inc());
        match self.current_opcode {
            Opcode::ANDabs => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator &= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ANDabsX => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator &= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ANDabsY => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.y);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator &= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ANDimm => {
                self.registers.accumulator &= pipe_byte1;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ANDindX => {
                let addr =  self.indirect_x_addr(bus,pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator &= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ANDindY => {
                let addr = self.indirect_y_addr(bus,pipe_byte1, self.registers.y);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator &= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ANDz => {
                let addr = self.zero_page_addr(pipe_byte1,0);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator &= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ANDzX => {
                let addr = self.zero_page_addr(pipe_byte1,self.registers.x);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator &= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ASLabs => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.arithmetic_shift_left(val));
            },
            Opcode::ASLabsX => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.arithmetic_shift_left(val));
            },
            Opcode::ASLacc => {
                self.registers.accumulator = self.arithmetic_shift_left(self.registers.accumulator);
            },
            Opcode::ASLz => {
                let addr = self.zero_page_addr(pipe_byte1, 0);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.arithmetic_shift_left(val));
            },
            Opcode::ASLzX => {
                let addr = self.zero_page_addr(pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.arithmetic_shift_left(val));
            },
            Opcode::ADCabs => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let val = bus.get_byte_at(addr);
                self.add_with_carry(val);
            },
            Opcode::ADCabsX => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.add_with_carry(val);
            },
            Opcode::ADCabsY => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.y);
                let val = bus.get_byte_at(addr);
                self.add_with_carry(val);
            },
            Opcode::ADCimm => { // immediate
                self.add_with_carry(pipe_byte1);
            },
            Opcode::ADCindX => {
                let addr = self.indirect_x_addr(bus,pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.add_with_carry(val);
            },
            Opcode::ADCindY => {
                let addr = self.indirect_y_addr(bus,pipe_byte1, self.registers.y);
                let val = bus.get_byte_at(addr);
                self.add_with_carry(val);
            },
            Opcode::ADCz => {
                let addr = self.zero_page_addr(pipe_byte1, 0);
                let val = bus.get_byte_at(addr);
                self.add_with_carry(val);
            },
            Opcode::ADCzX => {
                let addr = self.zero_page_addr(pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.add_with_carry(val);
            },
            Opcode::BITabs => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let val = bus.get_byte_at(addr);
                self.bit_test(val);
            },
            Opcode::BITz => {
                let addr = self.zero_page_addr(pipe_byte1, 0);
                let val = bus.get_byte_at(addr);
                self.bit_test(val);
            },
            Opcode::DECabs => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let val = bus.get_byte_at(addr).wrapping_sub(1);
                self.processor_status.update_zero_neg_flags(val);
                bus.set_byte_at(addr, val);
            },
            Opcode::DECabsX => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                let val = bus.get_byte_at(addr).wrapping_sub(1);
                self.processor_status.update_zero_neg_flags(val);
                bus.set_byte_at(addr, val);
            },
            Opcode::DECz => {
                let addr = self.zero_page_addr(pipe_byte1, 0);
                let val = bus.get_byte_at(addr).wrapping_sub(1);
                self.processor_status.update_zero_neg_flags(val);
                bus.set_byte_at(addr, val);
            },
            Opcode::DECzX => {
                let addr = self.zero_page_addr(pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr).wrapping_sub(1);
                self.processor_status.update_zero_neg_flags(val);
                bus.set_byte_at(addr, val);
            },
            Opcode::EORabs => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator ^= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::EORabsX => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator ^= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::EORabsY => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.y);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator ^= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::EORimm => {
                self.registers.accumulator ^= pipe_byte1;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::EORindX => {
                let addr = self.indirect_x_addr(bus,pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator ^= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::EORindY => {
                let addr = self.indirect_y_addr(bus,pipe_byte1, self.registers.y);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator ^= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::EORz => {
                let addr = self.zero_page_addr(pipe_byte1, 0);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator ^= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::EORzX => {
                let addr = self.zero_page_addr(pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator ^= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::JMP => {
                self.registers.program_counter = self.abs_addr(pipe_byte1,pipe_byte2, 0);
            },
            Opcode::JMPi => {
                let indirect_jmp_addr =self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let lo = bus.get_byte_at(indirect_jmp_addr);
                let hi = bus.get_byte_at(indirect_jmp_addr.wrapping_add(1));
                self.registers.program_counter = self.abs_addr(lo,hi, 0);
            },
            Opcode::JSR => {
                let jmp_addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let pc_rtn_addr_bytes = self.registers.program_counter.wrapping_sub(1).to_le_bytes();
                self.push_stack(bus, pc_rtn_addr_bytes[1]);
                self.push_stack(bus, pc_rtn_addr_bytes[0]);
                self.registers.program_counter = jmp_addr;
            },
            Opcode::LDAimm => { // immediate
                self.registers.accumulator = pipe_byte1;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::LDAz => { // zero page
                let get_addr = self.zero_page_addr(pipe_byte1,0);
                self.registers.accumulator = bus.get_byte_at(get_addr);
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::LDAzX => { // zero page
                let get_addr = self.zero_page_addr(pipe_byte1,self.registers.x);
                self.registers.accumulator = bus.get_byte_at(get_addr);
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::LDAabs => { // absolute
                let get_addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                self.registers.accumulator = bus.get_byte_at(get_addr);
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::LDAabsX => {
                let get_addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                self.registers.accumulator = bus.get_byte_at(get_addr);
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::LDAabsY => {
                let get_addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.y);
                self.registers.accumulator = bus.get_byte_at(get_addr);
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::LDAindX => {
                let addr = self.indirect_x_addr(bus,pipe_byte1, self.registers.x);

                self.registers.accumulator = bus.get_byte_at(addr);
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::LDAindY => {
                let addr = self.indirect_y_addr(bus,pipe_byte1, self.registers.y);
                self.registers.accumulator = bus.get_byte_at(addr);
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::LDXabs => {
                let get_addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                self.registers.x = bus.get_byte_at(get_addr);
                self.processor_status.update_zero_neg_flags(self.registers.x);
            },
            Opcode::LDXabsY => {
                let get_addr = self.abs_addr(pipe_byte1,pipe_byte2, self.registers.y);
                self.registers.x = bus.get_byte_at(get_addr);
                self.processor_status.update_zero_neg_flags(self.registers.x);
            },
            Opcode::LDXimm => {
                self.registers.x = pipe_byte1;
                self.processor_status.update_zero_neg_flags(self.registers.x);
            },
            Opcode::LDXz => {
                let addr = self.zero_page_addr(pipe_byte1,0);
                self.registers.x = bus.get_byte_at(addr);
                self.processor_status.update_zero_neg_flags(self.registers.x);
            },
            Opcode::LDXzy => {
                let addr = self.zero_page_addr(pipe_byte1,self.registers.y);
                self.registers.x = bus.get_byte_at(addr);
                self.processor_status.update_zero_neg_flags(self.registers.x);
            },
            Opcode::LDYabs => {
                let get_addr = self.abs_addr(pipe_byte1,pipe_byte2,0);
                self.registers.y = bus.get_byte_at(get_addr);
                self.processor_status.update_zero_neg_flags(self.registers.y);
            },
            Opcode::LDYabsX => {
                let get_addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                self.registers.y = bus.get_byte_at(get_addr);
                self.processor_status.update_zero_neg_flags(self.registers.y);
            },
            Opcode::LDYimm => {
                self.registers.y = pipe_byte1;
                self.processor_status.update_zero_neg_flags(self.registers.y);
            },
            Opcode::LDYz => {
                let addr = self.zero_page_addr(pipe_byte1,0);
                self.registers.y = bus.get_byte_at(addr);
                self.processor_status.update_zero_neg_flags(self.registers.y);
            },
            Opcode::LDYzx => {
                let addr = self.zero_page_addr(pipe_byte1,self.registers.x);
                self.registers.y = bus.get_byte_at(addr);
                self.processor_status.update_zero_neg_flags(self.registers.y);
            },
            Opcode::LSRabs => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.logical_shift_right(val));
            },
            Opcode::LSRabsX => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.logical_shift_right(val));
            },
            Opcode::LSRacc => {
                self.registers.accumulator = self.logical_shift_right(self.registers.accumulator);
            },
            Opcode::LSRz => {
                let addr = self.zero_page_addr(pipe_byte1, 0);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.logical_shift_right(val));
            },
            Opcode::LSRzX => {
                let addr = self.zero_page_addr(pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.logical_shift_right(val));
            },
            Opcode::ORAabs => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator |= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ORAabsX => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator |= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ORAabsY => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.y);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator |= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ORAimm => {
                self.registers.accumulator |= pipe_byte1;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ORAindX => {
                let addr = self.indirect_x_addr(bus,pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator |= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ORAindY => {
                let addr = self.indirect_y_addr(bus,pipe_byte1, self.registers.y);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator |= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ORAz => {
                let addr = self.zero_page_addr(pipe_byte1, 0);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator |= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ORAzX => {
                let addr = self.zero_page_addr(pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.registers.accumulator |= val;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::ROLabs => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.rotate_left(val));
            },
            Opcode::ROLabsX => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.rotate_left(val));
            },
            Opcode::ROLacc => {
                self.registers.accumulator = self.rotate_left(self.registers.accumulator);
            },
            Opcode::ROLz => {
                let addr = self.zero_page_addr(pipe_byte1, 0);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.rotate_left(val));
            },
            Opcode::ROLzX => {
                let addr = self.zero_page_addr(pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.rotate_left(val));
            },
            Opcode::RORabs => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.rotate_right(val));
            },
            Opcode::RORabsX => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.rotate_right(val));
            },
            Opcode::RORacc => {
                self.registers.accumulator = self.rotate_right(self.registers.accumulator);
            },
            Opcode::RORz => {
                let addr = self.zero_page_addr(pipe_byte1, 0);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.rotate_right(val));
            },
            Opcode::RORzX => {
                let addr = self.zero_page_addr(pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                bus.set_byte_at(addr, self.rotate_right(val));
            },
            Opcode::RTI => {
                let mut status = self.pull_stack(bus) & 0b1100_1111;
                status |= self.processor_status.as_byte() & 0b0011_0000;
                self.processor_status = status.into();

                let ret_addr_lo = self.pull_stack(bus);
                let ret_addr_hi =  self.pull_stack(bus);
                let ret_addr = self.abs_addr(ret_addr_lo, ret_addr_hi, 0);

                self.registers.program_counter = ret_addr;
            },
            Opcode::RTS => {
                let ret_addr_lo = self.pull_stack(bus);
                let ret_addr_hi =  self.pull_stack(bus);
                let ret_addr = self.abs_addr(ret_addr_lo,ret_addr_hi, 1);
                self.registers.program_counter = ret_addr;
            },
            Opcode::SBCabs => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, 0);
                let val = bus.get_byte_at(addr);
                self.subtract_with_carry(val);
            },
            Opcode::SBCabsX => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.subtract_with_carry(val);
            },
            Opcode::SBCabsY => {
                let addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.y);
                let val = bus.get_byte_at(addr);
                self.subtract_with_carry(val);
            },
            Opcode::SBCimm => { // immediate
                self.subtract_with_carry(pipe_byte1);
            },
            Opcode::SBCindX => {
                let addr = self.indirect_x_addr(bus,pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.subtract_with_carry(val);
            },
            Opcode::SBCindY => {
                let addr = self.indirect_y_addr(bus,pipe_byte1, self.registers.y);
                let val = bus.get_byte_at(addr);
                self.subtract_with_carry(val);
            },
            Opcode::SBCz => {
                let addr = self.zero_page_addr(pipe_byte1, 0);
                let val = bus.get_byte_at(addr);
                self.subtract_with_carry(val);
            },
            Opcode::SBCzX => {
                let addr = self.zero_page_addr(pipe_byte1, self.registers.x);
                let val = bus.get_byte_at(addr);
                self.subtract_with_carry(val);
            },
            Opcode::STA => {
                let set_addr = self.abs_addr(pipe_byte1,pipe_byte2, 0);
                bus.set_byte_at(set_addr, self.registers.accumulator);
            },
            Opcode::STAz => {
                let set_addr = self.zero_page_addr(pipe_byte1,0);
                bus.set_byte_at(set_addr, self.registers.accumulator);
            },
            Opcode::STAzX => {
                let set_addr = self.zero_page_addr(pipe_byte1,self.registers.x);
                bus.set_byte_at(set_addr, self.registers.accumulator);
            },
            Opcode::STAabsX => { // store accumulator absolute + relative X
                let set_addr = self.abs_addr(pipe_byte1, pipe_byte2, self.registers.x);
                bus.set_byte_at(set_addr, self.registers.accumulator);
            },
            Opcode::STAay => {
                let set_addr = self.abs_addr(pipe_byte1,pipe_byte2, self.registers.y);
                bus.set_byte_at(set_addr, self.registers.accumulator);
            },
            Opcode::STAindX => {
                let addr =  self.indirect_x_addr(bus,pipe_byte1, self.registers.x);

                bus.set_byte_at(addr, self.registers.accumulator);
            },
            Opcode::STAindY => {
                let addr = self.indirect_y_addr(bus,pipe_byte1, self.registers.y);
                bus.set_byte_at(addr, self.registers.accumulator);
            },
            Opcode::STX => {
                let set_addr = self.abs_addr(pipe_byte1,pipe_byte2, 0);
                bus.set_byte_at(set_addr, self.registers.x);
            },
            Opcode::STXz => {
                let set_addr = self.zero_page_addr(pipe_byte1,0);
                bus.set_byte_at(set_addr, self.registers.x);
            },
            Opcode::STXzY => {
                let set_addr = self.zero_page_addr(pipe_byte1,self.registers.y);
                bus.set_byte_at(set_addr, self.registers.x);
            },
            Opcode::STY => {
                let set_addr = self.abs_addr(pipe_byte1,pipe_byte2, 0);
                bus.set_byte_at(set_addr, self.registers.y);
            },
            Opcode::STYz => {
                let set_addr = self.zero_page_addr(pipe_byte1,0);
                bus.set_byte_at(set_addr, self.registers.y);
            }
            Opcode::STYzX => {
                let set_addr = self.zero_page_addr(pipe_byte1,self.registers.x);
                bus.set_byte_at(set_addr, self.registers.y);
            },
            Opcode::TXS => { // transfer X to SP
                self.registers.stack_pointer = self.registers.x;
            },
            Opcode::TSX => { // transfer SP to X
                self.registers.x = self.registers.stack_pointer;
                self.processor_status.update_zero_neg_flags(self.registers.x);
            },
            Opcode::TAX => {
                self.registers.x = self.registers.accumulator;
                self.processor_status.update_zero_neg_flags(self.registers.x);
            },
            Opcode::TXA => {
                self.registers.accumulator = self.registers.x;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::TAY => {
                self.registers.y = self.registers.accumulator;
                self.processor_status.update_zero_neg_flags(self.registers.y);
            },
            Opcode::TYA => {
                self.registers.accumulator = self.registers.y;
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::PHP => {
                // possibly required to set bits 4 & 5 when pushing..
                // self.processor_status.byte |= 0b0011_0000;
                let mut push_status = self.processor_status.as_byte();
                push_status |= 0b0011_0000;
                self.push_stack(bus, push_status);
            },
            Opcode::PHA => {
                self.push_stack(bus, self.registers.accumulator);
            },
            Opcode::PLA => {
                self.registers.accumulator = self.pull_stack(bus);
                self.processor_status.update_zero_neg_flags(self.registers.accumulator);
            },
            Opcode::PLP => {
                // errata: bflag0 and 1 can not be pulled with PLP
                // these two bits do not physically exist on the real processor, and always report as 1
                let mut status_without_bflags = self.pull_stack(bus) & 0b1100_1111;
                status_without_bflags |= self.processor_status.as_byte() & 0b0011_0000;
                self.processor_status = status_without_bflags.into();
            },
            Opcode::CLC => {
                self.processor_status.clr_carry();
            },
            Opcode::SEC => {
                self.processor_status.set_carry();
            }
            Opcode::CLD => {
                self.processor_status.clr_decimal();
            },
            Opcode::SED => {
                self.processor_status.set_decimal();
            }
            Opcode::CLI => {
                self.processor_status.clr_interrupt_disable();
            },
            Opcode::SEI => {
                self.processor_status.set_interrupt_disable();
            },
            Opcode::CLV => {
                self.processor_status.clr_overflow();
            },
            Opcode::INX => {
                self.registers.x = self.registers.x.wrapping_add(1);
                self.processor_status.update_zero_neg_flags(self.registers.x);
            },
            Opcode::DEX => {
                self.registers.x = self.registers.x.wrapping_sub(1);
                self.processor_status.update_zero_neg_flags(self.registers.x);
            }
            Opcode::INY => {
                self.registers.y = self.registers.y.wrapping_add(1);
                self.processor_status.update_zero_neg_flags(self.registers.y);
            },
            Opcode::INCabs => {
                let addr = self.abs_addr(pipe_byte1,pipe_byte2,0);
                let val = bus.get_byte_at(addr).wrapping_add(1);
                self.processor_status.update_zero_neg_flags(val);
                bus.set_byte_at(addr, val);
            },
            Opcode::INCabsx => {
                let addr = self.abs_addr(pipe_byte1,pipe_byte2,self.registers.x);
                let val = bus.get_byte_at(addr).wrapping_add(1);
                self.processor_status.update_zero_neg_flags(val);
                bus.set_byte_at(addr, val);
            },
            Opcode::INCz => {
                let addr = self.zero_page_addr(pipe_byte1,0);
                let val = bus.get_byte_at(addr).wrapping_add(1);
                self.processor_status.update_zero_neg_flags(val);
                bus.set_byte_at(addr, val);
            },
            Opcode::INCzx => { // note: we are supposed to wrap within pages
                let addr = self.zero_page_addr(pipe_byte1,self.registers.x);
                let val = bus.get_byte_at(addr).wrapping_add(1);
                self.processor_status.update_zero_neg_flags(val);
                bus.set_byte_at(addr, val);
            },
            Opcode::DEY => {
                self.registers.y = self.registers.y.wrapping_sub(1);
                self.processor_status.update_zero_neg_flags(self.registers.y);
            },
            Opcode::BCC => {
                if !self.processor_status.carry() {
                    self.last_pc_cycles += 1;
                    self.branch_by_offset(pipe_byte1);
                }
            },
            Opcode::BCS => {
                if self.processor_status.carry() {
                    self.last_pc_cycles += 1;
                    self.branch_by_offset(pipe_byte1);
                }
            },
            Opcode::BEQ => {
                if self.processor_status.zero() {
                    self.last_pc_cycles += 1;
                    self.branch_by_offset(pipe_byte1);
                }
            },
            Opcode::BNE => {
                if !self.processor_status.zero() {
                    self.last_pc_cycles += 1;
                    self.branch_by_offset(pipe_byte1);
                }
            },
            Opcode::BPL => {
                if !self.processor_status.negative() {
                    self.last_pc_cycles += 1;
                    self.branch_by_offset(pipe_byte1);
                }
            },
            Opcode::BMI => {
                if self.processor_status.negative() {
                    self.last_pc_cycles += 1;
                    self.branch_by_offset(pipe_byte1);
                }
            },
            Opcode::BVC => {
                if !self.processor_status.overflow() {
                    self.last_pc_cycles += 1;
                    self.branch_by_offset(pipe_byte1);
                }
            },
            Opcode::BVS => {
                if self.processor_status.overflow() {
                    self.last_pc_cycles += 1;
                    self.branch_by_offset(pipe_byte1);
                }
            },
            Opcode::CPX => {
                self.processor_status.update_flags_with_compare(self.registers.x,pipe_byte1);
            },
            Opcode::CPXz => {
                let get_addr = self.zero_page_addr(pipe_byte1,0);
                let cmp_val = bus.get_byte_at(get_addr);
                self.processor_status.update_flags_with_compare(self.registers.x,cmp_val);
            },
            Opcode::CPXabs => {
                let get_addr = self.abs_addr(pipe_byte1,pipe_byte2, 0);
                let cmp_val = bus.get_byte_at(get_addr);
                self.processor_status.update_flags_with_compare(self.registers.x, cmp_val);
            },
            Opcode::CPY => {
                self.processor_status.update_flags_with_compare(self.registers.y,pipe_byte1);
            }
            Opcode::CPYz => {
                let get_addr = self.zero_page_addr(pipe_byte1,0);
                let val = bus.get_byte_at(get_addr);
                self.processor_status.update_flags_with_compare(self.registers.y,val);
            },
            Opcode::CPYabs => {
                let get_addr = self.abs_addr(pipe_byte1,pipe_byte2, 0);
                let val = bus.get_byte_at(get_addr);
                self.processor_status.update_flags_with_compare(self.registers.y, val);
            },
            Opcode::CMPabs => {
                let cmp_addr = self.abs_addr(pipe_byte1,pipe_byte2, 0);
                let val = bus.get_byte_at(cmp_addr);
                self.processor_status.update_flags_with_compare(self.registers.accumulator,val);
            },
            Opcode::CMPabsx => { 
                let cmp_addr = self.abs_addr(pipe_byte1,pipe_byte2, self.registers.x);
                let val = bus.get_byte_at(cmp_addr);
            
                self.processor_status.update_flags_with_compare(self.registers.accumulator,val);
            },
            Opcode::CMPabsy => { 
                let cmp_addr = self.abs_addr(pipe_byte1,pipe_byte2, self.registers.y);
                let cmp_val = bus.get_byte_at(cmp_addr);
            
                self.processor_status.update_flags_with_compare(self.registers.accumulator,cmp_val);
            },
            Opcode::CMPindX => {
                let addr =  self.indirect_x_addr(bus,pipe_byte1, self.registers.x);

                let cmp_val = bus.get_byte_at(addr);
                self.processor_status.update_flags_with_compare(self.registers.accumulator, cmp_val);
            },
            Opcode::CMPindY => {
                let addr = self.indirect_y_addr(bus,pipe_byte1, self.registers.y);
                let cmp_val = bus.get_byte_at(addr);
                self.processor_status.update_flags_with_compare(self.registers.accumulator, cmp_val);
            },
            Opcode::CMPimm => {
                self.processor_status.update_flags_with_compare(self.registers.accumulator,pipe_byte1);
            },
            Opcode::CMPz => {
                let cmp_addr = self.zero_page_addr(pipe_byte1,0);
                let val = bus.get_byte_at(cmp_addr);
                self.processor_status.update_flags_with_compare(self.registers.accumulator, val);
            },
            Opcode::CMPzX => {
                let cmp_addr = self.zero_page_addr(pipe_byte1,self.registers.x);
                let val = bus.get_byte_at(cmp_addr);
                self.processor_status.update_flags_with_compare(self.registers.accumulator, val);
            },
            Opcode::BRK => {
                self.push_stack_interrupt(InterruptType::BRK, bus);
                self.break_flag_ext_debug = true;
            }, 
            Opcode::NOP => (),
            Opcode::NOPi0 => { // "Illegal" immediate NOP

            },
            Opcode::NOPim => {
                self.uncaught_opcode_debug = Some(raw_opcode_byte);
            } // "Illegal" implied NOP (here for debug)
        }

    }


    fn indirect_x_addr<T:BusInterface>(&mut self, bus:&mut T, byte:u8, x:u8) -> u16 {
        let zp_addr = self.zero_page_addr(byte,x);
        u16::from_le_bytes([bus.get_byte_at(zp_addr),bus.get_byte_at(zp_addr.wrapping_add(1))])
    }

    fn indirect_y_addr<T:BusInterface>(&mut self, bus:&mut T, byte:u8, y:u8) -> u16 {
        let zp_addr = self.zero_page_addr(byte,0);
        if (zp_addr as u8).overflowing_add(y).1 {
            self.last_pc_cycles += 1
        }
        let addr = self.abs_addr(bus.get_byte_at(zp_addr),bus.get_byte_at(zp_addr.wrapping_add(1)), 0);
        addr.wrapping_add(y as u16)
    }

    fn zero_page_addr(&mut self, index:u8, off:u8) -> u16 {
        if index.overflowing_add(off).1 {
            self.last_pc_cycles += 1;
        }
        (index.wrapping_add(off)) as u16
    }

    fn abs_addr(&mut self, lo:u8,hi:u8,off:u8) -> u16 {
        let addr = u16::from_le_bytes([lo,hi]).wrapping_add(off as u16);
        if (addr as u8).overflowing_add(off).1 {
            self.last_pc_cycles += 1
        }
        addr
    }

    fn add_with_carry(&mut self, byte:u8) {
        let c = match self.processor_status.carry() {
            false => 0,
            true => 1
        };

        let mut uresult = self.registers.accumulator.wrapping_add(byte); 

        if !self.processor_status.decimal() {
            // set carry based on unsigned math
            if (self.registers.accumulator > uresult) || (byte > uresult) {
                self.processor_status.set_carry();
            } else {
                if uresult == 0xFF && c == 1 { // stupid edge case
                    self.processor_status.set_carry();
                } else {
                    self.processor_status.clr_carry();
                }
            }

            uresult = uresult.wrapping_add(c);
        } else {
            let a_lo = self.registers.accumulator & 0xF;
            let a_hi = self.registers.accumulator >> 4;
            let op_lo = byte & 0xF;
            let op_hi = byte >> 4;

            let lo_result = a_lo + op_lo + c;
            let c = if lo_result > 9 { 1 } else { 0 };
            let hi_result = a_hi + op_hi + c;
            
            if hi_result > 9 {
                self.processor_status.set_carry();
            } else {
                self.processor_status.clr_carry();
            }

            uresult = (lo_result%10) | ((hi_result%10) << 4);
        }

        self.processor_status.clr_overflow();

        // if 7 bit of acc and pipe are the same,
        // they are either both neg or both pos
        // so therefore some risk of overflow
        if (self.registers.accumulator & 0b1000_0000) == (byte & 0b1000_0000) {
            // if the sign bit of the result does not match,
            // we overflowed.
            if (uresult & 0b1000_0000) != (byte & 0b1000_0000) {
                self.processor_status.set_overflow();
            }
        }

        // finally we can just set the result as the unsigned version
        self.registers.accumulator = uresult;

        self.processor_status.update_zero_neg_flags(self.registers.accumulator);
    }

    fn subtract_with_carry(&mut self, byte:u8) {
        if !self.processor_status.decimal() {
            let inv_byte = !byte;
            self.add_with_carry(inv_byte); // maybe? lol
            return;
        }
        
        // decimal sbc
        let mut c = match self.processor_status.carry() {
            false => 1,
            true => 0
        };

        let a_lo = self.registers.accumulator & 0xF;
        let a_hi = self.registers.accumulator >> 4;
        let op_lo = byte & 0xF;
        let op_hi = byte >> 4;

        let mut lo_result = a_lo.wrapping_sub(op_lo + c);
        if lo_result > 10 {
            // wrapped under
            c = 1;
            lo_result = lo_result.wrapping_add(10);
        } else {
            c = 0;
        }

        let mut hi_result = a_hi.wrapping_sub(op_hi + c);
        if hi_result > 10 {
            self.processor_status.clr_carry();
            hi_result = hi_result.wrapping_add(10);
        } else {
            self.processor_status.set_carry();
        }

        let uresult = lo_result | hi_result.checked_shl(4).unwrap();

        self.processor_status.clr_overflow();
        if (self.registers.accumulator & 0b1000_0000) == (byte & 0b1000_0000) {
            // if the sign bit of the result does not match,
            // we overflowed.
            if (uresult & 0b1000_0000) != (byte & 0b1000_0000) {
                self.processor_status.set_overflow();
            }
        }

        self.processor_status.update_zero_neg_flags(uresult);
        self.registers.accumulator = uresult;
    }

    fn branch_by_offset(&mut self, byte:u8) {
        let signed_byte = byte as i8;
        let jmp_addr = self.registers.program_counter.wrapping_add_signed(signed_byte as i16);
        self.registers.program_counter = jmp_addr;
    }

    fn push_stack<T:BusInterface>(&mut self, mem:&mut T, byte:u8) {
        let set_addr = self.abs_addr(self.registers.stack_pointer, 0x01, 0);
        mem.set_byte_at(set_addr, byte);
        self.registers.stack_pointer = self.registers.stack_pointer.wrapping_sub(1);
    }

    fn pull_stack<T:BusInterface>(&mut self, mem:&mut T) -> u8 {
        self.registers.stack_pointer = self.registers.stack_pointer.wrapping_add(1);
        let get_addr = self.abs_addr(self.registers.stack_pointer, 0x01, 0);
        mem.get_byte_at(get_addr)
    }

    // This is a weird test.
    fn bit_test(&mut self, val:u8) {
        if (val & 0b1000_0000) > 0 {
            self.processor_status.set_negative();
        } else {
            self.processor_status.clr_negative();
        }
        if (val & 0b0100_0000) > 0 {
            self.processor_status.set_overflow();
        } else {
            self.processor_status.clr_overflow();
        }
        if (val & self.registers.accumulator) == 0 {
            self.processor_status.set_zero();
        } else {
            self.processor_status.clr_zero();
        }
    }

    fn arithmetic_shift_left(&mut self, val:u8) -> u8 {
        if (val & 0b1000_0000) > 0 {
            self.processor_status.set_carry();
        } else {
            self.processor_status.clr_carry();
        }
        
        let result = val.checked_shl(1).unwrap();
        self.processor_status.update_zero_neg_flags(result);
        result
    }

    fn logical_shift_right(&mut self, val:u8) -> u8 {
        if (val & 0b0000_0001) > 0 {
            self.processor_status.set_carry();
        } else {
            self.processor_status.clr_carry();
        }
        
        let result = val.checked_shr(1).unwrap();
        self.processor_status.update_zero_neg_flags(result);
        result
    }

    fn rotate_left(&mut self, val:u8) -> u8 {
        let c = match self.processor_status.carry() {
            false => 0,
            true => 1
        };

        if (val & 0b1000_0000) > 0 {
            self.processor_status.set_carry();
        } else {
            self.processor_status.clr_carry();
        }

        let result = val.checked_shl(1).unwrap() + c;
        self.processor_status.update_zero_neg_flags(result);
        result
    }

    fn rotate_right(&mut self, val:u8) -> u8 {
        let c = match self.processor_status.carry() {
            false => 0,
            true => 0b1000_0000
        };

        if (val & 0b0000_0001) > 0 {
            self.processor_status.set_carry();
        } else {
            self.processor_status.clr_carry();
        }

        let result = val.checked_shr(1).unwrap() + c;
        self.processor_status.update_zero_neg_flags(result);
        result
    }

    // DEBUG Suite:
    pub fn get_pc(&self) -> u16 {
        self.registers.program_counter
    }

    pub fn set_pc(&mut self, addr:u16) {
        self.registers.program_counter = addr;
    }

    pub fn get_x(&self) -> u8 {
        self.registers.x
    }

    pub fn get_y(&self) -> u8 {
        self.registers.y
    }

    pub fn get_a(&self) -> u8 {
        self.registers.accumulator
    }
    
    pub fn get_status(&self) -> u8 {
        self.processor_status.as_byte()
    }

    pub fn get_opcode(&self) -> u8 {
        self.current_opcode as u8
    }

    pub fn get_stack_pointer(&self) -> u8 {
        self.registers.stack_pointer
    }

}


pub(crate) struct Registers {
    program_counter: u16,
    accumulator: u8,
    x: u8,
    y: u8,
    stack_pointer: u8
}
