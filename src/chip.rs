use std::fs;
use byteorder::ReadBytesExt;
use byteorder::BigEndian;

pub struct Chip {
	pub screen: [bool; 64*32],
	memory: [u8; 0x1000],
	v: [u8; 0x10],
	i: u16,
	delay_timer: u16,
	sound_timer: u16,
	stack: [u16; 16],
	sp: usize,
	pc: u16,
}

impl Chip {
	pub fn new() -> Self {
		Self {
			screen: [false; 64*32],
			memory: [0; 0x1000],
			v: [0; 0x10],
			i: 0,
			delay_timer: 0,
			sound_timer: 0,
			stack: [0; 16],
			sp: 0,
			pc: 0,
		}
	}

	pub fn load(&mut self, filename: &str) {
        let program = fs::read(filename).expect(filename);
        self.memory.fill(0);
        self.memory[0x200..(0x200 + program.len())].copy_from_slice(&program);
	}

	pub fn reset(&mut self) {
		self.v.fill(0);
		self.i = 0;
		self.delay_timer = 0;
		self.sound_timer = 0;
		self.stack.fill(0);
		self.sp = 0;
		self.pc = 0x200;
	}

	pub fn tick(&mut self) {
		assert!((self.pc % 2) == 0);
		let inst = self.next_instruction();
		// println!("inst: {inst:#X} at {:#X}", self.pc);

		if inst == 0x00E0 { // CLS
			self.screen.fill(false);
		} else if inst == 0x00EE { // RET
			self.sp -= 1;
			self.pc = self.stack[self.sp];
		} else if (inst & 0xF000) == 0x1000 { // JP addr
			self.pc = inst & 0x0FFF;
		} else if (inst & 0xF000) == 0x2000 { // CALL addr
			self.stack[self.sp] = self.pc;
			self.sp += 1;
			self.pc = inst & 0x0FFF;
		} else if (inst & 0xF000) == 0x3000 { // SE Vx, byte
			let x = ((inst >> 8) & 0x0F) as usize;
			let byte = (inst & 0xFF) as u8;
			if self.v[x] == byte {
				self.pc += 2;
			}
		} else if (inst & 0xF000) == 0x4000 { // SNE Vx, byte
			let x = ((inst >> 8) & 0x0F) as usize;
			let byte = (inst & 0xFF) as u8;
			if self.v[x] != byte {
				self.pc += 2;
			}
		} else if (inst & 0xF000) == 0x5000 { // SE Vx, Vy
			let x = ((inst >> 8) & 0x0F) as usize;
			let y = ((inst >> 4) & 0x0F) as usize;
			if self.v[x] == self.v[y] {
				self.pc += 2;
			}
		} else if (inst & 0xF000) == 0x6000 { // LD Vx, byte
			let x = ((inst >> 8) & 0x0F) as usize;
			let byte = (inst & 0xFF) as u8;
			self.v[x] = byte;
		} else if (inst & 0xF000) == 0x7000 { // ADD Vx, byte
			let x = ((inst >> 8) & 0x0F) as usize;
			let byte = (inst & 0xFF) as u8;
			self.v[x] = self.v[x].overflowing_add(byte).0;
		} else if (inst & 0xF000) == 0x8000 {
			let x = ((inst >> 8) & 0x0F) as usize;
			let y = ((inst >> 4) & 0x0F) as usize;
			let op = (inst & 0x0F) as usize;

			match op {
				0x0 => self.v[x] = self.v[y], // LD Vx, Vy
				0x1 => self.v[x] |= self.v[y],// OR Vx, Vy
				0x2 => self.v[x] &= self.v[y],// AND Vx, Vy
				0x3 => self.v[x] ^= self.v[y],// XOR Vx, Vy
				0x4 => {                      // ADD Vx, Vy
					let (val, ov) = self.v[x].overflowing_add(self.v[y]);
					self.v[x] = val;
					self.v[0xF] = if ov { 1 } else { 0 };
				}
				0x5 => {                      // SUB Vx, Vy
					let sub_cmp = self.v[x] >= self.v[y];
					let val = self.v[x].overflowing_sub(self.v[y]).0;
					self.v[x] = val;
					self.v[0xF] = if sub_cmp { 1 } else { 0 };
				}
				0x6 => {                      // SHR Vx
					let carry = self.v[y] & 1;
					self.v[x] = self.v[y] >> 1;
					self.v[0xF] = carry;
				}
				0x7 => {                      // SUBN Vx, Vy
					let sub_cmp = self.v[y] >= self.v[x];
					let val = self.v[y].overflowing_sub(self.v[x]).0;
					self.v[x] = val;
					self.v[0xF] = if sub_cmp { 1 } else { 0 };
				}
				0xE => {                      // SHL Vx
					let carry = (self.v[y] >> 7) & 1;
					self.v[x] = self.v[y] << 1;
					self.v[0xF] = carry;
				}
				_ => todo!(),
			}
		} else if (inst & 0xF000) == 0x9000 { // SNE Vx, Vy
			let x = ((inst >> 8) & 0x0F) as usize;
			let y = ((inst >> 4) & 0x0F) as usize;
			if self.v[x] != self.v[y] {
				self.pc += 2;
			}
		} else if (inst & 0xF000) == 0xA000 { // LD I, addr
			self.i = inst & 0x0FFF;
		} else if (inst & 0xF000) == 0xD000 { // DRW Vx, Vy, nibble
			let x = self.v[((inst >> 8) & 0x0F) as usize] as usize;
			let y = self.v[((inst >> 4) & 0x0F) as usize] as usize;
			let nibble = (inst & 0x0F) as usize;

			for row in 0..nibble {
				let byte = self.memory[self.i as usize + row];
				let offset = (row + y) * 64 + x;
				for col in 0..8 {
					let pixel = if (byte & (1 << (7 - col))) > 0 {
						true
					} else {
						false
					};

					self.screen[offset + col] ^= pixel;
				}
			}
			// TODO: handle vF
		} else if (inst & 0xF0FF) == 0xF01E { // ADD I, Vx
			self.i += self.v[((inst >> 8) & 0x0F) as usize] as u16;
		} else if (inst & 0xF0FF) == 0xF033 { // LD B, Vx
			let x = self.v[((inst >> 8) & 0x0F) as usize];
			let addr = self.i as usize;
			self.memory[addr+0] = x / 100;
			self.memory[addr+1] = (x % 100) / 10;
			self.memory[addr+2] = x % 10;
		} else if (inst & 0xF0FF) == 0xF055 { // LD [I], Vx
			let x = ((inst >> 8) & 0x0F) as usize;
			let addr = self.i as usize;
			for i in 0..(x+1) {
				self.memory[addr + i] = self.v[i];
			}
		} else if (inst & 0xF0FF) == 0xF065 { // LD Vx, [I]
			let x = ((inst >> 8) & 0x0F) as usize;
			let addr = self.i as usize;
			for i in 0..(x+1) {
				self.v[i] = self.memory[addr + i];
			}
		} else {
			panic!("{inst:#X} at addr {:#X}", self.pc);
		}
	}

	fn next_instruction(&mut self) -> u16 {
		let p = self.pc as usize;
		self.pc += 2;
		(&self.memory[p..(p+2)]).read_u16::<BigEndian>().unwrap()
	}
}
