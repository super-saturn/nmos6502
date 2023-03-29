# NMOS 6502 in Rust

## Another 6502 Emulator?

Yes! But wait, here's why you may want this one:

- `no_std` compatible
- Minimal dependencies (only `num_enum` as a preprocessor for Opcodes)
- Works in both Big Endian and Little Endian environments
- Passes the [Klaus2m5 Functional Test Suite for 6502 Processors](https://github.com/Klaus2m5/6502_65C02_functional_tests)
- Supports `IRQ` and `NMI` interrupts
- Tested on various systems including Mac, PC and Embedded RP2040
- Accurate cycle count exposed after each instruction

This implementation covers all standard opcodes for the NMOS 6502 and all the "illegal" NOP equivalents. Unrecognized opcodes are exposed for debugging purposes and will be implemented at a later time.


## How to Use

This library is *only* the CPU. In a 6502 system the CPU is always in charge of the current address of the bus.

The CPU send and receives data via a `BusInterface`, which the crate user must implement themselves. At its most rudimentary, an implementation could simply allocate a blank 64k array of `u8` and return/write the indexed value.

BusInterface must fundamentally provide:

```
fn get_byte_at(&mut self, addr:u16) -> u8;
fn set_byte_at(&mut self, addr:u16, byte: u8);
```


In more complex systems, eg., an Apple ][ emulator, you may implement whatever clever system you like to intercept/distribute these addresses to various subsystems.