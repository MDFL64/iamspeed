#![feature(portable_simd)]
#![feature(iter_array_chunks)]

use core::ops::FnOnce;
use std::time::Instant;

fn benchmark<T>(name: &str, f: impl FnOnce() -> T) -> T  {
    let t = Instant::now();
    let res = f();
    println!("bench {name}: {:?}",t.elapsed());
    res
}

pub mod day1 {
    fn parse_int(bytes: &[u8]) -> i32 {
        let a = (bytes[0] - 0x30) as i32 * 10000;
        let b = (bytes[1] - 0x30) as i32 * 1000;
        let c = (bytes[2] - 0x30) as i32 * 100;
        let d = (bytes[3] - 0x30) as i32 * 10;
        let e = (bytes[4] - 0x30) as i32;
        let canon = a + b + c + d + e;
        canon
    }

    struct Common {
        output_first: Vec<i32>,
        output_second: Vec<i32>
    }

    fn common(input: &str) -> Common {
        let mut saved = Common {
            output_first: Vec::with_capacity(1000),
            output_second: Vec::with_capacity(1000)
        };

        crate::benchmark("parse",|| {
            for row in input.bytes().array_chunks::<14>() {
                let num1 = parse_int(&row[0..5]);
                let num2 = parse_int(&row[8..13]);
                //let (num1,num2) = parse_chunk(&row);

                saved.output_first.push(num1);
                saved.output_second.push(num2);
            }
        });

        // sort
        crate::benchmark("sort",|| {
            // faster than std by up to 50%
            radsort::sort(&mut saved.output_first);
            radsort::sort(&mut saved.output_second);
        });

        saved
    }

    pub fn part1(input: &str) -> i32 {
        let saved = common(input);

        let mut sum = 0;
        for (a, b) in saved.output_first.iter().zip(saved.output_second.iter()) {
            sum += (a - b).abs();
        }

        sum
    }

    pub fn part2(input: &str) -> i32 {
        let saved = common(input);

        let mut sum = 0;
        let mut index = 0;

        for a in saved.output_first.iter().copied() {
            let mut count = 0;
            while index < saved.output_second.len() && saved.output_second[index] < a {
                index += 1;
            }
            while index < saved.output_second.len() && saved.output_second[index] == a {
                count += 1;
                index += 1;
            }
            sum += count * a;
        }

        sum
    }
}

pub mod day2 {
    use core::simd::prelude::*;

    pub fn part1(input: &str) -> i32 {
        unsafe { impl1(input) }
    }

    pub fn part2(input: &str) -> i32 {
        unsafe { impl2(input) }
    }

    fn check_fast(array: &[i8;8]) -> (bool,u32) {
        let numbers = i8x8::from_array(*array);
        let numbers_shifted = numbers.rotate_elements_left::<1>();

        let mut numbers_mask = numbers.simd_ne(i8x8::splat(-128));
        let count = numbers_mask.to_bitmask().trailing_ones();

        let deltas = numbers_shifted - numbers;

        let asc_okay = (deltas.simd_le(i8x8::splat(3)) & deltas.simd_gt(i8x8::splat(0))).to_bitmask().trailing_ones();
        let desc_okay = (deltas.simd_ge(i8x8::splat(-3)) & deltas.simd_lt(i8x8::splat(0))).to_bitmask().trailing_ones();

        {
            numbers_mask.set(count as usize - 1, false);
            let real_deltas = numbers_mask.select(deltas,i8x8::splat(0));
            let delta_sum = real_deltas.cast::<i16>().reduce_sum();

            ((asc_okay == count-1) | (desc_okay == count-1), if delta_sum > 0 { asc_okay } else { desc_okay })
        }
        // disabling direction calculation (even via monomorphization) does not have a big impact
        /* else {
            ((asc_okay == count-1) | (desc_okay == count-1), 0)
        }*/
    }

    fn midwit_parse(input: &[u8]) -> ([u8;8],usize) {
        let mut i = 0;
        let mut j = 0;
        let mut n = 0;
        let mut result = [128;8];

        loop {
            let byte = input[i];
            match byte {
                b'0'..=b'9' => {
                    n = n * 10 + (byte - b'0');
                }
                b' ' => {
                    result[j] = n;
                    n = 0;
                    j += 1;
                }
                b'\n' => {
                    result[j] = n;
                    break;
                }
                _ => panic!("char {}",byte as char)
            }
            i += 1;
        }

        (result,i)
    }

