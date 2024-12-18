use core::str::Bytes;

use arrayvec::ArrayVec;
use dynasmrt::{dynasm, x64::{Assembler,Rq,}, AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi};

fn read_int(bytes: &mut Bytes) -> i32 {
    let mut n = 0;
    while let Some(c) = bytes.next() {
        match c {
            b'0'..=b'9' => {
                n *= 10;
                n += (c-b'0') as i32;
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

pub fn part1(input: &str) -> &str {
    let mut input = input.bytes();
    let a = read_int(&mut input);
    let b = read_int(&mut input);
    let c = read_int(&mut input);

    // skip newline
    input.next();

    println!();
    println!("{} {} {}",a,b,c);

    let mut output = [0u8;64];

    let mut ops = Assembler::new().unwrap();
    let start = ops.offset();
    dynasm!(ops
        ; .arch x64
        ; push rbx // b
        ; push rdx // c
        ; push rcx // scratch
        ; push rdi // output
        ; mov rax, a
        ; mov rbx, b
        ; mov rdx, c
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
                load_combo(&mut ops, Rq::RCX, arg);
                dynasm!(ops
                    ; .arch x64
                    ; shr rax, cl
                );
            }
            1 => {
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
                load_combo(&mut ops, Rq::RCX, arg);
                dynasm!(ops
                    ; .arch x64
                    ; mov rbx, rax
                    ; shr rbx, cl
                );
            }
            7 => {
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
        ; pop rdi // output
        ; pop rcx // scratch
        ; pop rdx // c
        ; pop rbx // b
        ; ret
    );

    let code = ops.finalize().unwrap();

    let func: extern "C" fn(usize,usize,usize,usize) -> usize = unsafe { std::mem::transmute(code.ptr(start)) };
    let out_addr = output.as_ptr() as usize;
    let final_addr = func(out_addr,out_addr,out_addr,out_addr);

    let count = final_addr - out_addr;
    //println!("{} {} {}",out_addr,final_addr,final_addr - out_addr);
    println!("-> {:?}",&output[..count]);

    "wew"
}

pub fn part2(input: &str) -> &str {
    "wew"
}
