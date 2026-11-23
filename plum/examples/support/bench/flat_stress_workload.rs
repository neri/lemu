use plum::flat::FlatUME;
use plum::gpr::PartialRegister;
use plum::prelude::*;

pub struct Stress {
    pub cpu: FlatUME,
    functions: u32,
    separate: u32,
    mixed: u32,
    immediate: u32,
}

impl Stress {
    pub fn new(functions: u32) -> Self {
        assert!((1..=8192).contains(&functions));
        let main_len = 2 + functions as usize * 7 + 9;
        let function_start = main_len.next_multiple_of(4096);
        let end = function_start + functions as usize * 16;
        let separate = end.next_multiple_of(4096);
        let mut memory = vec![0x90; separate + 8192];
        let mut main = vec![0x31, 0xdb]; // XOR EBX,EBX
        for i in 0..functions as usize {
            let target = function_start + i * 16;
            let next = main.len() + 5;
            main.push(0xe8);
            main.extend((target as i32 - next as i32).to_le_bytes());
            main.extend([0x01, 0xc3]); // ADD EBX,EAX
            memory[target..target + 6].copy_from_slice(&[0xb8, 1, 0, 0, 0, 0xc3]);
        }
        main.extend([0x89, 0xd8, 0xcd, 0x40, 0xe9]); // MOV EAX,EBX; INT40; JMP start
        main.extend((-(main.len() as i32 + 4)).to_le_bytes());
        assert_eq!(main.len(), main_len);
        memory[..main.len()].copy_from_slice(&main);
        let len = memory.len() as u32;
        let cpu = plum::ume::flat(
            memory.into_boxed_slice(),
            Offset32(0),
            Offset32(0),
            Offset32(len),
        )
        .unwrap();
        let mut stress = Self {
            cpu,
            functions,
            separate: separate as u32,
            mixed: end as u32 - 4,
            immediate: end as u32 - 15,
        };
        stress.execute(functions);
        // Include the loop-back jump in the warm cache too.
        stress.execute(functions);
        stress
    }

    fn execute(&mut self, expected: u32) -> u32 {
        assert!(matches!(self.cpu.execute(), Err(Exception::Swi(0x40))));
        let value = self.cpu.state().eax().read();
        assert_eq!(value, expected);
        self.cpu.resume_next();
        value
    }

    pub fn step(&mut self, mode: u32, value: u32) -> u32 {
        let expected = match mode {
            0 => {
                self.cpu.write_memory32(self.separate, value).unwrap();
                self.functions
            }
            1 => {
                // Unexecuted padding, on a decoded function's tracked page.
                self.cpu.write_memory32(self.mixed, value).unwrap();
                self.functions
            }
            2 => {
                self.cpu.write_memory32(self.immediate, value).unwrap();
                self.functions - 1 + value
            }
            _ => panic!("unknown write mode"),
        };
        self.execute(expected)
    }
}