    fn fast_parse(input: &[u8]) -> ([u8;8],usize) {
        {
            let newline = u8x32::splat(b'\n');

            let text = if input.len() >= 32 {
                u8x32::from_slice(input)
            } else {
                return midwit_parse(input);
            };

            let len = text.simd_eq(newline).first_set().unwrap();

            let (mask_spaces,mask_final) = match len {
                // having more than 3 fast cases is slower
                /*14 => {
                    // 5x two-digits
                    (0b100100100100,Mask::from_array([true,true,true,true,true,false,false,false]))
                }*/
                17 => {
                    // 6x two-digits
                    (0b100100100100100,Mask::from_array([true,true,true,true,true,true,false,false]))
                }
                20 => {
                    // 7x two-digits
                    (0b100100100100100100,Mask::from_array([true,true,true,true,true,true,true,false]))
                }
                23 => {
                    // 8x two-digits
                    (0b100100100100100100100,Mask::from_array([true,true,true,true,true,true,true,true]))
                }
                _ => {
                    return midwit_parse(input);
                }
            };

            let spaces = text.simd_eq(u8x32::splat(b' ')).to_bitmask();
            if (spaces & mask_spaces) != mask_spaces {
                return midwit_parse(input);
            }

            let digits = text - u8x32::splat(b'0');
            
            let res = simd_swizzle!(digits,[0,3,6,9,12,15,18,21]) * u8x8::splat(10);
            let res = res + simd_swizzle!(digits,[1,4,7,10,13,16,19,22]);

            let res = mask_final.select(res,u8x8::splat(128));

            (res.to_array(),len)
        }
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl1(input: &str) -> i32 {
        let mut bytes = input.as_bytes();

        let mut line_count = 0;

        while bytes.len() > 0 {
            let (nums,len) = fast_parse(bytes);
            
            let (okay,_) = check_fast(&std::mem::transmute(nums));
    
            if okay {
                // once one is good, assume all others are good too
                return 1000 - line_count;
            }

            line_count += 1;

            bytes = &bytes[len+1..];
        }
        0
    }

    fn slice_entry(mut entry: [u8;8], n: usize) -> [u8;8] {
        for i in n..7 {
            entry[i] = entry[i+1];
        }
        entry[7] = 128;
        entry
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl2(input: &str) -> i32 {
        let mut bytes = input.as_bytes();
        let mut count = 0;
        let mut line_count = 0;

        while bytes.len() > 0 {
            let (nums,len) = fast_parse(bytes);
            
            let (okay,fail) = check_fast(&std::mem::transmute(nums));
    
            if okay {
                // once one is good, assume all others are good too
                return 1000 - line_count + count;
            } else {
                // no significant perf impact of moving these behind branches
                let sliced1 = slice_entry(nums,fail as usize);
                let sliced2 = slice_entry(nums,fail as usize + 1);

                let (okay1,_) = check_fast(&std::mem::transmute(sliced1));
                let (okay2,_) = check_fast(&std::mem::transmute(sliced2));
                
                if okay1 | okay2 {
                    count += 1;
                }
            }
            line_count += 1;

            bytes = &bytes[len+1..];
        }

        count
    }
}

pub mod day3 {
    use core::{cmp::Ord, iter::Iterator, simd::prelude::*};

    pub fn part1(input: &str) -> i64 {
        unsafe { impl1(input) }
    }

    pub fn part2(input: &str) -> i64 {
        unsafe { impl2(input) }
    }

    #[inline(always)]
    unsafe fn parse(bytes: &[u8]) -> Option<i64> {
        'fast:
        {
            if bytes.len() >= 8 {
                let vector = u8x8::from_slice(bytes);
                let digit_mask = (vector.simd_ge(u8x8::splat(b'0')) & vector.simd_le(u8x8::splat(b'9'))).to_bitmask();
                let punt_mask = vector.simd_eq(u8x8::from_array(*b"__,,__))")).to_bitmask();
    
                let zero = u8x8::splat(0);
                let digits = vector - u8x8::splat(b'0');
                // 3x3 accounts for the vast majority of cases, with a fair number of 3x2 and 2x3's
                // other cases account for around 1% each
                let digits: u16x8 = match (digit_mask,punt_mask) {
                    (0b1110111,0b10001000) => {                        
                        simd_swizzle!(digits,zero,[0,1,2,4,5,6,10,10]).cast()
                    }
                    (0b111011,0b1000100) => {
                        simd_swizzle!(digits,zero,[10,0,1,3,4,5,10,10]).cast()
                    }
                    (0b110111,0b1001000) => {
                        simd_swizzle!(digits,zero,[0,1,2,10,4,5,10,10]).cast()
                    }
                    _ => break 'fast
                };
    
                let dm = digits * u16x8::from_array([100,10,1,100,10,1,0,0]);

                let dm = dm.as_array();
                let a = dm[0] + dm[1] + dm[2];
                let b = dm[3] + dm[4] + dm[5];
                return Some(a as i64 * b as i64);
            }
        }

        let mut iter = bytes.iter();

        let mut digit_count = 0;
        let mut n1 = 0;
        while let Some(d) = iter.next() {
            match d {
                b'0'..=b'9' => {
                    n1 = n1 * 10 + (d - b'0') as i64;
                    digit_count += 1;
                }
                b',' => break,
                _ => return None
            }
        }

        if digit_count == 0 {
            return None;
        }

        let mut digit_count = 0;
        let mut n2 = 0;
        while let Some(d) = iter.next() {
            match d {
                b'0'..=b'9' => {
                    n2 = n2 * 10 + (d - b'0') as i64;
                    digit_count += 1;
                }
                b')' => break,
                _ => return None
            }
        }

        if digit_count == 0 {
            return None;
        }

        Some(n1*n2)
    }

    #[inline(always)]
    unsafe fn scan4(haystack: &[u8], needle_bytes: [u8;4]) -> Option<usize> {
        const STRIDE: usize = 32;

        let needle = i32x8::splat(i32::from_le_bytes(needle_bytes));

        for offset in (0..).step_by(STRIDE) {
            let chunk0 = &haystack[offset..];
            // not enough bytes, slow path
            if chunk0.len() < STRIDE+3 {
                for (i,window) in chunk0.windows(4).enumerate() {
                    if window == needle_bytes {
                        return Some(offset + i + 4);
                    }
                }
                return None;
            }
            let vector0: i32x8 = std::mem::transmute( u8x32::from_slice(chunk0) );
            let vector1: i32x8 = std::mem::transmute( u8x32::from_slice(&haystack[offset+1..]) );
            let vector2: i32x8 = std::mem::transmute( u8x32::from_slice(&haystack[offset+2..]) );
            let vector3: i32x8 = std::mem::transmute( u8x32::from_slice(&haystack[offset+3..]) );
    
            let offset0 = needle.simd_eq(vector0).to_bitmask().trailing_zeros()*4;
            let offset1 = needle.simd_eq(vector1).to_bitmask().trailing_zeros()*4+1;
            let offset2 = needle.simd_eq(vector2).to_bitmask().trailing_zeros()*4+2;
            let offset3 = needle.simd_eq(vector3).to_bitmask().trailing_zeros()*4+3;
    
            let index = offset0.min(offset1).min(offset2.min(offset3)) as usize;
            
            if index < 256 {
                return Some(offset + index + 4);
            }
        }
        None
    }

    #[inline(always)]
    unsafe fn scan4x2(haystack: &[u8], needle_bytes_1: [u8;4], needle_bytes_2: [u8;4]) -> (Option<usize>,Option<usize>) {
        const STRIDE: usize = 32;

        let needle1 = i32x8::splat(i32::from_le_bytes(needle_bytes_1));
        let needle2 = i32x8::splat(i32::from_le_bytes(needle_bytes_2));

        for offset in (0..).step_by(STRIDE) {
            let chunk0 = &haystack[offset..];
            // not enough bytes, slow path
            if chunk0.len() < STRIDE+3 {
                let mut ret1 = None;
                let mut ret2 = None;

                for (i,window) in chunk0.windows(4).enumerate() {
                    if window == needle_bytes_1 {
                        ret1 = Some(offset + i + 4);
                        break;
                    }
                }

                for (i,window) in chunk0.windows(4).enumerate() {
                    if window == needle_bytes_2 {
                        ret2 = Some(offset + i + 4);
                        break;
                    }
                }

                return (ret1,ret2);
            }
            let vector0: i32x8 = std::mem::transmute( u8x32::from_slice(chunk0) );
            let vector1: i32x8 = std::mem::transmute( u8x32::from_slice(&haystack[offset+1..]) );
            let vector2: i32x8 = std::mem::transmute( u8x32::from_slice(&haystack[offset+2..]) );
            let vector3: i32x8 = std::mem::transmute( u8x32::from_slice(&haystack[offset+3..]) );
    
            let n1_offset0 = needle1.simd_eq(vector0).to_bitmask().trailing_zeros()*4;
            let n1_offset1 = needle1.simd_eq(vector1).to_bitmask().trailing_zeros()*4+1;
            let n1_offset2 = needle1.simd_eq(vector2).to_bitmask().trailing_zeros()*4+2;
            let n1_offset3 = needle1.simd_eq(vector3).to_bitmask().trailing_zeros()*4+3;

            let n2_offset0 = needle2.simd_eq(vector0).to_bitmask().trailing_zeros()*4;
            let n2_offset1 = needle2.simd_eq(vector1).to_bitmask().trailing_zeros()*4+1;
            let n2_offset2 = needle2.simd_eq(vector2).to_bitmask().trailing_zeros()*4+2;
            let n2_offset3 = needle2.simd_eq(vector3).to_bitmask().trailing_zeros()*4+3;
    
            let index_1 = n1_offset0.min(n1_offset1).min(n1_offset2.min(n1_offset3)) as usize;
            let index_2 = n2_offset0.min(n2_offset1).min(n2_offset2.min(n2_offset3)) as usize;
            
            if index_1 < 256 || index_2 < 256 {
                let ret1 = if index_1 < 256 { Some(offset + index_1 + 4) } else { None };
                let ret2 = if index_2 < 256 { Some(offset + index_2 + 4) } else { None };
                return (ret1,ret2);
            }
        }
        (None,None)
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl1(input: &str) -> i64 {
        let mut input = input.as_bytes();

        let mut sum = 0;

        while let Some(index) = scan4(input,*b"mul(") {
            input = &input[index..];
            if let Some(product) = parse( input ) {
                sum += product;
            }
        }

        sum
    }

    unsafe fn impl2(input: &str) -> i64 {
        let mut input = input.as_bytes();

        let mut sum = 0;

        loop {
            let (mul_index,dont_index) = scan4x2(input,*b"mul(",*b"don'");
            match (mul_index,dont_index) {
                (Some(mul_index),Some(_dont_index)) if mul_index < _dont_index => {
                    input = &input[mul_index..];
                    if let Some(product) = parse( input ) {
                        sum += product;
                    }
                }
                (_,Some(dont_index)) => {
                    input = &input[dont_index..];
                    if input.len() > 3 && input[0] == b't' && input[1] == b'(' &&  input[2] == b')' {
                        // skip until do
                        if let Some(index) = scan4(input,*b"do()") {
                            input = &input[index..];
                        } else {
                            break;
                        }
                    }
                }
                (Some(mul_index),None) => {
                    input = &input[mul_index..];
                    if let Some(product) = parse( input ) {
                        sum += product;
                    }
                }
                (None,None) => {
                    break;
                }
            }
        }
        return sum;
    }
}

pub mod day4 {
    // 140 = (32 + 32 + 32 + 32) + (8 + 4)
    // 140 = (64 + 64) + (try 16)

    use core::simd::prelude::*;

    static mut GLOBAL: Globals = Globals{
        x_0: Grid::empty(),
        m_0: Grid::empty(),
        a_0: Grid::empty(),
        s_0: Grid::empty(),
    };

    #[derive(Copy,Clone)]
    // (high word,low word)
    struct Row(u16,u128);

    impl Row {
        fn new(bytes: &[u8], char: u8) -> Self {
            assert!(bytes.len() >= 140);

            let char_splat = u8x32::splat(char);

            let byte_vector = u8x32::from_slice(bytes);
            let mut low = byte_vector.simd_eq(char_splat).to_bitmask() as u128;

            let byte_vector = u8x32::from_slice(&bytes[32..]);
            low |= (byte_vector.simd_eq(char_splat).to_bitmask() as u128) << 32;
            
            let byte_vector = u8x32::from_slice(&bytes[64..]);
            low |= (byte_vector.simd_eq(char_splat).to_bitmask() as u128) << 64;

            let byte_vector = u8x32::from_slice(&bytes[96..]);
            low |= (byte_vector.simd_eq(char_splat).to_bitmask() as u128) << 96;

            let byte_vector = u8x8::from_slice(&bytes[128..]);
            let mut high = byte_vector.simd_eq(u8x8::splat(char)).to_bitmask() as u16;

            let byte_vector = u8x4::from_slice(&bytes[136..]);
            high |= (byte_vector.simd_eq(u8x4::splat(char)).to_bitmask() as u16) << 8;

            Row(high,low)
        }

        fn shift(self, n: u8) -> Self {
            let Row(a,b) = self;
            let carry = (a as u128) << (128 - n);
            Self(a >> n,(b >> n) | carry)
        }

        fn and(self, other: Self) -> Self {
            Self(self.0 & other.0, self.1 & other.1)
        }

        fn count(self) -> i64 {
            (self.1.count_ones() + self.0.count_ones()) as i64
        }

        fn print(self) {
            println!("{:b} {:b}",self.1.reverse_bits(),self.0.reverse_bits());
        }
    }

    struct Globals {
        x_0: Grid,
        m_0: Grid,
        a_0: Grid,
        s_0: Grid
    }

    struct Grid {
        rows: [Row;140]
    }

    impl Grid {
        const fn empty() -> Self {
            let zero = Row(0,0);
            Self{
                rows: [
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                    zero, zero, zero, zero, zero, zero, zero, zero, zero, zero,
                ]
            }
        }
    }

    pub fn part1(input: &str) -> i64 {
        unsafe { impl1(input) }
    }

    pub fn part2(input: &str) -> i64 {
        unsafe { impl2(input) }
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    pub unsafe fn impl1(input: &str) -> i64 {
        let bytes = input.as_bytes();
        for i in 0..140 {
            let byte_row = &bytes[(i*141)..];
            GLOBAL.x_0.rows[i] = Row::new(byte_row, b'X');
            GLOBAL.m_0.rows[i] = Row::new(byte_row, b'M');
            GLOBAL.a_0.rows[i] = Row::new(byte_row, b'A');
            GLOBAL.s_0.rows[i] = Row::new(byte_row, b'S');
        }
        let mut matches = 0;
        // forward and back
        for i in 0..140 {
            let x = GLOBAL.x_0.rows[i];
            let m = GLOBAL.m_0.rows[i].shift(1);
            let a = GLOBAL.a_0.rows[i].shift(2);
            let s = GLOBAL.s_0.rows[i].shift(3);

            matches += ( x.and(m) ).and( a.and(s) ).count();
        }
        for i in 0..140 {
            let x = GLOBAL.x_0.rows[i].shift(3);
            let m = GLOBAL.m_0.rows[i].shift(2);
            let a = GLOBAL.a_0.rows[i].shift(1);
            let s = GLOBAL.s_0.rows[i];

            matches += ( x.and(m) ).and( a.and(s) ).count();
        }
        // up and down
        for i in 0..137 {
            let x = GLOBAL.x_0.rows[i];
            let m = GLOBAL.m_0.rows[i+1];
            let a = GLOBAL.a_0.rows[i+2];
            let s = GLOBAL.s_0.rows[i+3];

            matches += ( x.and(m) ).and( a.and(s) ).count();
        }
        for i in 0..137 {
            let x = GLOBAL.x_0.rows[i+3];
            let m = GLOBAL.m_0.rows[i+2];
            let a = GLOBAL.a_0.rows[i+1];
            let s = GLOBAL.s_0.rows[i];

            matches += ( x.and(m) ).and( a.and(s) ).count();
        }
        // diagonal
        for i in 0..137 {
            let x = GLOBAL.x_0.rows[i];
            let m = GLOBAL.m_0.rows[i+1].shift(1);
            let a = GLOBAL.a_0.rows[i+2].shift(2);
            let s = GLOBAL.s_0.rows[i+3].shift(3);

            matches += ( x.and(m) ).and( a.and(s) ).count();
        }
        for i in 0..137 {
            let x = GLOBAL.x_0.rows[i+3];
            let m = GLOBAL.m_0.rows[i+2].shift(1);
            let a = GLOBAL.a_0.rows[i+1].shift(2);
            let s = GLOBAL.s_0.rows[i].shift(3);

            matches += ( x.and(m) ).and( a.and(s) ).count();
        }
        for i in 0..137 {
            let x = GLOBAL.x_0.rows[i].shift(3);
            let m = GLOBAL.m_0.rows[i+1].shift(2);
            let a = GLOBAL.a_0.rows[i+2].shift(1);
            let s = GLOBAL.s_0.rows[i+3];

            matches += ( x.and(m) ).and( a.and(s) ).count();
        }
        for i in 0..137 {
            let x = GLOBAL.x_0.rows[i+3].shift(3);
            let m = GLOBAL.m_0.rows[i+2].shift(2);
            let a = GLOBAL.a_0.rows[i+1].shift(1);
            let s = GLOBAL.s_0.rows[i];

            matches += ( x.and(m) ).and( a.and(s) ).count();
        }

        matches
    }

    pub unsafe fn impl2(input: &str) -> i64 {
        0
    }
}
