#![feature(portable_simd)]
#![feature(iter_array_chunks)]

use std::time::Instant;

fn benchmark<T>(name: &str, mut f: impl FnMut() -> T)  {
    //let t = Instant::now();
    f();
    //println!("bench {name}: {:?}",t.elapsed());
}

pub mod day1 {
    use core::simd::{num::SimdUint, u16x8, u8x16, u8x8};
    use std::sync::Mutex;

    fn parse_int(bytes: &[u8]) -> i32 {
        let a = (bytes[0] - 0x30) as i32 * 10000;
        let b = (bytes[1] - 0x30) as i32 * 1000;
        let c = (bytes[2] - 0x30) as i32 * 100;
        let d = (bytes[3] - 0x30) as i32 * 10;
        let e = (bytes[4] - 0x30) as i32;
        let canon = a + b + c + d + e;
        canon
    }

    // I could not for the life of me put together a good SIMD parsing routine.
    // Maybe portable SIMD is undercooked. Maybe I am just dumb.
    // Maybe enabling better target features would help, but it's not clear to me whether the test harness will even have those features enabled.
    fn parse_chunk(bytes: &[u8]) -> (i32,i32) {
        let mut array= [0u8;16];
        array[0..14].copy_from_slice(bytes);

        let text = u8x16::from_array(array);
        let sub = u8x16::splat(0x30);
        let digits = text - sub;

        let src = digits.to_array();
        let dst = [src[1],src[2],src[3],src[4],src[9],src[10],src[11],src[12]];

        let shorts: u16x8 = u8x8::from_array(dst).cast();

        let mul1 = u16x8::from_array([1000,100,10,1,0,0,0,0]);
        let mul2 = u16x8::from_array([0,0,0,0,1000,100,10,1]);

        let sum1 = (mul1 * shorts).reduce_sum() as i32 + digits.to_array()[0] as i32 * 10000;
        let sum2 = (mul2 * shorts).reduce_sum() as i32 + digits.to_array()[8] as i32 * 10000;

        (sum1,sum2)
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
            //saved.output_first.sort();
            //saved.output_second.sort();
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
