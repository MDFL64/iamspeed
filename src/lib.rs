#![feature(portable_simd)]
#![feature(iter_array_chunks)]

use core::ops::FnOnce;
use std::{collections::HashMap, sync::Mutex, time::Instant};

fn benchmark<T>(name: &str, f: impl FnOnce() -> T) -> T  {
    //let t = Instant::now();
    let res = f();
    //println!("bench {name}: {:?}",t.elapsed());
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
    //use core::cmp::Ord;

    use core::{iter::Iterator, simd::prelude::*};

    pub fn part1(input: &str) -> i32 {
        unsafe { impl1(input) }
    }

    pub fn part2(input: &str) -> i32 {
        unsafe { impl2(input) }
    }

    fn check_line(line: &str) -> bool {
        let mut iter = line.split(' ').map(|x| x.parse::<i32>().unwrap());
        let mut last = iter.next().unwrap();

        let n = iter.next().unwrap();

        if (n-last).abs() > 3 {
            return false;
        }

        if n > last {
            // up
            last = n;
            for n in iter {
                let delta = n - last;
                if delta < 1 || delta > 3 {
                    return false;
                }
                last = n;
            }
        } else if n < last {
            // down
            last = n;
            for n in iter {
                let delta = last - n;
                if delta < 1 || delta > 3 {
                    return false;
                }
                last = n;
            }
        } else {
            return false;
        }

        true
    }

    fn check_array(array: &[i8;8]) -> bool {
        let mut iter = array.iter().copied().filter(|x| *x != -1);
        let mut last = iter.next().unwrap();

        let n = iter.next().unwrap();

        if (n-last).abs() > 3 {
            return false;
        }

        if n > last {
            // up
            last = n;
            for n in iter {
                let delta = n - last;
                if delta < 1 || delta > 3 {
                    return false;
                }
                last = n;
            }
        } else if n < last {
            // down
            last = n;
            for n in iter {
                let delta = last - n;
                if delta < 1 || delta > 3 {
                    return false;
                }
                last = n;
            }
        } else {
            return false;
        }

        true
    }

    fn check_fast<const FIND_DIR: bool>(array: &[i8;8]) -> (bool,u32) {
        let numbers = i8x8::from_array(*array);
        let numbers_shifted = numbers.rotate_elements_left::<1>();

        let mut numbers_mask = numbers.simd_ne(i8x8::splat(-128));
        let count = numbers_mask.to_bitmask().trailing_ones();

        let deltas = numbers_shifted - numbers;

        let asc_okay = (deltas.simd_le(i8x8::splat(3)) & deltas.simd_gt(i8x8::splat(0))).to_bitmask().trailing_ones();
        let desc_okay = (deltas.simd_ge(i8x8::splat(-3)) & deltas.simd_lt(i8x8::splat(0))).to_bitmask().trailing_ones();

        if FIND_DIR {
            numbers_mask.set(count as usize - 1, false);
            let real_deltas = numbers_mask.select(deltas,i8x8::splat(0));
            let delta_sum = real_deltas.cast::<i16>().reduce_sum();

            ((asc_okay == count-1) | (desc_okay == count-1), if delta_sum > 0 { asc_okay } else { desc_okay })
        } else {
            ((asc_okay == count-1) | (desc_okay == count-1), 0)
        }
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
            
            let (okay,_) = check_fast::<false>(&std::mem::transmute(nums));
    
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
            
            let (okay,fail) = check_fast::<true>(&std::mem::transmute(nums));
    
            if okay {
                // once one is good, assume all others are good too
                return 1000 - line_count + count;
            } else {
                // no significant perf impact of moving these behind branches
                let sliced1 = slice_entry(nums,fail as usize);
                let sliced2 = slice_entry(nums,fail as usize + 1);

                let (okay1,_) = check_fast::<false>(&std::mem::transmute(sliced1));
                let (okay2,_) = check_fast::<false>(&std::mem::transmute(sliced2));
                
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
