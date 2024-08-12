pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const NUM_STACK_FRAMES: usize = 16;
const NUM_INPUT_KEYS: usize = 16;
const PC_START: usize = 0x200;

const FONT_BEGIN_OFFSET: usize = 0x50;
const FONT_NEXT_OFFSET: usize = 0x5;
const FONT_SET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

pub struct Chip8 {
    memory: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],

    // regs
    program_counter: usize, // has to be usize to index into array
    v_regs: [u8; NUM_REGS],
    i_reg: u16,

    // stack
    stack_ptr: usize, // has to be usize to index into array
    stack: [u16; NUM_STACK_FRAMES],

    // timers
    delay_timer: u8,
    sound_timer: u8,

    // input reg
    input: [bool; NUM_INPUT_KEYS],
}

impl Chip8 {
    pub fn new() -> Self {
        let mut newself = Self {
            memory: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            program_counter: PC_START,
            v_regs: [0; NUM_REGS],
            i_reg: 0,
            stack_ptr: 0,
            stack: [0; NUM_STACK_FRAMES],
            input: [false; NUM_INPUT_KEYS],
            delay_timer: 0,
            sound_timer: 0,
        };

        // init fonts
        newself.load_mem(FONT_BEGIN_OFFSET, &FONT_SET);

        return newself;
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        self.load_mem(PC_START, data);
    }

    fn load_mem(&mut self, addr: impl Into<usize> + Copy, data: &[u8]) {
        self.memory[addr.into() ..(addr.into() + data.len())].copy_from_slice(data);
    }

    pub fn get_screen(&mut self) -> &[bool] {
        &self.screen
    }

    // fetch-decode-execute cycle

    pub fn run_cycle(&mut self) {
        let next_op = self.fetch_next_instruction();
        self.execute_instruction(next_op);
        self.tick_timers();
    }

    fn fetch_next_instruction(&mut self) -> u16 {
        let opcode: u16 = 
            (self.get_ram(self.program_counter) as u16) << 8 
            | self.get_ram(self.program_counter + 1) as u16;

        self.incr_pc();

        return opcode;
    }

    fn execute_instruction(&mut self, opcode: u16) {
        // typecast from u16 to u8 here because we are confident that 4 bits fits in a u8
        let nib1: u8 = ((opcode & 0xF000) >> 12) as u8;
        let nib2: u8 = ((opcode & 0x0F00) >> 8) as u8;
        let nib3: u8 = ((opcode & 0x00F0) >> 4) as u8;
        let nib4: u8 = (opcode & 0x000F) as u8;

        match (nib1, nib2, nib3, nib4) {
            (0x0, 0x0, 0xE, 0x0) => self.op_clear_screen(),
            (0x0, 0x0, 0xE, 0xE) => self.op_ret(),

            (0x1, _, _, _) => self.op_jump(opcode & 0xFFF),
            (0xB, _, _, _) => self.op_jump_offset(opcode & 0xFFF),
            (0x2, _, _, _) => self.op_call(opcode & 0xFFF),

            (0x3, _, _, _) => self.op_skip_eq(nib2, (opcode & 0xFF) as u8),
            (0x4, _, _, _) => self.op_skip_neq(nib2, (opcode & 0xFF) as u8),
            (0x5, _, _, 0x0) => self.op_skip_reg_eq(nib2, nib3),
            (0x9, _, _, 0x0) => self.op_skip_reg_neq(nib2, nib3),

            (0x6, _, _, _) => self.op_set_val(nib2, (opcode & 0xFF) as u8),
            
            (0x7, _, _, _) => self.op_incr_reg(nib2, (opcode & 0xFF) as u8),
            
            (0x8, _, _, 0x0) => self.op_set_regs(nib2, nib3),
            (0x8, _, _, 0x1) => self.op_or(nib2, nib3), // or
            (0x8, _, _, 0x2) => self.op_and(nib2, nib3), // and
            (0x8, _, _, 0x3) => self.op_xor(nib2, nib3), // xor
            (0x8, _, _, 0x4) => self.op_add(nib2, nib3), // add
            (0x8, _, _, 0x5) => self.op_sub(nib2, nib3, nib2), // subtract
            (0x8, _, _, 0x7) => self.op_sub(nib3, nib2, nib2), // subtract other way
            (0x8, _, _, 0x6) => todo!(), // shr 
            (0x8, _, _, 0xE) => todo!(), // shl

            (0xA, _, _, _) => self.op_mov_i(opcode & 0xFFF),

            (0xC, _, _, _) => todo!(), // rand

            (0xD, _, _, _) => self.op_draw(nib2, nib3, nib4),

            (0xE, _, 0x9, 0xE) => todo!(), // skip if key
            (0xE, _, 0xA, 0x1) => todo!(), // skip ifn key
            (0xF, _, 0x0, 0xA) => todo!(), // wait key

            (0xF, _, 0x0, 0x7) => todo!(), // get delay timer
            (0xF, _, 0x1, 0x5) => todo!(), // set delay timer
            (0xF, _, 0x1, 0x8) => todo!(), // set sound timer

            (0xF, _, 0x1, 0xE) => todo!(), // addi

            (0xF, _, 0x2, 0x9) => self.ld_font_addr_i(nib2), // get char glyph ptr

            (0xF, _, 0x5, 0x5) => todo!(), // store regs
            (0xF, _, 0x6, 0x5) => todo!(), // ld regs

            (_, _, _, _) => eprintln!("Invalid opcode: {:#04x}", opcode),
        }
    }

