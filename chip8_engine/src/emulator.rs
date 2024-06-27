pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const NUM_STACK_FRAMES: usize = 16;
const NUM_INPUT_KEYS: usize = 16;
const PC_START: usize = 0x200;

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
        Self {
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
        }
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        self.memory[PC_START..(PC_START + data.len())].copy_from_slice(data);
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
            (0x8, _, _, 0x0) => self.op_mov_reg(nib2, nib3),
            (0xA, _, _, _) => self.op_mov_i(opcode & 0xFFF),

            (0xD, _, _, _) => self.op_draw(nib2, nib3, nib4),

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

    fn op_mov_reg(&mut self, reg_x: u8, reg_y: u8) {
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

    fn op_draw(&mut self, x_reg: u8, y_reg: u8, n: u8) {
        self.set_reg(0xFusize, 0);

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
                        self.set_reg(0xFusize, 1);
                    }
                    
                    self.screen[idx] ^= true;
                }
            }
        }
    }
}
