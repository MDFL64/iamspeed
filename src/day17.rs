use core::str::Bytes;

use arrayvec::ArrayVec;
use dynasmrt::{dynasm, x64::{Assembler,Rq,}, AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi};
use rand::prelude::*;

fn read_int(bytes: &mut Bytes) -> i64 {
    let mut n = 0;
    while let Some(c) = bytes.next() {
        match c {
            b'0'..=b'9' => {
                n *= 10;
                n += (c-b'0') as i64;
            }
            b'\n' => break,
            _ => (),
        }
    }
    n
}

fn read_instr(bytes: &mut Bytes) -> Option<(u8,u8)> {
    let a = loop {
        let Some(c) = bytes.next() else { return None };
        match c {
            b'0'..=b'9' => {
                break c - b'0';
            }
            b'\n' => return None,
            _ => (),
        }
    };

    let b = loop {
        let Some(c) = bytes.next() else { return None };
        match c {
            b'0'..=b'9' => {
                break c - b'0';
            }
            b'\n' => return None,
            _ => (),
        }
    };

    Some((a,b))
}

fn load_combo(ops: &mut Assembler, destination: Rq, combo: u8) {
    match combo {
        0..=3 => {
            dynasm!(ops; .arch x64; mov Rq(destination as u8), combo as i32)
        }
        4 => if destination != Rq::RAX { dynasm!(ops; .arch x64; mov Rq(destination as u8), rax) } // a
        5 => if destination != Rq::RBX { dynasm!(ops; .arch x64; mov Rq(destination as u8), rbx) } // b
        6 => if destination != Rq::RDX { dynasm!(ops; .arch x64; mov Rq(destination as u8), rdx) } // c
        _ => panic!("todo instr {}",combo)
    }
}

static mut FINAL_OUT: [u8;1024] = [0;1024];

pub fn part1(input: &str) -> &str {
    let mut input = input.bytes();
    let a = read_int(&mut input);
    let b = read_int(&mut input);
    let c = read_int(&mut input);

    // skip newline
    input.next();

    let mut output = [0u8;64];

    let mut ops = Assembler::new().unwrap();
    let start = ops.offset();
    dynasm!(ops
        ; .arch x64
        ; push rbx
        ; mov rax, rsi
        ; mov rbx, 0
        ; mov rdx, 0
    );

    let mut offsets = ArrayVec::<DynamicLabel,64>::new();

    while let Some((instr,arg)) = read_instr(&mut input) {
        let start = ops.new_dynamic_label();
        offsets.push(start);
        offsets.push(start);
        dynasm!(ops
            ; .arch x64
            ; =>start
        );
        match instr {
            0 => {
                // divide a
                load_combo(&mut ops, Rq::RCX, arg);
                dynasm!(ops
                    ; .arch x64
                    ; shr rax, cl
                );
            }
            1 => {
                // b = b ^ combo
                dynasm!(ops
                    ; .arch x64
                    ; xor rbx, arg as i32
                )
            }
            2 => {
                // b = combo % 8
                load_combo(&mut ops, Rq::RBX, arg);
                dynasm!(ops
                    ; .arch x64
                    ; and rbx, 7
                )
            }
            3 => {
                // jump if a != 0
                let target = offsets[arg as usize];
                dynasm!(ops
                    ; .arch x64
                    ; test rax, rax
                    ; jnz =>target
                )
            }
            4 => {
                // b = b ^ c
                dynasm!(ops
                    ; .arch x64
                    ; xor rbx, rdx
                )
            }
            5 => {
                // out combo % 8
                load_combo(&mut ops, Rq::RCX, arg);
                dynasm!(ops
                    ; .arch x64
                    ; and rcx, 7
                    ; mov [rdi], rcx
                    ; inc rdi
                )
            }
            6 => {
                // divide b
                load_combo(&mut ops, Rq::RCX, arg);
                dynasm!(ops
                    ; .arch x64
                    ; mov rbx, rax
                    ; shr rbx, cl
                );
            }
            7 => {
                // divide c
                load_combo(&mut ops, Rq::RCX, arg);
                dynasm!(ops
                    ; .arch x64
                    ; mov rdx, rax
                    ; shr rdx, cl
                );
            }
            _ => panic!("todo instr {}",instr)
        }
    };

    dynasm!(ops
        ; .arch x64
        ; mov rax, rdi
        ; pop rbx
        ; ret
    );

    let code = ops.finalize().unwrap();

    let func: extern "C" fn(&mut [u8;64],i64) -> usize = unsafe { std::mem::transmute(code.ptr(start)) };
    let out_addr = output.as_ptr() as usize;
    let final_addr = func(&mut output,a);

    let count = final_addr - out_addr;

    unsafe {
        for index in 0..count {
            if index > 0 {
                FINAL_OUT[index*2-1] = b',';
            }
            FINAL_OUT[index*2] = output[index]+b'0';
        }
        std::str::from_utf8(&FINAL_OUT[..count*2-1]).unwrap()
    }
}
