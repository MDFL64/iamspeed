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
