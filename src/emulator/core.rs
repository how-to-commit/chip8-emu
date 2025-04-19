use super::fontset::{FONTSET, FONTSET_SIZE};
use super::state::{ProgramState, Screen, TimerState};

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4 * 1024;
const NUM_V_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;
const START_ADDR: usize = 0x200;

pub struct Chip8 {
    program_counter: usize,
    memory: [u8; RAM_SIZE],

    v_regs: [u8; NUM_V_REGS],
    i_reg: u16,
    stack: [u16; STACK_SIZE],
    stack_pointer: usize,

    screen: Screen,
    keys: [bool; NUM_KEYS],

    delay_timer: u8,
    sound_timer: u8,

    // not part of the chip8 spec, just for use in this emulator
    _finished: bool,
}

impl Chip8 {
    pub fn new() -> Self {
        let mut new = Self {
            program_counter: START_ADDR,
            memory: [0; RAM_SIZE],
            v_regs: [0; NUM_V_REGS],
            i_reg: 0,
            stack: [0; STACK_SIZE],
            stack_pointer: 0,
            screen: Screen::new(),
            keys: [false; NUM_KEYS],
            delay_timer: 0,
            sound_timer: 0,

            _finished: false,
        };
        new.copy_fontset();
        new
    }

    pub fn copy_fontset(&mut self) {
        self.memory[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    /// call to progress the emulator
    pub fn tick(&mut self) -> ProgramState {
        if self._finished || self.program_counter > RAM_SIZE - 2 {
            return ProgramState::Finished;
        }

        let higher = self.memory[self.program_counter] as u16;
        let lower = self.memory[self.program_counter + 1] as u16;
        let op = higher << 8 | lower;
        self.exec_op(op);

        return match self.checked_pc_increment(2usize) {
            Err(_) => ProgramState::Finished,
            Ok(_) => ProgramState::Running,
        };
    }

    /// call once per frame, returns whether to play sound or not
    pub fn tick_timers(&mut self) -> TimerState {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                return TimerState::PlaySound;
            }
            self.sound_timer -= 1;
        }

        TimerState::None
    }

    fn checked_pc_set<T>(&mut self, val: T) -> Result<(), ()>
    where
        T: Into<usize>,
    {
        self.program_counter = val.into();
        if self.program_counter > RAM_SIZE - 2 {
            self._finished = true;
            return Err(());
        } else {
            return Ok(());
        }
    }

    fn checked_pc_increment<T>(&mut self, val: T) -> Result<(), ()>
    where
        T: Into<usize>,
    {
        self.checked_pc_set(self.program_counter + val.into())
    }

    fn checked_pc_decrement<T>(&mut self, val: T) -> Result<(), ()>
    where
        T: Into<usize>,
    {
        self.checked_pc_set(self.program_counter - val.into())
    }

    fn get_reg<T>(&mut self, reg: T) -> u8
    where
        T: Into<usize>,
    {
        self.v_regs[reg.into()]
    }

    fn set_reg<T>(&mut self, reg: T, val: u8)
    where
        T: Into<usize>,
    {
        self.v_regs[reg.into()] = val
    }

    fn incr_reg<T>(&mut self, reg: T, val: u8)
    where
        T: Into<usize> + Copy,
    {
        let current_value = self.get_reg(reg);
        self.set_reg(reg, current_value + val);
    }

    fn stack_push(&mut self, val: u16) {
        self.stack[self.stack_pointer] = val;
        self.stack_pointer += 1;
    }

    fn stack_pop(&mut self) -> u16 {
        self.stack_pointer -= 1;
        self.stack[self.stack_pointer]
    }

