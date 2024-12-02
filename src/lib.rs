#![feature(portable_simd)]
#![feature(iter_array_chunks)]

use std::time::Instant;

fn benchmark<T>(name: &str, mut f: impl FnMut() -> T)  {
    //let t = Instant::now();
    f();
    //println!("bench {name}: {:?}",t.elapsed());
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

    use core::{iter::Iterator, simd::prelude::*, u64};

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
        let mut iter = array.iter().copied().filter(|x| *x != 0);
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

    fn midwit_parse(input: &str) -> ([u8;8],usize) {
        let bytes = input.as_bytes();
        let mut i = 0;
        let mut j = 0;
        let mut n = 0;
        let mut result = [0u8;8];

        loop {
            let byte = bytes[i];
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

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn fast_parse(input: &str) -> ([u8;8],usize) {
        {
            let newline = u8x32::splat(b'\n');

            let text = if input.len() >= 32 {
                u8x32::from_slice(input.as_bytes())
            } else {
                u8x32::load_or(input.as_bytes(), newline)
            };

            let space = u8x32::splat(b' ');
            let ascii_zero = u8x32::splat(b'0');
            let zero = u8x32::splat(0);
            let ten = u8x32::splat(10);

            // zero out invalid elements from the next line
            let len = text.simd_eq(newline).first_set().unwrap();
            let valid = Mask::from_bitmask(!(u64::MAX << len));
            let text = valid.select(text,zero);

            // find end of each number
            let spaces = text.simd_eq(space);
            let mut one_places = Mask::from_bitmask(spaces.to_bitmask() >> 1);
            one_places.set(len-1, true);
            let ten_places = Mask::from_bitmask(one_places.to_bitmask() >> 1) & !spaces;

            // produce parsed bytes
            let digits = text - ascii_zero;
            let tens = ten_places.select(digits * ten, zero);
            let ones = one_places.select(digits, zero);

            // remove gaps
            let gappy = (tens.rotate_elements_right::<1>() + ones).to_array();
            let mut i = 0;
            let mut result = [0u8;8];

            for x in gappy {
                if x != 0 {
                    result[i] = x;
                    i += 1;
                }
            }

            (result, len)
        }
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl1(mut input: &str) -> i32 {

        let mut count = 0;

        while input.len() > 0 {
            let (nums,len) = fast_parse(input);
            if check_array(&std::mem::transmute(nums)) {
                count += 1;
            }
            input = &input[len+1..];
        }

        /*for line in input.lines() {
            if check_line(line) {
                count += 1;
            }
        }*/

        count
    }

    #[target_feature(enable = "avx2,bmi1,bmi2,cmpxchg16b,lzcnt,movbe,popcnt")]
    unsafe fn impl2(input: &str) -> i32 {
        1
    }
}
