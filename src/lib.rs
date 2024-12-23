#![feature(portable_simd)]
#![feature(iter_array_chunks)]
#![feature(strict_overflow_ops)]

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

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    pub unsafe fn impl2(input: &str) -> i64 {
        let bytes = input.as_bytes();
        for i in 0..140 {
            let byte_row = &bytes[(i*141)..];
            GLOBAL.x_0.rows[i] = Row::new(byte_row, b'X');
            GLOBAL.m_0.rows[i] = Row::new(byte_row, b'M');
            GLOBAL.a_0.rows[i] = Row::new(byte_row, b'A');
            GLOBAL.s_0.rows[i] = Row::new(byte_row, b'S');
        }
        let mut matches = 0;
        // direction 1
        for i in 0..138 {
            let a = GLOBAL.a_0.rows[i+1].shift(1);

            let m1 = GLOBAL.m_0.rows[i];
            let m2 = GLOBAL.m_0.rows[i+2];

            let s1 = GLOBAL.s_0.rows[i].shift(2);
            let s2 = GLOBAL.s_0.rows[i+2].shift(2);

            matches += a.and( m1.and(m2) ).and( s1.and(s2) ).count();
        }
        // direction 2
        for i in 0..138 {
            let a = GLOBAL.a_0.rows[i+1].shift(1);

            let m1 = GLOBAL.m_0.rows[i].shift(2);
            let m2 = GLOBAL.m_0.rows[i+2].shift(2);

            let s1 = GLOBAL.s_0.rows[i];
            let s2 = GLOBAL.s_0.rows[i+2];

            matches += a.and( m1.and(m2) ).and( s1.and(s2) ).count();
        }
        // direction 3
        for i in 0..138 {
            let a = GLOBAL.a_0.rows[i+1].shift(1);

            let m1 = GLOBAL.m_0.rows[i];
            let m2 = GLOBAL.m_0.rows[i].shift(2);

            let s1 = GLOBAL.s_0.rows[i+2];
            let s2 = GLOBAL.s_0.rows[i+2].shift(2);

            matches += a.and( m1.and(m2) ).and( s1.and(s2) ).count();
        }
        // direction 4
        for i in 0..138 {
            let a = GLOBAL.a_0.rows[i+1].shift(1);

            let m1 = GLOBAL.m_0.rows[i+2];
            let m2 = GLOBAL.m_0.rows[i+2].shift(2);

            let s1 = GLOBAL.s_0.rows[i];
            let s2 = GLOBAL.s_0.rows[i].shift(2);

            matches += a.and( m1.and(m2) ).and( s1.and(s2) ).count();
        }

        matches
    }
}

pub mod day5 {
    use core::iter::Iterator;

    pub fn part1(input: &str) -> i64 {
        unsafe { impl1(input) }
    }

    pub fn part2(input: &str) -> i64 {
        unsafe { impl2(input) }
    }

    // array bitfields that represent which numbers can NOT come before a given number
    static mut RULES: [u128;100] = [0;100];

    use core::simd::prelude::*;
    use std::arch::x86_64::_mm256_maddubs_epi16;

    #[inline(always)]
    unsafe fn write_rule(n1: u8, n2: u8) {
        RULES[n1 as usize] |= 1<<n2;
    }

    #[inline(always)]
    unsafe fn parse_rules(mut bytes: &[u8]) -> &[u8] {
        while bytes.len() >= 32 {
            let vec = u8x32::from_slice(bytes);
            // check punctuation
            let compare = u8x32::from_array(*b"\n_|__\n\n_|__\n\n_|__\n\n_|__\n\n_|__\n??");
            let eq = vec.simd_eq(compare).to_bitmask();
            if eq != 0b100100100100100100100100100100 {
                break;
            }
            let digits = simd_swizzle!(vec,[
                0,1, 3,4,
                6,7, 9,10,
                12,13, 15,16,
                18,19, 21,22,
                24,25, 27,28,

                0,0,0,0,0,0,0,0,0,0,0,0
            ]) - u8x32::splat(b'0');

            let places = u8x32::from_array([
                10,1,10,1,
                10,1,10,1,
                10,1,10,1,
                10,1,10,1,
                10,1,10,1,

                0,0,0,0,0,0,0,0,0,0,0,0
            ]);

            let res_vec: u16x16 = _mm256_maddubs_epi16(digits.into(),places.into()).into();
            let res = res_vec.to_array();
            write_rule(res[0] as u8, res[1] as u8);
            write_rule(res[2] as u8, res[3] as u8);
            write_rule(res[4] as u8, res[5] as u8);
            write_rule(res[6] as u8, res[7] as u8);
            write_rule(res[8] as u8, res[9] as u8);

            bytes=&bytes[30..];
        }

        loop {
            if bytes[0] == b'\n' {
                return &bytes[1..];
            }
            let n1 = (bytes[0]-b'0')*10 + (bytes[1]-b'0');
            let n2 = (bytes[3]-b'0')*10 + (bytes[4]-b'0');
            write_rule(n1, n2);
    
            bytes=&bytes[6..];
        }
    }