    fn exec_op(&mut self, op: u16) {
        let nib1 = (op & 0xF000) >> 12;
        let nib2 = (op & 0x0F00) >> 8;
        let nib3 = (op & 0x00F0) >> 4;
        let nib4 = op & 0x000F;

        match (nib1, nib2, nib3, nib4) {
            (0x0, 0x0, 0x0, 0x0) => return,
            (0x0, 0x0, 0xE, 0x0) => self.screen.reset(),
            (0x0, 0x0, 0xE, 0xE) => {
                // ret
                let return_addr = self.stack_pop();
                let _ = self.checked_pc_set(return_addr);
            }
            (0x1, _, _, _) => {
                // 1NNN: jump to addr NNN
                let _ = self.checked_pc_set(op & 0xFFF);
            }
            (0x2, _, _, _) => {
                // 2NNN: call procedure at addr NNN
                self.stack_push(
                    self.program_counter
                        .try_into()
                        .expect("program counter cannot be more than memory size"),
                );
                let _ = self.checked_pc_set(op & 0xFFF);
            }
            (0x3, _, _, _) => {
                // 3XNN: skip if reg X value == NN
                self.op_skip_if(nib2, op & 0xFF, true);
            }
            (0x4, _, _, _) => {
                // 4XNN: skip if reg X value != NN
                self.op_skip_if(nib2, op & 0xFF, false);
            }
            (0x5, _, _, 0x0) => {
                // 5XY0: skip if reg X value == reg Y value
                let reg2_val = self.get_reg(nib3);
                self.op_skip_if(nib2, reg2_val.into(), true);
            }
            (0x9, _, _, 0x0) => {
                // 9XY0: skip if reg X value != reg Y value
                let reg2_val = self.get_reg(nib3);
                self.op_skip_if(nib2, reg2_val.into(), false);
            }
            (0x6, _, _, _) => {
                // 6XNN: set value in reg X to NN
                self.set_reg(nib2, (op & 0xFF).try_into().expect("1 byte"));
            }
            (0x7, _, _, _) => {
                // 7XNN: increment reg X by NN
                self.incr_reg(nib2, (op & 0xFF).try_into().expect("1 byte"));
            }
            (0x8, _, _, 0x0) => {
                // 8XY0: set reg X value to reg Y value
                let reg_y_value = self.get_reg(nib3);
                self.set_reg(nib2, reg_y_value);
            }
            (0x8, _, _, 0x1) => {
                // 8XY1: reg X value OR reg Y value, stored in X
                let yval = self.get_reg(nib3);
                let xval = self.get_reg(nib2);
                self.set_reg(nib2, yval | xval);
            }
            (0x8, _, _, 0x2) => {
                // 8XY2: reg X value AND reg Y value, stored in X
                let yval = self.get_reg(nib3);
                let xval = self.get_reg(nib2);
                self.set_reg(nib2, yval & xval);
            }
            (0x8, _, _, 0x3) => {
                // 8XY3: reg X value XOR reg Y value, stored in X
                let yval = self.get_reg(nib3);
                let xval = self.get_reg(nib2);
                self.set_reg(nib2, yval ^ xval);
            }
            (0x8, _, _, 0x4) => {
                // 8XY4: add reg Y value to reg X
                // the carry flag VF is set if the result is greater than 8 bits
                let yval = self.get_reg(nib3);
                let xval = self.get_reg(nib2);
                let (new_x, carry) = xval.overflowing_add(yval);

                self.set_reg(nib2, new_x);
                self.set_reg(0xFusize, if carry { 1 } else { 0 });
            }
            (0x8, _, _, 0x5) => {
                // 8XY5: subtract reg Y value from reg X
                // the borrow flag VF is set if no underflow occurs
                let yval = self.get_reg(nib3);
                let xval = self.get_reg(nib2);
                let (new_x, borrow) = xval.overflowing_sub(yval);

                self.set_reg(nib2, new_x);
                self.set_reg(0xFusize, if borrow { 0 } else { 1 });
            }
            (0x8, _, _, 0x7) => {
                // 8XY7: subtract X value from Y value, then store result in X
                // the borrow flag VF is set if no underflow occurs
                // this is 8XY5 flipped around
                let yval = self.get_reg(nib3);
                let xval = self.get_reg(nib2);
                let (new_x, borrow) = yval.overflowing_sub(xval);

                self.set_reg(nib2, new_x);
                self.set_reg(0xFusize, if borrow { 0 } else { 1 });
            }
            (0x8, _, _, 0x6) => {
                // 8XY6: shift reg X value by 1 to the right
                // the flag VF is set to the dropped bit
                let value = self.get_reg(nib2);

                self.set_reg(nib2, value >> 1);
                self.set_reg(0xFusize, value & 1);
            }
            (0x8, _, _, 0xE) => {
                // 8XYE: shift reg X value by 1 to the left
                // the flag VF is set to the dropped bit
                let value = self.get_reg(nib2);

                self.set_reg(nib2, value << 1);
                self.set_reg(0xFusize, (value >> 7) & 1);
            }
            (0xA, _, _, _) => {
                // ANNN: set reg I to NNN
                self.i_reg = op & 0xFFF;
            }
            (0xB, _, _, _) => {
                // BNNN: jump to V0 + NNN
                let addr = u16::from(self.get_reg(0usize)) + (op & 0xFFF);
                let _ = self.checked_pc_set(addr);
            }
            (0xC, _, _, _) => {
                // CXNN: set X to random AND NN
                let r: u8 = rand::random();
                let r2 = r & (op & 0xFF) as u8;
                self.set_reg(nib2, r2)
            }
            (0xD, _, _, _) => {
                // DXYN: draw sprite at I with height N to coordinates X, Y
                let sprite_x = self.get_reg(nib2);
                let sprite_y = self.get_reg(nib3);
                let sprite_height = nib4 as u8; // truncate to fit u8, 1 nibble < 8 bits

                let mut pixels_flipped = false;
                for y_line in 0..sprite_height {
                    let addr = self.i_reg + u16::from(y_line);
                    let pixels = self.memory[usize::from(addr)];

                    for x_line in 0..8 {
                        let current_pixel = (pixels & (0b1000_0000 >> x_line)) != 0;

                        pixels_flipped |= self.screen.set_pixel(
                            sprite_x + x_line,
                            sprite_y + y_line,
                            current_pixel,
                        );
                    }
                }

                if pixels_flipped {
                    self.set_reg(0xFusize, 1);
                } else {
                    self.set_reg(0xFusize, 0);
                }
            }
            (0xE, _, 0x9, 0xE) => {
                // EX9E: skip if key id in VX is pressed
                let vx = self.get_reg(nib2);
                if self.keys[usize::from(vx)] {
                    let _ = self.checked_pc_increment(2usize);
                }
            }
            (0xE, _, 0xA, 0x1) => {
                // EXA1: skip if key id in VX is NOT pressed
                let vx = self.get_reg(nib2);
                if !self.keys[usize::from(vx)] {
                    let _ = self.checked_pc_increment(2usize);
                }
            }
            (0xF, _, 0x0, 0xA) => {
                // FX0A: wait for keypress
                let mut pressed = false;
                for (id, is_pressed) in self.keys.iter().enumerate() {
                    if *is_pressed {
                        pressed = true;
                        self.set_reg(nib2, id as u8);
                        break;
                    }
                }
                // block execution if not pressed
                if !pressed {
                    let _ = self.checked_pc_decrement(2usize);
                }
            }
            (0xF, _, 0x0, 0x7) => {
                // FX07: set VX to value in DT
                self.set_reg(nib2, self.delay_timer);
            }
            (0xF, _, 0x1, 0x5) => {
                // FX15: set DT to value in VX
                self.delay_timer = self.get_reg(nib2);
            }
            (0xF, _, 0x1, 0x8) => {
                // FX18: set ST to value in VX
                self.sound_timer = self.get_reg(nib2);
            }
            (0xF, _, 0x1, 0xE) => {
                // FX1E: increment I reg with value in VX
                self.i_reg = self.i_reg.wrapping_add(self.get_reg(nib2).into());
            }
            (0xF, _, 0x2, 0x9) => {
                // FX29: set I to font address of character in vx
                self.i_reg = u16::from(self.get_reg(nib2)) * 5;
            }
            (0xF, _, 0x3, 0x3) => {
                // FX33: set mem @ [I..I+3) (3 bytes) to binary-coded decimal of value in VX
                let vx = self.get_reg(nib2);

                self.memory[usize::from(self.i_reg)] = vx / 100; // hundreds
                self.memory[usize::from(self.i_reg)] = (vx / 10) % 10; // tens
                self.memory[usize::from(self.i_reg)] = vx % 10; // ones
            }
            (0xF, _, 0x5, 0x5) => {
                // FX55: store value of registers from V0 to Vx into memory @ I
                for idx in 0..=nib2 {
                    self.memory[usize::from(self.i_reg) + usize::from(idx)] =
                        self.v_regs[usize::from(idx)];
                }
            }
            (0xF, _, 0x6, 0x5) => {
                // FX65: load registers V0 to Vx from memory @ I
                for idx in 0..=nib2 {
                    self.v_regs[usize::from(idx)] =
                        self.memory[usize::from(self.i_reg) + usize::from(idx)];
                }
            }
            (_, _, _, _) => unimplemented!(),
        }
    }

    #[inline]
    pub fn op_skip_if(&mut self, v_reg: u16, val: u16, eq: bool) {
        if eq ^ (u16::from(self.v_regs[usize::from(v_reg)]) != val) {
            self.program_counter += 2;
        }
    }
}