    // helper operations

    fn stack_push(&mut self, val: u16) {
        self.stack[self.stack_ptr] = val;
        self.stack_ptr += 1;
    }

    fn stack_pop(&mut self) -> u16 {
        self.stack_ptr -= 1;
        self.stack[self.stack_ptr]
    }

    fn get_reg(&self, reg: impl Into<usize>) -> u8 {
        self.v_regs[reg.into()]
    }

    fn set_reg(&mut self, reg: impl Into<usize>, value: u8) {
        self.v_regs[reg.into()] = value;
    }

    fn set_carry_reg(&mut self) {
        self.v_regs[0xFusize] = 1;
    }

    fn clear_carry_reg(&mut self) {
        self.v_regs[0xFusize] = 0;
    }

    fn get_ram(&mut self, addr: impl Into<usize>) -> u8 {
        self.memory[addr.into()]
    }

    fn set_pc(&mut self, c: impl Into<usize>) {
        self.program_counter = c.into();
    }

    fn incr_pc(&mut self) {
        self.program_counter += 2;
    }

    // input control

    pub fn register_key(&mut self, key: u8, is_pressed: bool) {
        self.input[key as usize] = is_pressed;
    }

    // timer methods

    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
            // plus play sound
        }
    }

    // opcode instructions

    fn op_clear_screen(&mut self) {
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
    }

    fn op_jump(&mut self, addr: u16) {
        self.set_pc(addr);
    }

    fn op_jump_offset(&mut self, addr: u16) {
        self.set_pc(addr + self.get_reg(0usize) as u16);
    }

    fn op_call(&mut self, addr: u16) {
        self.stack_push(
            self.program_counter
                .try_into()
                .expect("PC value cannot exceed 4096."),
        );
        self.set_pc(addr);
    }

    fn op_ret(&mut self) {
        let stack_addr = self.stack_pop();
        self.set_pc(stack_addr);
    }

    fn op_skip_eq(&mut self, register: u8, value: u8) {
        if self.get_reg(register) == value {
            self.incr_pc();
        }
    }

    fn op_skip_neq(&mut self, register: u8, value: u8) {
        if self.get_reg(register) != value {
            self.incr_pc();
        }
    }

    fn op_skip_reg_eq(&mut self, reg_x: u8, reg_y: u8) {
        if self.get_reg(reg_x) == self.get_reg(reg_y) {
            self.incr_pc();
        }
    }

    fn op_skip_reg_neq(&mut self, reg_x: u8, reg_y: u8) {
        if self.get_reg(reg_x) != self.get_reg(reg_y) {
            self.incr_pc();
        }
    }

    fn op_set_regs(&mut self, reg_x: u8, reg_y: u8) {
        self.set_reg(reg_x, self.get_reg(reg_y));
    }

    fn op_mov_i(&mut self, addr: u16) {
        self.i_reg = addr;
    }

    fn op_set_val(&mut self, reg: u8, val: u8) {
        self.set_reg(reg, val);
    }

    fn op_incr_reg(&mut self, reg: u8, val: u8) {
        self.set_reg(reg, self.get_reg(reg) + val);
    }

    fn op_or(&mut self, reg_x: u8, reg_y: u8) {
        self.set_reg(reg_x, self.get_reg(reg_x) | self.get_reg(reg_y));
    }

    fn op_and(&mut self, reg_x: u8, reg_y: u8) {
        self.set_reg(reg_x, self.get_reg(reg_x) & self.get_reg(reg_y));
    }

    fn op_xor(&mut self, reg_x: u8, reg_y: u8) {
        self.set_reg(reg_x, self.get_reg(reg_x) ^ self.get_reg(reg_y));
    }

    fn op_add(&mut self, reg_x: u8, reg_y: u8) {
        // set VF on overflow
        self.clear_carry_reg();

        let x = self.get_reg(reg_x);
        let y = self.get_reg(reg_y);

        match x.checked_add(y) {
            Some(val) => self.set_reg(reg_x, val), // no overflow
            None => {
                self.set_carry_reg();
                self.set_reg(reg_x, x.wrapping_add(y));
            }
        }
    }

    // opcodes 8XY5 and 8XY7
    // - for 8XY5, call op_sub(X, Y, X)
    // - for 8XY7, call op_sub(Y, X, X)
    // given x - y, sets the carry register VF if x > y, and clears if y > x
    fn op_sub(&mut self, reg_minuend: u8, reg_subtrahend: u8, reg_result_store: u8) {
        self.set_carry_reg();
        
        let x = self.get_reg(reg_minuend);
        let y = self.get_reg(reg_subtrahend);

        if y > x {
            self.clear_carry_reg();
            self.set_reg(reg_result_store, x.wrapping_sub(y));
        } else {
            self.set_reg(reg_result_store, x - y);
        }
    }

    // opcodes 8XY6 and 8XYE
    // - for 8XY6, call op_shf(X, Y, 1)
    // - for 8XYE, call op_shf(X, Y, -1)
    fn op_shf(&mut self, reg_x: u8, reg_y: u8, shift: i8) { 
        let y = self.get_reg(reg_y).wrapping_shr(shift as u32);
        self.set_reg(reg_x, y);
    }

    // opcode DXYN
    // - sprites should wrap around the screen
    // - if "on" pixel is overwritten, set VF register
    // - screen buf is stored as an array [bool; 64 * 32] where [x1y1, x2y1...x1y2, x2y2...]
    fn op_draw(&mut self, x_reg: u8, y_reg: u8, n: u8) {
        self.clear_carry_reg();

        for sprite_height in 0..n {
            let y = (self.get_reg(y_reg) + sprite_height) as usize % SCREEN_HEIGHT;

            let addr = self.i_reg + sprite_height as u16;
            let row = self.get_ram(addr);

            for x_offset in 0..8 {
                let x = (self.get_reg(x_reg) + x_offset) as usize % SCREEN_WIDTH;
                
                if (row & (0b1000_0000 >> x_offset)) != 0 {
                    let idx = x + SCREEN_WIDTH * y;

                    // check to increment the collision register
                    if self.screen[idx] {
                        self.set_carry_reg();
                    }
                    
                    self.screen[idx] ^= true;
                }
            }
        }
    }

    // load address of chosen glyph, in register vx, to register i
    // only the lower nibble of vx will be considered
    fn ld_font_addr_i(&mut self, x_reg: u8) {
        // type fuckery - offset should actually be a u16, val should be a u8

        let val: usize = (self.get_reg(x_reg) & 0xF).into();
        let offset: u16 = (FONT_BEGIN_OFFSET + (val * FONT_NEXT_OFFSET)).try_into()
            .expect("Font offset cannot exceed 4096.");
        
        self.i_reg = offset;
    }
}
