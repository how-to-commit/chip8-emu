pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const NUM_STACK_FRAMES: usize = 16;
const NUM_INPUT_KEYS: usize = 16;
const PC_START: usize = 0x200;

pub struct Machine {
    memory: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],

    // regs
    program_counter: u16,
    v_regs: [u8; NUM_REGS],
    i_reg: u16,

    // stack
    stack_ptr: u16,
    stack: [u16; NUM_STACK_FRAMES],

    // timers
    delay_timer: u8,
    sound_timer: u8,

    // input reg
    input: [bool; NUM_INPUT_KEYS],
}

impl Machine {
    pub fn new() -> Self {
        Self {
            memory: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            program_counter: PC_START as u16,
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
        let opcode: u16 = (self.memory[self.program_counter as usize] as u16) << 8
            | self.memory[(self.program_counter + 1) as usize] as u16;

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
            (0x0, 0x0, 0xE, 0x0) => self.clear_screen(),
            (0x0, 0x0, 0xE, 0xE) => self.ret(),
            (0x1, _, _, _) => self.jump(opcode & 0xFFF),
            (0x2, _, _, _) => self.call(opcode & 0xFFF),
            (0x3, _, _, _) => self.jump_if_equal(nib2, (opcode & 0xFF) as u8),
            (0x4, _, _, _) => self.jump_not_equal(nib2, (opcode & 0xFF) as u8),
            (0x5, _, _, 0x0) => self.jump_regs_equal(nib2, nib3),
            (0x6, _, _, _) => self.assign_reg(nib2, (opcode & 0xFF) as u8),
            (0x7, _, _, _) => self.incr_reg(nib2, (opcode & 0xFF) as u8),
            (0x9, _, _, 0x0) => self.jump_regs_not_equal(nib2, nib3),
            (0x8, _, _, 0x0) => self.set_reg(nib2, nib3),
            (0xA, _, _, _) => self.store_i(opcode & 0xFFF),
            (0xD, _, _, _) => self.draw(nib2, nib3, nib4),
            (_, _, _, _) => eprintln!("Invalid opcode: {:#04x}", opcode),
        }
    }

    // stack operations

    fn stack_push(&mut self, val: u16) {
        self.stack[self.stack_ptr as usize] = val;
        self.stack_ptr += 1;
    }

    fn stack_pop(&mut self) -> u16 {
        self.stack_ptr -= 1;
        self.stack[self.stack_ptr as usize]
    }

    // program counter

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

    fn clear_screen(&mut self) {
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
    }

    fn jump(&mut self, addr: u16) {
        self.program_counter = addr;
    }

    fn call(&mut self, addr: u16) {
        self.stack_push(self.program_counter);
        self.program_counter = addr;
    }

    fn ret(&mut self) {
        self.program_counter = self.stack_pop();
    }

    fn jump_if_equal(&mut self, register: u8, value: u8) {
        if self.v_regs[register as usize] == value {
            self.incr_pc();
        }
    }

    fn jump_not_equal(&mut self, register: u8, value: u8) {
        if self.v_regs[register as usize] != value {
            self.incr_pc();
        }
    }

    fn jump_regs_equal(&mut self, reg_x: u8, reg_y: u8) {
        if self.v_regs[reg_x as usize] == self.v_regs[reg_y as usize] {
            self.incr_pc();
        }
    }

    fn jump_regs_not_equal(&mut self, reg_x: u8, reg_y: u8) {
        if self.v_regs[reg_x as usize] != self.v_regs[reg_y as usize] {
            self.incr_pc();
        }
    }

    fn set_reg(&mut self, reg_x: u8, reg_y: u8) {
        self.v_regs[reg_x as usize] = self.v_regs[reg_y as usize];
    }

    fn store_i(&mut self, addr: u16) {
        self.i_reg = addr;
    }

    fn assign_reg(&mut self, reg: u8, val: u8) {
        self.v_regs[reg as usize] = val;
    }

    fn incr_reg(&mut self, reg: u8, val: u8) {
        self.v_regs[reg as usize] += val;
    }

    // not implemented: collision register
    fn draw(&mut self, x_reg: u8, y_reg: u8, n: u8) {
        for sprite_offset in 0..n {
            let y = (self.v_regs[y_reg as usize] + sprite_offset) as usize;
            let addr = (self.i_reg + sprite_offset as u16) as usize;
            let row = self.memory[addr];

            for x_offset in 0..8 {
                let x = (self.v_regs[x_reg as usize] + x_offset) as usize;
                self.screen[x + SCREEN_WIDTH * y] ^= ((row >> x_offset) & 0x1) != 0;
            }
        }
    }
}