    #[inline(always)]
    unsafe fn parse_line<'a>(mut bytes: &'a[u8], line: &mut [u8;64]) -> (&'a[u8],usize) {
        if bytes.len()==0 {
            return (bytes,0);
        }

        let mut i = 0;
        while bytes.len() >= 32 {
            // parse in up to 10 digit chunks
            let vec = u8x32::from_slice(bytes);

            // determine length, including ending newline
            let newline = vec.simd_eq(u8x32::splat(b'\n')).first_set();

            let places = u8x32::from_array([
                10,1,10,1,
                10,1,10,1,
                10,1,10,1,
                10,1,10,1,
                10,1,10,1,

                10,1,0,0,0,0,0,0,0,0,0,0
            ]);

            let digits1 = simd_swizzle!(vec,[
                0,1, 3,4,
                6,7, 9,10,
                12,13, 15,16,
                18,19, 21,22,
                24,25, 27,28,

                0,0,0,0,0,0,0,0,0,0,0,0
            ]) - u8x32::splat(b'0');
            
            let res_vec: u16x16 = _mm256_maddubs_epi16(digits1.into(),places.into()).into();
            let res_vec8: u8x16 = res_vec.cast();
            let dest = &mut line[i..];
            res_vec8.copy_to_slice(dest);

            match newline {
                None => {
                    i += 10;
                    bytes = &bytes[30..];
                }
                Some(newline) => {
                    let advance = newline+1;
                    i += advance/3;
                    bytes = &bytes[advance..];
                    return (&bytes,i);
                }
            };
        }

        loop {
            let n1 = (bytes[0]-b'0')*10 + (bytes[1]-b'0');
            line[i] = n1;
            i += 1;
            if bytes[2] == b'\n' {
                return (&bytes[3..],i);
            }
            bytes=&bytes[3..];
        }
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl1(input: &str) -> i64 {
        let mut bytes = parse_rules(input.as_bytes());
        let mut line = [0;64];
        let mut count;

        let mut sum = 0;

        'outer:
        loop {
            (bytes,count) = parse_line(bytes,&mut line);
            if count == 0 {
                break;
            }

            let mut seen_mask = 0;
            for n in line.iter().copied().take(count) {
                let rules = RULES[n as usize];
                if rules & seen_mask != 0 {
                    continue 'outer;
                }
                seen_mask |= 1<<n;
            }
            
            let mid = line[count/2];
            sum += mid as i64;
        }

        sum
    }

    #[inline(always)]
    unsafe fn fix(line: &[u8]) -> i64 {
        let mid_i = line.len()/2;

        let mut full_mask = 0;
        for n in line.iter().copied() {
            full_mask |= 1<<n;
        }
        for n in line.iter().copied() {
            let cool_mask = full_mask & RULES[n as usize];
            let index = cool_mask.count_ones();
            if index as usize == mid_i {
                return n as i64;
            }
            full_mask |= 1<<n;
        }
        panic!();
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl2(input: &str) -> i64 {
        let mut bytes = parse_rules(input.as_bytes());
        let mut line = [0;64];
        let mut count;

        let mut sum = 0;

        'outer:
        loop {
            (bytes,count) = parse_line(bytes,&mut line);
            if count == 0 {
                break;
            }

            let mut seen_mask = 0;
            for n in line.iter().copied().take(count) {
                let rules = RULES[n as usize];
                if rules & seen_mask != 0 {
                    sum += fix(&line[..count]);
                    continue 'outer;
                }
                seen_mask |= 1<<n;
            }
        }
        sum
    }
}


pub mod day6 {
    use core::iter::Iterator;
    use std::collections::{HashMap, HashSet};

    use ahash::AHashSet;

    #[derive(Debug,PartialEq,Eq,Hash,Clone,Copy)]
    #[repr(u8)]
    enum Direction {
        North = 1,
        East = 2,
        South = 4,
        West = 8
    }

    const SIZE: usize = 130;
    static mut GRID: [u8;17030] = [0;17030];
    // used to detect cycles in part2
    static mut WALKED: [u8;130*130] = [0;130*130];

    pub fn part1(input: &str) -> i64 {
        unsafe { impl1(input) }
    }

    pub fn part2(input: &str) -> i64 {
        //crate::benchmark("too",|| unsafe { impl2(input) })
        unsafe { impl2(input) }
    }

    unsafe fn mark((x, y): (usize,usize)) {
        GRID[y*(SIZE+1) + x] = b'X';
    }

    unsafe fn is_marked((x, y): (usize,usize)) -> bool {
        GRID[y*(SIZE+1) + x] == b'X'
    }

    unsafe fn check((x, y): (usize,usize)) -> bool {
        GRID[y*(SIZE+1) + x] == b'#'
    }

    unsafe fn set_blocked((x, y): (usize,usize), b: bool) {
        GRID[y*(SIZE+1) + x] = if b { b'#' } else { b'.' }
    }

    unsafe fn clear_walked() {
        std::ptr::write_bytes::<u8>(WALKED.as_mut_ptr(),0,WALKED.len());
    }

    unsafe fn set_walked((x, y): (usize,usize), dir: Direction) -> bool {
        let bit = dir as u8;
        if WALKED[y*130 + x] & bit != 0 {
            true
        } else {
            WALKED[y*130 + x] |= bit;
            false
        }
    }

    fn can_move_north((_, y): (usize,usize)) -> bool {
        y > 0
    }

    fn can_move_south((_, y): (usize,usize)) -> bool {
        y < SIZE-1
    }

    fn can_move_west((x, _): (usize,usize)) -> bool {
        x > 0
    }

    fn can_move_east((x, _): (usize,usize)) -> bool {
        x < SIZE-1
    }

    fn move_north((x, y): (usize,usize)) -> (usize,usize) {
        (x,y-1)
    }

    fn move_south((x, y): (usize,usize)) -> (usize,usize) {
        (x,y+1)
    }

    fn move_west((x, y): (usize,usize)) -> (usize,usize) {
        (x-1,y)
    }

    fn move_east((x, y): (usize,usize)) -> (usize,usize) {
        (x+1,y)
    }

    unsafe fn draw_map() {
        println!();
        println!();
        println!("{}",std::str::from_utf8(&GRID).unwrap());
    }

    unsafe fn final_count() -> i64 {
        let mut count = 0;
        let line = &GRID;
        for b in line.iter().copied() {
            if b == b'X' {
                count += 1;
            }
        }
        count
    }

    unsafe fn part2_base(mut pos: (usize,usize)) -> Vec<((usize,usize),Direction)> {
        let mut result = Vec::with_capacity(10000);

        'outer:
        loop {
            // north
            loop {
                result.push((pos,Direction::North));
                if !can_move_north(pos) {
                    break 'outer;
                }
                let next = move_north(pos);
                if check(next) {
                    break;
                } else {
                    pos = next;
                }
            }
            // east
            loop {
                result.push((pos,Direction::East));
                if !can_move_east(pos) {
                    break 'outer;
                }
                let next = move_east(pos);
                if check(next) {
                    break;
                } else {
                    pos = next;
                }
            }
            // south
            loop {
                result.push((pos,Direction::South));
                if !can_move_south(pos) {
                    break 'outer;
                }
                let next = move_south(pos);
                if check(next) {
                    break;
                } else {
                    pos = next;
                }
            }
            // west
            loop {
                result.push((pos,Direction::West));
                if !can_move_west(pos) {
                    break 'outer;
                }
                let next = move_west(pos);
                if check(next) {
                    break;
                } else {
                    pos = next;
                }
            }
        }

        result
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn part2_check(
        (mut pos,mut dir): ((usize,usize),Direction),
    ) -> bool {
        //walked.clear();
        clear_walked();
        'outer:
        loop {
            // north
            while dir == Direction::North {
                if set_walked(pos, dir) {
                    return true;
                }

                if !can_move_north(pos) {
                    break 'outer;
                }
                let next = move_north(pos);
                if check(next) {
                    dir = Direction::East;
                } else {
                    pos = next;
                }
            }
            // east
            while dir == Direction::East {
                if set_walked(pos, dir) {
                    return true;
                }

                if !can_move_east(pos) {
                    break 'outer;
                }
                let next = move_east(pos);
                if check(next) {
                    dir = Direction::South;
                } else {
                    pos = next;
                }
            }
            // south
            while dir == Direction::South {
                if set_walked(pos, dir) {
                    return true;
                }

                if !can_move_south(pos) {
                    break 'outer;
                }
                let next = move_south(pos);
                if check(next) {
                    dir = Direction::West;
                } else {
                    pos = next;
                }
            }
            // west
            while dir == Direction::West {
                if set_walked(pos, dir) {
                    return true;
                }

                if !can_move_west(pos) {
                    break 'outer;
                }
                let next = move_west(pos);
                if check(next) {
                    dir = Direction::North;
                } else {
                    pos = next;
                }
            }
        }
        false
    }

    fn get_blocking_pos((pos,dir): ((usize,usize),Direction)) -> Option<(usize,usize)> {
        match dir {
            Direction::North => {
                if can_move_north(pos) {
                    Some(move_north(pos))
                } else {
                    None
                }
            }
            Direction::East => {
                if can_move_east(pos) {
                    Some(move_east(pos))
                } else {
                    None
                }
            }
            Direction::South => {
                if can_move_south(pos) {
                    Some(move_south(pos))
                } else {
                    None
                }
            }
            Direction::West => {
                if can_move_west(pos) {
                    Some(move_west(pos))
                } else {
                    None
                }
            }
        }
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl1(input: &str) -> i64 {
        let input = input.as_bytes();
        GRID.copy_from_slice(input);
        
        let start_index = GRID.iter().copied().position(|x| x == b'^').unwrap();
        let mut pos = (start_index%131,start_index/131);

        'outer:
        loop {
            // north
            loop {
                mark(pos);
                if !can_move_north(pos) {
                    break 'outer;
                }
                let next = move_north(pos);
                if check(next) {
                    break;
                } else {
                    pos = next;
                }
            }
            // east
            loop {
                mark(pos);
                if !can_move_east(pos) {
                    break 'outer;
                }
                let next = move_east(pos);
                if check(next) {
                    break;
                } else {
                    pos = next;
                }
            }
            // south
            loop {
                mark(pos);
                if !can_move_south(pos) {
                    break 'outer;
                }
                let next = move_south(pos);
                if check(next) {
                    break;
                } else {
                    pos = next;
                }
            }
            // west
            loop {
                mark(pos);
                if !can_move_west(pos) {
                    break 'outer;
                }
                let next = move_west(pos);
                if check(next) {
                    break;
                } else {
                    pos = next;
                }
            }
        }

        final_count()
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl2(input: &str) -> i64 {
        let input = input.as_bytes();
        GRID.copy_from_slice(input);
        
        let mut looping_blockers = AHashSet::new();

        let start_index = GRID.iter().copied().position(|x| x == b'^').unwrap();
        let start_pos = (start_index%131,start_index/131);

        let path = part2_base(start_pos);
        for path_point in path.iter() {
            if let Some(block_pos) = get_blocking_pos(*path_point) {
                if !check(block_pos) && !looping_blockers.contains(&block_pos) && start_pos != block_pos && !is_marked(block_pos) {
                    set_blocked(block_pos,true);

                    let looped = part2_check(*path_point);
                    if looped {
                        looping_blockers.insert(block_pos);
                    }

                    set_blocked(block_pos,false);
                }
            }
            // disallow blocking already walked paths
            mark(path_point.0);
        }

        looping_blockers.len() as i64
    }
}

// did not participate in day 7

pub mod day8 {
    use core::u8;
    use std::simd::prelude::*;

    use arrayvec::ArrayVec;

    const SIZE: usize = 50;
    static mut TABLE: [ArrayVec<(i8,i8),4> ;256] = [
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
        ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),ArrayVec::new_const(),
    ];
    static mut MAP: [u64;SIZE] = [0;SIZE];

    pub fn part1(input: &str) -> i64 {
        unsafe { impl1(input) }
    }

    pub fn part2(input: &str) -> i64 {
        unsafe { impl2(input) }
    }

    unsafe fn mark(x: i8, y: i8) -> bool {
        if x < 0 || x as usize >= SIZE {
            return false;
        }
        if y < 0 || y as usize >= SIZE {
            return false;
        }
        MAP[y as usize] |= 1<<x;
        true
    }

    unsafe fn check_pair(pos1: (i8,i8), pos2: (i8,i8)) {
        let dx = pos1.0 - pos2.0;
        let dy = pos1.1 - pos2.1;
        mark(pos1.0 + dx, pos1.1 + dy);
        mark(pos2.0 - dx, pos2.1 - dy);
    }

    unsafe fn check_pair_2(pos1: (i8,i8), pos2: (i8,i8)) {
        let dx = pos1.0 - pos2.0;
        let dy = pos1.1 - pos2.1;
        {
            let mut ax = pos1.0;
            let mut ay = pos1.1;
            while mark(ax, ay) {
                ax += dx;
                ay += dy;
            }
        }
        {
            let mut ax = pos2.0;
            let mut ay = pos2.1;
            while mark(ax, ay) {
                ax -= dx;
                ay -= dy;
            }
        }
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl1(input: &str) -> i64 {
        
        // reset table
        for list in TABLE.iter_mut() {
            list.clear();
        }
        for row in MAP.iter_mut() {
            *row = 0;
        }
        
        // fill table
        let mut min = u8::MAX;
        let mut max = u8::MIN;

        let input = input.as_bytes();
        for y in 0..SIZE {
            if y < SIZE-1 {
                let row_index = (SIZE+1)*y;
                let mut mask = u8x64::from_slice(&input[row_index..]).simd_ne(u8x64::splat(b'.')).to_bitmask() & 0x3_FFFF_FFFF_FFFF;
                while mask != 0 {
                    let index = mask.trailing_zeros() as usize;
                    let b = input[row_index + index];

                    TABLE[b as usize].push((index as i8,y as i8));
                    max = max.max(b);
                    min = min.min(b);

                    mask &= !(1<<index);
                }
            } else {
                for x in 0..SIZE {
                    let index = (SIZE+1)*y+x;
                    let b = input[index];
                    if b != b'.' {
                        TABLE[b as usize].push((x as i8,y as i8));
                        max = max.max(b);
                        min = min.min(b);
                    }
                }
            }
        }

        // mark spots
        for i in min..=max {
            let list = &TABLE[i as usize];

            for i in 0..list.len() {
                for j in (i+1)..list.len() {
                    check_pair(list[i],list[j]);
                }
            }
        }

        // count spots
        let mut count = 0;
        for row in MAP.iter() {
            count += row.count_ones();
        }
        count as _
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl2(input: &str) -> i64 {
        
        // reset table
        for list in TABLE.iter_mut() {
            list.clear();
        }
        for row in MAP.iter_mut() {
            *row = 0;
        }
        
        // fill table
        let mut min = u8::MAX;
        let mut max = u8::MIN;

        let input = input.as_bytes();
        for y in 0..SIZE {
            if y < SIZE-1 {
                let row_index = (SIZE+1)*y;
                let mut mask = u8x64::from_slice(&input[row_index..]).simd_ne(u8x64::splat(b'.')).to_bitmask() & 0x3_FFFF_FFFF_FFFF;
                while mask != 0 {
                    let index = mask.trailing_zeros() as usize;
                    let b = input[row_index + index];

                    TABLE[b as usize].push((index as i8,y as i8));
                    //mark(index as i8,y as i8);
                    max = max.max(b);
                    min = min.min(b);

                    mask &= !(1<<index);
                }
            } else {
                for x in 0..SIZE {
                    let index = (SIZE+1)*y+x;
                    let b = input[index];
                    if b != b'.' {
                        TABLE[b as usize].push((x as i8,y as i8));
                        //mark(x as i8,y as i8);
                        max = max.max(b);
                        min = min.min(b);
                    }
                }
            }
        }

        // mark spots
        for i in min..=max {
            let list = &TABLE[i as usize];

            for i in 0..list.len() {
                for j in (i+1)..list.len() {
                    check_pair_2(list[i],list[j]);
                }
            }
        }

        // count spots
        let mut count = 0;
        for row in MAP.iter() {
            count += row.count_ones();
        }
        count as _
    }
}

pub mod day9 {
    use core::{iter::Iterator, u16, usize};

    pub fn part1(input: &str) -> i64 {
        unsafe { impl1(input) }
    }

    pub fn part2(input: &str) -> i64 {
        unsafe { impl2(input) }
    }

    #[derive(Debug,Clone,Copy)]
    struct File {
        id: u32,
        size: u8
    }

    struct Part1<'a> {
        next_front: usize,
        next_back: usize,
        extra_back: File,
        input: &'a [u8],
        next_disk_index: usize,
        sum: i64
    }

    impl<'a> Part1<'a> {
        fn new(input: &'a str) -> Self {
            let input = input.as_bytes();
            let mut next_back = input.len()-1;
            if input[next_back] == b'\n' {
                next_back -= 1;
            }
            if next_back % 2 != 0 {
                next_back -= 1;
            }
            Self {
                next_front: 0,
                next_back,
                extra_back: File{id: 0, size: 0},
                input,
                next_disk_index: 0,
                sum: 0
            }
        }

        fn pull_from_back(&mut self, used_front_index: usize, size_to_fill: u8) -> File {
            // use up extra
            if self.extra_back.size > 0 {
                return if self.extra_back.size > size_to_fill {
                    self.extra_back.size -= size_to_fill;
                    
                    File{
                        id: self.extra_back.id,
                        size: size_to_fill
                    }
                } else {
                    let size = self.extra_back.size;
                    self.extra_back.size = 0;

                    File{
                        id: self.extra_back.id,
                        size
                    }
                };
            }

            // consumed everything
            if self.next_back <= used_front_index {
                return File{
                    id: 0,
                    size: 0
                }
            }

            let count = self.input[self.next_back] - b'0';
            let file_id = self.next_back / 2;
            self.next_back -= 2;
            //assert!(count != 0);

            if count <= size_to_fill {
                return File{
                    id: file_id as u32,
                    size: count
                }
            } else {
                // save extra
                self.extra_back = File{
                    id: file_id as u32,
                    size: count - size_to_fill
                };
                return File{
                    id: file_id as u32,
                    size: size_to_fill
                }
            }
        }

        fn step(&mut self) -> bool {
            if self.next_back < self.next_front {
                // 'next' elements can be the same, but they cannot cross over
                if self.extra_back.size > 0 {
                    self.tally(self.extra_back);
                }
                return false;
            }

            let front_index = self.next_front;
            let is_filled = front_index % 2 == 0;
            let count = self.input[front_index] - b'0';
            self.next_front += 1;

            if is_filled {
                let file_id = front_index / 2;
                self.tally(File {
                    id: file_id as u32,
                    size: count
                });
            } else {
                let mut gap_size = count;
                while gap_size > 0 {
                    let pulled = self.pull_from_back(front_index,gap_size);
                    if pulled.size == 0 {
                        return false;
                    }
                    gap_size -= pulled.size;
                    self.tally(pulled);
                }
            }

            true
        }
    
        fn tally(&mut self, file: File) {
            for _ in 0..file.size {
                self.sum += file.id as i64 * self.next_disk_index as i64;
                self.next_disk_index += 1;
            }
        }
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl1(input: &str) -> i64 {
        let mut part1 = Part1::new(input);
        while part1.step() {}
        part1.sum
    }

    #[derive(Debug)]
    struct File2 {
        id: u16,
        size: u8,
        offset: u32
    }

    #[derive(Debug)]
    struct Gap2 {
        size: u8,
        offset: u32,
        next: u16
    }

    struct GapFinder {
        gaps: Vec<Gap2>,
        chain_starts: [u16;10],
        chain_ends: [u16;10]
    }

    impl GapFinder {
        fn new() -> Self {
            Self {
                gaps: Vec::<Gap2>::with_capacity(11000),
                chain_starts: [u16::MAX;10],
                chain_ends: [u16::MAX;10],
            }
        }

        fn push(&mut self, gap: Gap2) {
            let new_index = self.gaps.len();
            // set chain start if no chain
            if self.chain_starts[gap.size as usize] == u16::MAX {
                self.chain_starts[gap.size as usize] = new_index as u16;
            }
            // link chain end
            if self.chain_ends[gap.size as usize] != u16::MAX {
                let end_index = self.chain_ends[gap.size as usize] as usize;
                self.gaps[end_index].next = new_index as u16;
            }
            self.chain_ends[gap.size as usize] = new_index as u16;

            self.gaps.push(gap);
        }

        // only valid to call for first
        fn remove(&mut self, index: usize) {
            let gap = &self.gaps[index];
            let size = gap.size;
            let next = gap.next;
            self.chain_starts[size as usize] = next;
        }

        fn insert(&mut self, index: usize) {
            let gap = &self.gaps[index];
            let insert_offset = gap.offset;

            let next_index = self.chain_starts[gap.size as usize];
            if next_index == u16::MAX {
                // list is empty
                self.chain_starts[gap.size as usize] = index as u16;
                self.gaps[index].next = next_index;
            } else {
                let check_gap = &self.gaps[next_index as usize];
    
                if insert_offset < check_gap.offset {
                    // insert first
                    self.chain_starts[gap.size as usize] = index as u16;
                    self.gaps[index].next = next_index;
                } else {
                    let mut current_index = next_index;
                    loop {
                        let next_index = self.gaps[current_index as usize].next;
                        if next_index == u16::MAX {
                            // end of list
                            self.gaps[current_index as usize].next = index as u16;
                            self.gaps[index].next = next_index;
                            break;
                        } else {
                            let check_gap = &self.gaps[next_index as usize];
                            if insert_offset < check_gap.offset {
                                // insert
                                self.gaps[current_index as usize].next = index as u16;
                                self.gaps[index].next = next_index;
                                break;
                            }
                        }
                        current_index = next_index;
                    }
                }
            }
        }

        fn next_gap(&mut self, size: u8, max_offset: u32) -> Option<u32> {

            // find gap
            let mut first_index = usize::MAX;
            for check_size in size..10 {
                let index = self.chain_starts[check_size as usize] as usize;
                if index != u16::MAX as usize {
                    first_index = first_index.min(index);
                }
            }

            // bail if none found
            if first_index == usize::MAX {
                return None;
            }

            // bail if offset is too high
            {
                let gap = &self.gaps[first_index];
                if gap.offset > max_offset {
                    return None;
                }
            }

            // remove from current chain
            self.remove(first_index);

            // get result
            {
                let gap = &mut self.gaps[first_index];
                let result = gap.offset;
                gap.size -= size;
                gap.offset += size as u32;
                // re-insert
                if gap.size > 0 {
                    self.insert(first_index);
                }
                Some(result)
            }
        }
    }

    fn tally2(file: &File2) -> i64 {
        let mut sum = 0;
        let mut disk_index = file.offset;
        for _ in 0..file.size {
            sum += file.id as i64 * disk_index as i64;
            disk_index += 1;
        }
        sum
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl2(input: &str) -> i64 {
        let mut input = input.bytes();
        let mut files = Vec::<File2>::with_capacity(11000);
        let mut gaps = GapFinder::new();
        // parse
        {
            let mut next_id = 0;
            let mut next_offset = 0;
            loop {
                // file
                {
                    let Some(b) = input.next() else { break };
                    if b == b'\n' { break };
                    let size = b - b'0';
                    files.push(File2{
                        id: next_id,
                        size,
                        offset: next_offset
                    });
                    next_id += 1;
                    next_offset += size as u32;
                }
                // gap
                {
                    let Some(b) = input.next() else { break };
                    if b == b'\n' { break };
                    let size = b - b'0';
                    if size > 0 {
                        gaps.push(Gap2{
                            size,
                            offset: next_offset,
                            next: u16::MAX
                        });
                    }
                    next_offset += size as u32;
                }
            }
        }
        // process
        {
            let mut sum = 0;
            for file_i in (0..files.len()).rev() {
                let file = &mut files[file_i];
                if let Some(new_offset) = gaps.next_gap(file.size, file.offset) {
                    file.offset = new_offset;
                }
                sum += tally2(file);
            }
            sum
        }
    }
}

pub mod day10 {
    use core::ptr;

    const SIZE: usize = 45;

    pub fn part1(input: &str) -> i64 {
        unsafe { impl1_turbocursed(input) }
    }

    pub fn part2(input: &str) -> i64 {
        unsafe { impl2_turbocursed(input) }
    }
    
    static mut MAP: [u16;SIZE*SIZE] = [0;SIZE*SIZE];

    unsafe fn impl1_turbocursed(input: &str) -> i64 {
        let input = input.as_bytes();

        MAP.fill(0);

        unsafe fn count(input: &[u8], b: u8, (mut x,mut y): (usize,usize), tag: u16) -> i64 {
            use std::arch::asm;
            let mut sum: i64 = 0;
            let mut b = b as i16;
            let mut input_index = input.as_ptr() as usize + y * (SIZE+1) + x;
            let mut map_index = MAP.as_ptr() as usize + (y * SIZE + x) * 2;

            asm!(
                "call 2f",
                "jmp 1002f",

                "2:",
                "mov word ptr [{map_index}], {tag:x}",
                // 9 check
                "cmp {b:x},0x39",
                "je 102f",
                "inc {b}",

                // x+
                "cmp {x},44",
                "jge 3f",
                    "inc {input_index}",
                    // check for char
                    "cmp byte ptr [{input_index}], {b:l}",
                    "jne 4f",
                        "add {map_index}, 2",
                        "cmp word ptr [{map_index}], {tag:x}",
                        "je 5f",
                            "inc {x}",
                            "call 2b",
                            "dec {x}",
                        "5:",
                        "sub {map_index}, 2",
                    "4:",
                    "dec {input_index}",
                "3:",

                // x-
                "cmp {x},0",
                "je 3f",
                    "dec {input_index}",
                    // check for char
                    "cmp byte ptr [{input_index}], {b:l}",
                    "jne 4f",
                        "sub {map_index}, 2",
                        "cmp word ptr [{map_index}], {tag:x}",
                        "je 5f",
                            "dec {x}",
                            "call 2b",
                            "inc {x}",
                        "5:",
                        "add {map_index}, 2",
                    "4:",
                    "inc {input_index}",
                "3:",

                // y+
                "cmp {y},44",
                "jge 3f",
                    "add {input_index}, 46",
                    // check for char
                    "cmp byte ptr [{input_index}], {b:l}",
                    "jne 4f",
                        "add {map_index}, 90",
                        "cmp word ptr [{map_index}], {tag:x}",
                        "je 5f",
                            "inc {y}",
                            "call 2b",
                            "dec {y}",
                        "5:",
                        "sub {map_index}, 90",
                    "4:",
                    "sub {input_index}, 46",
                "3:",

                // y-
                "cmp {y},0",
                "je 3f",
                    "sub {input_index}, 46",
                    // check for char
                    "cmp byte ptr [{input_index}], {b:l}",
                    "jne 4f",
                        "sub {map_index}, 90",
                        "cmp word ptr [{map_index}], {tag:x}",
                        "je 5f",
                            "dec {y}",
                            "call 2b",
                            "inc {y}",
                        "5:",
                        "add {map_index}, 90",
                    "4:",
                    "add {input_index}, 46",
                "3:",

                "dec {b}",
                "ret",

                // handle 9
                "102:",
                    "inc {sum}",
                    "ret",

                "1002:",
                // constants, unchanged when recursing
                tag = in(reg) tag,
                // values (must be pushed or adjusted back to previous value)
                input_index = inout(reg) input_index,
                map_index = inout(reg) map_index,
                b = inout(reg) b,
                x = inout(reg) x,
                y = inout(reg) y,
                // return value
                sum = inout(reg) sum,
            );

            sum
        }

        let mut next_tag = 1;
        let mut sum = 0;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let byte_index = y*(SIZE+1)+x;
                if input[byte_index] == b'0' {
                    let c = count(input,b'0',(x,y),next_tag);
                    sum += c;
                    next_tag += 1;
                }
            }
        }
        sum
    }

    unsafe fn impl2_turbocursed(input: &str) -> i64 {
        let input = input.as_bytes();

        //MAP.fill(0);

        unsafe fn count(input: &[u8], b: u8, (mut x,mut y): (usize,usize)) -> i64 {
            use std::arch::asm;
            let mut sum: i64 = 0;
            let mut b = b as i16;
            let mut input_index = input.as_ptr() as usize + y * (SIZE+1) + x;
            //let mut map_index = MAP.as_ptr() as usize + (y * SIZE + x) * 2;

            asm!(
                "call 2f",
                "jmp 1002f",

                "2:",
                // 9 check
                "cmp {b:x},0x39",
                "je 102f",
                "inc {b}",

                // x+
                "cmp {x},44",
                "jge 3f",
                    "inc {input_index}",
                    // check for char
                    "cmp byte ptr [{input_index}], {b:l}",
                    "jne 4f",
                        "inc {x}",
                        "call 2b",
                        "dec {x}",
                    "4:",
                    "dec {input_index}",
                "3:",

                // x-
                "cmp {x},0",
                "je 3f",
                    "dec {input_index}",
                    // check for char
                    "cmp byte ptr [{input_index}], {b:l}",
                    "jne 4f",
                        "dec {x}",
                        "call 2b",
                        "inc {x}",
                    "4:",
                    "inc {input_index}",
                "3:",

                // y+
                "cmp {y},44",
                "jge 3f",
                    "add {input_index}, 46",
                    // check for char
                    "cmp byte ptr [{input_index}], {b:l}",
                    "jne 4f",
                        "inc {y}",
                        "call 2b",
                        "dec {y}",
                    "4:",
                    "sub {input_index}, 46",
                "3:",

                // y-
                "cmp {y},0",
                "je 3f",
                    "sub {input_index}, 46",
                    // check for char
                    "cmp byte ptr [{input_index}], {b:l}",
                    "jne 4f",
                        "dec {y}",
                        "call 2b",
                        "inc {y}",
                    "4:",
                    "add {input_index}, 46",
                "3:",

                "dec {b}",
                "ret",

                // handle 9
                "102:",
                    "inc {sum}",
                    "ret",

                "1002:",
                // constants, unchanged when recursing
                //tag = in(reg) tag,
                // values (must be pushed or adjusted back to previous value)
                input_index = inout(reg) input_index,
                //map_index = inout(reg) map_index,
                b = inout(reg) b,
                x = inout(reg) x,
                y = inout(reg) y,
                // return value
                sum = inout(reg) sum,
            );

            sum
        }

        let mut sum = 0;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let byte_index = y*(SIZE+1)+x;
                if input[byte_index] == b'0' {
                    let c = count(input,b'0',(x,y));
                    sum += c;
                }
            }
        }
        sum
    }
}

pub mod day11 {
    use arrayvec::ArrayVec;

    static LUT: &[i64;76_000] = unsafe { &std::mem::transmute( *include_bytes!("day11_lut.bin") ) };

    pub fn part1(input: &str) -> i64 {
        unsafe { impl_rec(input, 25) }
    }

    pub fn part2(input: &str) -> i64 {
        unsafe { impl_rec(input, 75) }
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl_rec(input: &str, count: i32) -> i64 {
        let input = input.as_bytes();
        let mut n = 0;
        let mut sum = 0;
        for c in input.iter().copied() {
            match c {
                b'0'..=b'9' => {
                    n *= 10;
                    n += (c-b'0') as i64;
                }
                _ => {
                    sum += solve(n,count);
                    n = 0;
                }
            }
        }
        sum
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn solve(value: i64, n: i32) -> i64 {
        if n==0 {
            return 1;
        } else if value < 1000 {
            let index = value as usize + n as usize * 1000;
            LUT[index]
        } else {
            // trying to get the log of 0 is impossible due to previous checks
            let digits = value.ilog10() + 1;
            
            if digits%2 == 0 {
                let divisor = 10i64.pow(digits/2);

                let a = value / divisor;
                let b = value % divisor;
                solve(a,n-1) + solve(b,n-1)
            } else {
                solve(value * 2024,n-1)
            }
        }
    }

    static mut ARRAY1: ArrayVec<i64,10000> = ArrayVec::new_const();
    static mut ARRAY2: ArrayVec<i64,10000> = ArrayVec::new_const();

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl_flat(input: &str, count: i32) -> i64 {
        let input = input.as_bytes();
        let mut n = 0;
        ARRAY1.clear();
        for c in input.iter().copied() {
            match c {
                b'0'..=b'9' => {
                    n *= 10;
                    n += (c-b'0') as i64;
                }
                _ => {
                    ARRAY1.push(n);
                    n = 0;
                }
            }
        }
        solve_flat(count)
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn solve_flat(mut n: i32) -> i64 {
        let mut input = &mut ARRAY1;
        let mut output = &mut ARRAY2;

        let mut sum = 0;
        while input.len() > 0 {
            //println!("handle {} {:?} {}",n,input,sum);
            output.clear();
            for value in input.iter().copied() {
                if n==0 {
                    sum += 1;
                } else if value < 1000 {
                    let index = value as usize + n as usize * 1000;
                    sum += LUT[index];
                } else {
                    // trying to get the log of 0 is impossible due to previous checks
                    let digits = value.ilog10() + 1;
                    
                    if digits%2 == 0 {
                        let divisor = 10i64.pow(digits/2);
        
                        let a = value / divisor;
                        let b = value % divisor;
                        output.push(a);
                        output.push(b);
                        //solve(a,n-1) + solve(b,n-1)
                    } else {
                        //solve(value * 2024,n-1)
                        output.push(value * 2024);
                    }
                }
            }
            std::mem::swap(&mut input,&mut output);
            n -= 1;
        }

        sum
    }
}

pub mod day12 {
    use core::{iter::Iterator, u16, usize};
    use std::simd::prelude::*;

    use arrayvec::ArrayVec;

    pub fn part1(input: &str) -> i64 {
        unsafe { impl1(input) }
    }

    pub fn part2(input: &str) -> i64 {
        unsafe { impl2(input) }
    }

    static mut LINE_A: ArrayVec<CharSpan,500> = ArrayVec::new_const();
    static mut LINE_B: ArrayVec<CharSpan,500> = ArrayVec::new_const();
    static mut PREV_LINE: &mut ArrayVec<CharSpan,500> = unsafe { &mut LINE_A };
    static mut NEXT_LINE: &mut ArrayVec<CharSpan,500> = unsafe { &mut LINE_B };
    static mut PREV_LINE_INDEX: usize = 0;

    static mut REGIONS: ArrayVec<Region,60_000> = ArrayVec::new_const();

    #[derive(Debug, Clone, Copy)]
    struct CharSpan {
        char: u8,
        start: u16,
        end: u16,
        region: u16
    }

    impl CharSpan {
        pub fn len(&self) -> u16 {
            self.end - self.start
        }

        pub fn overlap(&self, other: &Self) -> u16 {
            let start = self.start.max(other.start);
            let end = self.end.min(other.end);
            assert!(end > start);
            end - start
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct Region {
        area: u16,
        perimeter: u16,
        merged_to: u16
    }

    #[derive(Debug)]
    enum ScanResult {
        Span(CharSpan),
        NewLine,
        End
    }

    struct LineScanner<'a> {
        bytes: &'a [u8],
        index: usize,
        line_start: usize
    }

    // runs down a chain of regions to find the correct id
    unsafe fn get_region(mut id: u16) -> u16 {
        while REGIONS[id as usize].merged_to != u16::MAX {
            id = REGIONS[id as usize].merged_to;
        }
        id
    }

    impl<'a> LineScanner<'a> {
        fn new(input: &'a str) -> Self {
            Self {
                bytes: input.as_bytes(),
                index: 0,
                line_start: 0
            }
        }

        fn next(&mut self) -> ScanResult {
            if let Some(b) = self.bytes.get(self.index) {
                let start_index = self.index;
                self.index += 1;
                if *b == b'\n' {
                    self.line_start = self.index;
                    ScanResult::NewLine
                } else {
                    {
                        const SIMD_WIDTH: usize = 32;
                        loop {
                            let rest = &self.bytes[self.index..];
                            if rest.len() < SIMD_WIDTH {
                                break;
                            }
                            let vec = u8x32::from_slice(rest);
                            let count = vec.simd_eq(u8x32::splat(*b)).to_bitmask().trailing_ones() as usize;
                            self.index += count;
                            if count != SIMD_WIDTH {
                                break;
                            }
                        }
                    }

                    while let Some(b2) = self.bytes.get(self.index) {
                        if b2 != b {
                            break;
                        }
                        self.index += 1;
                    }
                    ScanResult::Span(CharSpan{
                        char: *b,
                        start: (start_index - self.line_start) as u16,
                        end: (self.index - self.line_start) as u16,
                        region: u16::MAX
                    })
                }
            } else {
                ScanResult::End
            }
        }
    }

    struct PrevLineMatcher {
        char: u8,
        start: u16,
        end: u16,
        done: bool
    }

    impl PrevLineMatcher {
        fn new(span: &CharSpan) -> Self {
            Self {
                char: span.char,
                start: span.start,
                end: span.end,
                done: false
            }
        }
    }

    impl Iterator for PrevLineMatcher {
        type Item = CharSpan;

        fn next(&mut self) -> Option<Self::Item> {
            unsafe {
                if self.done {
                    return None;
                }
                while PREV_LINE_INDEX < PREV_LINE.len() {
                    let candidate = PREV_LINE[PREV_LINE_INDEX];
                    if self.char == candidate.char && candidate.start < self.end && self.start < candidate.end {
                        // overlap

                        // we may want to re-use the candidate for another scan, mark done if so
                        if self.end < candidate.end {
                            self.done = true;
                        } else {
                            PREV_LINE_INDEX += 1;
                        }
                        
                        return Some(candidate);
                    }
                    if self.end < candidate.end {
                        break;
                    }
                    if candidate.end <= self.start {
                        break;
                    }
                    PREV_LINE_INDEX += 1;
                }
            }
            None
        }
    }

    unsafe fn impl1(input: &str) -> i64 {
        PREV_LINE.clear();
        NEXT_LINE.clear();
        REGIONS.clear();

        let mut scanner = LineScanner::new(input);
        loop {
            let item = scanner.next();
            match item {
                ScanResult::Span(mut span) => {
                    let mut overlaps = 0;
                    
                    for prev_span in PrevLineMatcher::new(&span) {
                        let prev_region_id = get_region(prev_span.region);
                        if span.region == u16::MAX {
                            // no clash, just update
                            span.region = prev_region_id;
                        } else if prev_region_id != span.region {
                            let region = &mut REGIONS[span.region as usize];
                            let prev_region = &mut REGIONS[prev_region_id as usize];

                            prev_region.merged_to = span.region;
                            region.area += prev_region.area;
                            region.perimeter += prev_region.perimeter;
                        }

                        overlaps += span.overlap(&prev_span);
                    }
                    if span.region == u16::MAX {
                        // new region, no previous regions involved
                        let region_id = REGIONS.len() as u16;
                        span.region = region_id;
                        let span_len = span.len();
                        REGIONS.push(Region{
                            area: span_len,
                            perimeter: span_len * 2 + 2,
                            merged_to: u16::MAX
                        });
                    } else {
                        // update region
                        let region = &mut REGIONS[span.region as usize];
                        let span_len = span.len();
                        region.area += span_len;
                        region.perimeter += span_len * 2 + 2 - overlaps * 2;
                    }

                    NEXT_LINE.push(span);
                }
                ScanResult::NewLine => {
                    // do a flip!
                    std::mem::swap(&mut LINE_A, &mut LINE_B);
                    PREV_LINE_INDEX = 0;
                    NEXT_LINE.clear();
                }
                ScanResult::End => break
            }
        }
        let mut sum = 0;
        for r in REGIONS.iter() {
            //println!("region = {:?}",r);
            if r.merged_to == u16::MAX {
                sum += r.area as i64 * r.perimeter as i64;
            }
        }
        sum
    }

    unsafe fn impl2(input: &str) -> i64 {
        PREV_LINE.clear();
        NEXT_LINE.clear();
        REGIONS.clear();

        let mut scanner = LineScanner::new(input);
        loop {
            let item = scanner.next();
            match item {
                ScanResult::Span(mut span) => {
                    let mut start_eq = false;
                    let mut end_eq = false;
                    
                    for prev_span in PrevLineMatcher::new(&span) {
                        let prev_region_id = get_region(prev_span.region);
                        if span.region == u16::MAX {
                            // no clash, just update
                            span.region = prev_region_id;
                        } else if prev_region_id != span.region {
                            let region = &mut REGIONS[span.region as usize];
                            let prev_region = &mut REGIONS[prev_region_id as usize];

                            prev_region.merged_to = span.region;
                            region.area += prev_region.area;
                            region.perimeter += prev_region.perimeter;
                        }

                        if span.start == prev_span.start {
                            start_eq = true;
                        }
                        if span.end == prev_span.end {
                            end_eq = true;
                        }
                    }
                    if span.region == u16::MAX {
                        // new region, no previous regions involved
                        let region_id = REGIONS.len() as u16;
                        span.region = region_id;
                        let span_len = span.len();
                        REGIONS.push(Region{
                            area: span_len,
                            perimeter: 4,
                            merged_to: u16::MAX
                        });
                    } else {
                        // update region
                        let region = &mut REGIONS[span.region as usize];
                        let span_len = span.len();
                        region.area += span_len;
                        if !start_eq {
                            region.perimeter += 2;
                        }
                        if !end_eq {
                            region.perimeter += 2;
                        }
                    }

                    NEXT_LINE.push(span);
                }
                ScanResult::NewLine => {
                    // do a flip!
                    std::mem::swap(&mut LINE_A, &mut LINE_B);
                    PREV_LINE_INDEX = 0;
                    NEXT_LINE.clear();
                }
                ScanResult::End => break
            }
        }
        let mut sum = 0;
        for r in REGIONS.iter() {
            //println!("region = {:?}",r);
            if r.merged_to == u16::MAX {
                sum += r.area as i64 * r.perimeter as i64;
            }
        }
        sum
    }
}

pub mod day13 {
    use core::simd::prelude::*;
    use std::arch::asm;

    static mut PARSE_LUT: [u8x16;1_000_000] = [u8x16::from_array([
        255,255,255,255,
        255,255,255,255,
        255,255,255,255,
        255,255,255,255,
    ]);1_000_000];

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn part1_fast(input: &[u8]) -> (usize,i64) {
        let mut index = 0;

        const X: u8 = 255;
        const XX: u32 = 255;
        // 3,3
        PARSE_LUT[0b1100000001] =u8x16::from_array([
            X,X,0,1,2,X,X,X,
            X,X,7,8,9,X,X,X
        ]);
        // 3,4
        PARSE_LUT[0b1100000001] =u8x16::from_array([
            X,X,0,1,2,X,X,X,
            X,7,8,9,10,X,X,X
        ]);
        // 3,5
        PARSE_LUT[0b11000000001] =u8x16::from_array([
            X,X,0,1,2,X,X,X,
            7,8,9,10,11,X,X,X
        ]);
        
        // 4,3
        PARSE_LUT[0b1100000010] = u8x16::from_array([
            X,0,1,2,3,X,X,X,
            X,X,8,9,10,X,X,X
        ]);
        // 4,4
        PARSE_LUT[0b11000000010] = u8x16::from_array([
            X,0,1,2,3,X,X,X,
            X,8,9,10,11,X,X,X
        ]);
        // 4,5
        PARSE_LUT[0b110000000010] = u8x16::from_array([
            X,0,1,2,3,X,X,X,
            8,9,10,11,12,X,X,X
        ]);

        // 5,3
        PARSE_LUT[0b11000000100] = u8x16::from_array([
            0,1,2,3,4,X,X,X,
            X,X,9,10,11,X,X,X
        ]);
        // 5,4
        PARSE_LUT[0b110000000100] = u8x16::from_array([
            0,1,2,3,4,X,X,X,
            X,9,10,11,12,X,X,X
        ]);
        // 5,5
        PARSE_LUT[0b1100000000100] = u8x16::from_array([
            0,1,2,3,4,X,X,X,
            9,10,11,12,13,X,X,X
        ]);

        let mut sum = 0;
        // doesn't actually need 64 bytes but I don't want to test my luck
        {
            unsafe {
                let c_0 = u8x32::splat(b'0');
                let c_comma = u8x16::splat(b',');
                let c_newline = u8x16::splat(b'\n');
                const Z: u8 = 0;
                let shuf1 = u8x32::from_array([
                    0,1,X,X,6,7,X,X,
                    X,X,X,X,X,X,X,X,
                    21,22,X,X,27,28,X,X,
                    X,X,X,X,X,X,X,X,
                ]);
                let shuf2 = u32x8::from_array([
                    0,4,2,4,0,2,XX,XX,
                ]);
                let shuf3 = u32x8::from_array([
                    5,1,5,3,3,1,XX,XX,
                ]);
                let mul1 = u8x32::from_array([
                    10,1,Z,Z,10,1,Z,Z,
                    Z,Z,Z,Z,Z,Z,Z,Z,
                    10,1,Z,Z,10,1,Z,Z,
                    Z,Z,Z,Z,Z,Z,Z,Z,
                ]);
                let mul2 = u8x16::from_array([
                    10,1,10,1,1,Z,Z,Z,
                    10,1,10,1,1,Z,Z,Z,
                ]);
                let mul3 = u16x8::from_array([
                    1000,10,1,0,
                    1000,10,1,0,
                ]);
                asm!(
                    "3:",
                    "add {index}, 12",
                    // parse phase 1
                    "vmovups {vec1}, [{input}+{index}]",
                    "vpsubb {vec1}, {vec1}, {c_0}",
                    "vpshufb {vec1}, {vec1}, {shuf1}",
                    "vpmaddubsw {vec1}, {vec1}, {mul1}",
                    // parse phase 2
                    "vmovups {vec2}, [{input}+{index}+39]",
                    "vpcmpeqb {tmp1:x}, {vec2}, {c_comma}",
                    "vpmovmskb {lut_index}, {tmp1:x}",
                    "vpcmpeqb {tmp1:x}, {vec2}, {c_newline}",
                    "vpmovmskb {tmp2}, {tmp1:x}",
                    "or {lut_index}, {tmp2}",
                    "vpsubb {vec2}, {vec2}, {c_0:x}",
                    "vmovaps {tmp1:x}, [{lut}+{lut_index}*2]",
                    "vpshufb {vec2}, {vec2}, {tmp1:x}",
                    // update index
                    "add {index}, 71",
                    "lzcnt {lut_index:e}, {lut_index:e}",
                    "sub {index}, {lut_index}",
                    // yucky
                    "vpmaddubsw {vec2}, {vec2}, {mul2}",
                    "vpmaddwd {vec2}, {vec2}, {mul3}",
                    "phaddd {vec2}, {vec2}",
                    // calculate determinants
                    "vblendps {tmp1}, {vec1}, {vec2:y}, 12",
                    "vpermd {vec1}, {shuf2}, {tmp1}",
                    "vpermd {vec2:y}, {shuf3}, {tmp1}",
                    "vpmulld {vec1}, {vec1}, {vec2:y}",
                    "vpxor {tmp1}, {tmp1}, {tmp1}",
                    "vphsubd {vec1}, {vec1}, {tmp1}",
                    // calculate final
                    "vpextrd {tmp2:e}, {vec1:x}, 0",  // ds
                    "vpextrd eax, {vec1:x}, 1",       // da
                    "cdq",
                    "idiv {tmp2:e}",
                    "test edx, edx",
                    "jnz 2f",
                    "mov {final1:e}, 3",
                    "mul {final1:e}",
                    "movsxd {final1}, eax",

                    "vextractf128 {tmp1:x},{vec1},1",
                    "vpextrd eax, {tmp1:x}, 0",       // db
                    "cdq",
                    "idiv {tmp2:e}",
                    "test edx, edx",
                    "jnz 2f",
                    "movsxd {final2}, eax",

                    "add {sum}, {final1}",
                    "add {sum}, {final2}",

                    "2:",
                    "cmp {index}, {max_len}",
                    "jl 3b",

                    input = in(reg) input.as_ptr(),
                    index = inout(reg) index,
                    lut = in(reg) PARSE_LUT.as_ptr(),
                    vec1 = out(ymm_reg) _,
                    vec2 = out(xmm_reg) _,
                    tmp1 = out(ymm_reg) _,
                    tmp2 = out(reg) _,
                    final1 = out(reg) _,
                    final2 = out(reg) _,
                    lut_index = out(reg) _,
                    sum = inout(reg) sum,
                    c_0 = in(ymm_reg) c_0,
                    c_comma = in(xmm_reg) c_comma,
                    c_newline = in(xmm_reg) c_newline,
                    shuf1 = in(ymm_reg) shuf1,
                    shuf2 = in(ymm_reg) shuf2,
                    shuf3 = in(ymm_reg) shuf3,
                    mul1 = in(ymm_reg) mul1,
                    mul2 = in(xmm_reg) mul2,
                    mul3 = in(xmm_reg) mul3,
                    max_len = in(reg) input.len() - 64,
                    out("rax") _,
                    out("rdx") _,
                );
            }
        }

        (index,sum)
    }

    pub fn part1(input: &str) -> i64 {
        let mut sum;
        let mut index;
        let input = input.as_bytes();
        (index,sum) = unsafe { part1_fast(input) };

        while index < input.len() {
            // parse phase 1
            let chunk = &input[index+12..];
            let ax = ((chunk[0]-b'0')*10 + (chunk[1]-b'0')) as i32;
            let ay = ((chunk[6]-b'0')*10 + (chunk[7]-b'0')) as i32;

            let bx = ((chunk[21]-b'0')*10 + (chunk[22]-b'0')) as i32;
            let by = ((chunk[27]-b'0')*10 + (chunk[28]-b'0')) as i32;

            // parse phase 2
            index += 51;

            let mut sx = 0;
            let mut sy = 0;
            loop {
                let d = input[index];
                index += 1;
                if d < b'0' || d > b'9' {
                    break;
                }
                sx *= 10;
                sx += (d-b'0') as i32;
            }
            loop {
                let d = input[index];
                if d >= b'0' && d <= b'9' {
                    break;
                }
                index += 1;
            }
            loop {
                let d = input[index];
                index += 1;
                if d < b'0' || d > b'9' {
                    break;
                }
                sy *= 10;
                sy += (d-b'0') as i32;
            }
            // skip extra newline
            index += 1;

            let ds = ax*by-bx*ay;
            let da = sx*by-bx*sy;
            let db = ax*sy-sx*ay;
            
            if da % ds != 0 || db % ds != 0 {
                // skip
            } else {
                let a = (da / ds) as i64;
                let b = (db / ds) as i64;
                sum += a*3 + b;
            }
        }

        sum
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn part2_fast(input: &[u8]) -> (usize,i64) {
        let mut index = 0;
        const X: u8 = 255;
        // 3,3
        PARSE_LUT[0b1100000001] =u8x16::from_array([
            X,X,0,1,2,X,X,X,
            X,X,7,8,9,X,X,X
        ]);
        // 3,4
        PARSE_LUT[0b1100000001] =u8x16::from_array([
            X,X,0,1,2,X,X,X,
            X,7,8,9,10,X,X,X
        ]);
        // 3,5
        PARSE_LUT[0b11000000001] =u8x16::from_array([
            X,X,0,1,2,X,X,X,
            7,8,9,10,11,X,X,X
        ]);
        
        // 4,3
        PARSE_LUT[0b1100000010] = u8x16::from_array([
            X,0,1,2,3,X,X,X,
            X,X,8,9,10,X,X,X
        ]);
        // 4,4
        PARSE_LUT[0b11000000010] = u8x16::from_array([
            X,0,1,2,3,X,X,X,
            X,8,9,10,11,X,X,X
        ]);
        // 4,5
        PARSE_LUT[0b110000000010] = u8x16::from_array([
            X,0,1,2,3,X,X,X,
            8,9,10,11,12,X,X,X
        ]);
        // 5,3
        PARSE_LUT[0b11000000100] = u8x16::from_array([
            0,1,2,3,4,X,X,X,
            X,X,9,10,11,X,X,X
        ]);
        // 5,4
        PARSE_LUT[0b110000000100] = u8x16::from_array([
            0,1,2,3,4,X,X,X,
            X,9,10,11,12,X,X,X
        ]);
        // 5,5
        PARSE_LUT[0b1100000000100] = u8x16::from_array([
            0,1,2,3,4,X,X,X,
            9,10,11,12,13,X,X,X
        ]);
        let mut sum = 0;
        // doesn't actually need 64 bytes but I don't want to test my luck
        while input.len() - index > 64 {
            index += 12;
            let mut res1: u16x16;
            let mut res2: u32x4;
            let mut lut_index: u32;
            unsafe {
                let c_0 = u8x32::splat(b'0');
                let c_comma = u8x16::splat(b',');
                let c_newline = u8x16::splat(b'\n');
                const Z: u8 = 0;
                let shuf1 = u8x32::from_array([
                    0,1,6,7,X,X,X,X,
                    X,X,X,X,X,X,X,X,
                    21,22,27,28,X,X,X,X,
                    X,X,X,X,X,X,X,X,
                ]);
                let mul1 = u8x32::from_array([
                    10,1,10,1,Z,Z,Z,Z,
                    Z,Z,Z,Z,Z,Z,Z,Z,
                    10,1,10,1,Z,Z,Z,Z,
                    Z,Z,Z,Z,Z,Z,Z,Z,
                ]);
                let mul2 = u8x16::from_array([
                    10,1,10,1,1,Z,Z,Z,
                    10,1,10,1,1,Z,Z,Z,
                ]);
                let mul3 = u16x8::from_array([
                    1000,10,1,0,
                    1000,10,1,0,
                ]);
                asm!(
                    // parse phase 1
                    "vmovups {vec1}, [{input}]",
                    "vpsubb {vec1}, {vec1}, {c_0}",
                    "vpshufb {vec1}, {vec1}, {shuf1}",
                    "vpmaddubsw {vec1}, {vec1}, {mul1}",
                    // parse phase 2
                    "vmovups {vec2}, [{input}+39]",
                    "vpcmpeqb {tmp1:x}, {vec2}, {c_comma}",
                    "vpmovmskb {lut_index}, {tmp1:x}",
                    "vpcmpeqb {tmp1:x}, {vec2}, {c_newline}",
                    "vpmovmskb {tmp2}, {tmp1:x}",
                    "or {lut_index}, {tmp2}",
                    "vpsubb {vec2}, {vec2}, {c_0:x}",
                    "vmovaps {tmp1:x}, [{lut}+{lut_index}*2]",
                    "vpshufb {vec2}, {vec2}, {tmp1:x}",
                    // yucky
                    "vpmaddubsw {vec2}, {vec2}, {mul2}",
                    "vpmaddwd {vec2}, {vec2}, {mul3}",
                    "phaddd {vec2}, {vec2}",
                    input = in(reg) input.as_ptr().add(index),
                    lut = in(reg) PARSE_LUT.as_ptr(),
                    vec1 = out(ymm_reg) res1,
                    vec2 = out(xmm_reg) res2,
                    tmp1 = out(ymm_reg) _,
                    tmp2 = out(reg) _,
                    lut_index = out(reg) lut_index,
                    c_0 = in(ymm_reg) c_0,
                    c_comma = in(xmm_reg) c_comma,
                    c_newline = in(xmm_reg) c_newline,
                    shuf1 = in(ymm_reg) shuf1,
                    mul1 = in(ymm_reg) mul1,
                    mul2 = in(xmm_reg) mul2,
                    mul3 = in(xmm_reg) mul3,
                );
            }
            let res1 = res1.to_array();
            
            let sx = res2[0] as i64 + 10000000000000;
            let sy = res2[1] as i64 + 10000000000000;
            
            let ax = res1[0] as i64;
            let ay = res1[1] as i64;
            
            let bx = res1[8] as i64;
            let by = res1[9] as i64;
            index += (71 - lut_index.leading_zeros()) as usize;
            let ds = ax*by-bx*ay;
            let da = sx*by-bx*sy;
            let db = ax*sy-sx*ay;
            
            if da % ds != 0 || db % ds != 0 {
                // skip
            } else {
                let a = (da / ds) as i64;
                let b = (db / ds) as i64;
                sum += a*3 + b;
            }
        }
        (index,sum)
    }
    pub fn part2(input: &str) -> i64 {
        let mut sum;
        let mut index;
        let input = input.as_bytes();
        (index,sum) = unsafe { part2_fast(input) };
        while index < input.len() {
            // parse phase 1
            let chunk = &input[index+12..];
            let ax = ((chunk[0]-b'0')*10 + (chunk[1]-b'0')) as i64;
            let ay = ((chunk[6]-b'0')*10 + (chunk[7]-b'0')) as i64;
            let bx = ((chunk[21]-b'0')*10 + (chunk[22]-b'0')) as i64;
            let by = ((chunk[27]-b'0')*10 + (chunk[28]-b'0')) as i64;
            // parse phase 2
            index += 51;
            let mut sx = 0;
            let mut sy = 0;
            loop {
                let d = input[index];
                index += 1;
                if d < b'0' || d > b'9' {
                    break;
                }
                sx *= 10;
                sx += (d-b'0') as i64;
            }
            loop {
                let d = input[index];
                if d >= b'0' && d <= b'9' {
                    break;
                }
                index += 1;
            }
            loop {
                let d = input[index];
                index += 1;
                if d < b'0' || d > b'9' {
                    break;
                }
                sy *= 10;
                sy += (d-b'0') as i64;
            }
            // skip extra newline
            index += 1;
            sx += 10000000000000;
            sy += 10000000000000;

            let ds = ax*by-bx*ay;
            let da = sx*by-bx*sy;
            let db = ax*sy-sx*ay;
            
            if da % ds != 0 || db % ds != 0 {
                // skip
            } else {
                let a = (da / ds) as i64;
                let b = (db / ds) as i64;
                sum += a*3 + b;
            }
        }
        sum
    }
}

// 14-16 skipped

pub mod day17;

pub mod day22 {
    fn prune_mix(a: u32,b: u32) -> u32 {
        (a^b)&16777215
    }
    
    fn do_round(x: u32) -> u32 {
        // phase 1
        let x = prune_mix(x,x<<6);
        // phase 2
        let x = prune_mix(x,x>>5);
        // phase 3
        let x = prune_mix(x,x<<11);
        x
    }

    fn do2k(mut x: u32) -> u32 {
        for _ in 0..2000 {
            x = do_round(x);
        }
        x
    }

    static LUT: &[u32;16777216] = unsafe { &std::mem::transmute( *include_bytes!("day22_lut.bin") ) };
    fn generate_lut() {
        let mut lut = vec!(0;16777216);
        for i in 0..16777216 {
            if i % 100_000 == 0 {
                println!("{}",i);
            }
            lut[i as usize] = do2k(i);
        }
        let bytes: &[u8] = unsafe { std::slice::from_raw_parts(lut.as_ptr() as _,lut.len() * 4) };
        std::fs::write("day22_lut.bin", bytes).unwrap();
        panic!("done");
    }

    static mut MAP: [u16;19*19*19*19] = [0;19*19*19*19];
    static mut SET: [u64;2037] = [0;2037];
    fn set_contains(index: i32) -> bool {
        let cell = index / 64;
        let bit = index % 64;
        unsafe {
            SET[cell as usize] & (1 << bit) != 0
        }
    }

    fn set_insert(index: i32) {
        let cell = index / 64;
        let bit = index % 64;
        unsafe {
            SET[cell as usize] |= (1 << bit);
        }
    }

    fn do2k_part2(mut x: u32) -> u32 {
        let mut last_digit = (x%10) as i32;
        unsafe {
            SET.fill(0);
        }
        let mut hash = 0;
        for i in 0..2000 {
            x = do_round(x);
            let digit = (x%10) as i32;
            let d = digit - last_digit;
            last_digit = digit;
            hash = (hash * 19 + d+9) % (19*19*19*19);
            if i >= 3 {
                if !set_contains(hash) {
                    set_insert(hash);
                    unsafe {
                        MAP[hash as usize] += digit as u16;
                    }
                }
            }
        }
        x
    }

    pub fn part1(input: &str) -> i64 {
        //generate_lut();
        let mut index = 0;
        let input = input.as_bytes();
        let mut n = 0;
        let mut sum = 0;
        while index < input.len() {
            let digit = input[index];
            index += 1;
            if digit == b'\n' {
                sum += LUT[n as usize] as i64;
                n = 0;
            } else {
                n = 10*n + (digit-b'0') as u32;
            }
        }
        sum
    }

    pub fn part2(input: &str) -> i64 {
        let mut index = 0;
        let input = input.as_bytes();
        let mut n = 0;
        unsafe {
            MAP.fill(0);
        }

        while index < input.len() {
            let digit = input[index];
            index += 1;
            if digit == b'\n' {
                do2k_part2(n);
                n = 0;
            } else {
                n = 10*n + (digit-b'0') as u32;
            }
        }
        let max = unsafe { MAP.iter().copied().max() };
        max.unwrap() as i64
    }
}
