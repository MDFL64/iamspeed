#![feature(iter_array_chunks)]

pub mod day1 {
    use core::{assert_eq, iter::Iterator};
    use std::time::Instant;

    fn parse_int(bytes: &[u8]) -> i32 {
        let a = (bytes[0] - 0x30) as i32 * 10000;
        let b = (bytes[1] - 0x30) as i32 * 1000;
        let c = (bytes[2] - 0x30) as i32 * 100;
        let d = (bytes[3] - 0x30) as i32 * 10;
        let e = (bytes[4] - 0x30) as i32;
        a + b + c + d + e
    }

    pub fn part1(input: &str) -> i32 {
        let mut first: Vec<i32> = Vec::with_capacity(1000);
        let mut second: Vec<i32> = Vec::with_capacity(1000);

        // parse
        for row in input.bytes().array_chunks::<14>() {
            let num1 = parse_int(&row[0..5]);
            let num2 = parse_int(&row[8..13]);

            first.push(num1);
            second.push(num2);
        }

        // sort
        {
            first.sort();
            second.sort();
        }

        // sum
        let mut sum = 0;
        for (a, b) in first.iter().zip(second) {
            sum += (a - b).abs();
        }

        sum
    }

    pub fn part2(input: &str) -> usize {
        input.len()
    }
}
