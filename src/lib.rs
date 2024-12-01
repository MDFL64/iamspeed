#![feature(iter_array_chunks)]

pub mod day1 {
    use core::{assert_eq, iter::Iterator};
    use std::time::Instant;

    struct MapIter<'a> {
        map: &'a mut [u16],
        index: usize,
    }

    impl<'a> MapIter<'a> {
        pub fn new(map: &'a mut [u16]) -> Self {
            Self { map, index: 10000 }
        }

        pub fn next(&mut self) -> Option<usize> {
            loop {
                if self.index >= self.map.len() {
                    return None;
                }

                if self.map[self.index] > 0 {
                    self.map[self.index] -= 1;
                    return Some(self.index);
                }

                self.index += 1;
            }
        }
    }

    unsafe fn parse_int(bytes: &[u8]) -> i32 {
        let a = (bytes[0] - 0x30) as i32 * 10000;
        let b = (bytes[1] - 0x30) as i32 * 1000;
        let c = (bytes[2] - 0x30) as i32 * 100;
        let d = (bytes[3] - 0x30) as i32 * 10;
        let e = (bytes[4] - 0x30) as i32;
        a + b + c + d + e
    }

    pub fn part1(input: &str) -> i32 {
        //let mut first: Vec<i32> = Vec::with_capacity(1000);
        //let mut second: Vec<i32> = Vec::with_capacity(1000);

        let t = Instant::now();
        let mut first: Vec<u16> = vec![0; 100_100];
        let mut second: Vec<u16> = vec![0; 100_100];
        //println!("??? {:?}",t.elapsed());

        // parse
        for row in input.bytes().array_chunks::<14>() {
            let num1 = unsafe { parse_int(&row[0..5]) };
            let num2 = unsafe { parse_int(&row[8..13]) };

            first[num1 as usize] += 1;
            second[num2 as usize] += 1;
        }

        let mut map_1 = MapIter::new(&mut first);
        let mut map_2 = MapIter::new(&mut second);

        /*let mut sumx = 0;
        for i in first {
            sumx += i;
        }*/

        //1258579
        // sort
        /*{
            first.sort();
            second.sort();
        }*/

        // sum
        let mut sum = 0;
        loop {
            let Some(a) = map_1.next() else { break };
            let Some(b) = map_2.next() else { break };

            sum += (a as i32 - b as i32).abs();
        }

        sum
    }

    pub fn part2(input: &str) -> usize {
        input.len()
    }
}
