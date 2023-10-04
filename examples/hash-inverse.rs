#![allow(dead_code, unused_variables, unreachable_code)]

use pthash_rs::{
    hash::{Hash, Hasher, MulHash},
    reduce::{Reduce, FR64},
};

use std::{
    cmp::min,
    collections::{
        hash_map::Entry::{self},
        HashMap,
    },
};

use itertools::Itertools;
use rand::random;

type T = u64;
const C: T = MulHash::C as T;

fn find_diffs_bruteforce(c: T) {
    // Find multiples of C with r leading zeros:
    for r in 1..T::BITS {
        eprintln!("r = {}", r);
        let mut last = 0;
        let mut diffs = HashMap::new();
        let mut add_diff = |d| {
            if d == 0 {
                return false;
            }
            match diffs.entry(d) {
                Entry::Occupied(mut e) => *e.get_mut() += 1,
                Entry::Vacant(e) => {
                    e.insert(1);
                }
            }
            diffs.len() == 3
        };
        for i in 0..=T::MAX {
            let ci = c.wrapping_mul(i);
            if ci.leading_zeros() >= r {
                let diff = i - last;
                last = i;
                if add_diff(diff) {
                    break;
                }
            }
        }
        let mut diffs = diffs.into_iter().collect_vec();
        diffs.sort();
        for (diff, count) in diffs {
            eprintln!("{:10}: {:10}", diff, count);
        }
    }
}

/// Find the possible difference for a given `r` and `c`.
fn next_possible_diffs(c: T, r: u32, prev_diffs: &Vec<T>) -> Vec<T> {
    let mut possible_diffs = vec![];
    match prev_diffs.len() {
        1 => {
            for i in 1..100 {
                possible_diffs.push(prev_diffs[0] * i);
            }
        }
        2 => {
            for i in 0..100 {
                possible_diffs.push(prev_diffs[0] + i * prev_diffs[1]);
                possible_diffs.push(i * prev_diffs[0] + prev_diffs[1]);
            }
        }
        3 => {
            for i in 0..100 {
                possible_diffs.push(prev_diffs[0] + i * prev_diffs[1]);
                possible_diffs.push(i * prev_diffs[0] + prev_diffs[1]);

                possible_diffs.push(prev_diffs[0] + i * prev_diffs[2]);
                possible_diffs.push(i * prev_diffs[0] + prev_diffs[2]);

                possible_diffs.push(prev_diffs[1] + i * prev_diffs[2]);
                possible_diffs.push(i * prev_diffs[1] + prev_diffs[2]);
            }
        }
        _ => panic!(),
    }
    possible_diffs.sort();
    possible_diffs.dedup();
    if possible_diffs[0] == 0 {
        possible_diffs.remove(0);
    }

    let mut last = 0;
    let mut diffs = vec![];
    'l: while diffs.len() < 3 {
        for &d in &possible_diffs {
            if (c * (last + d)).leading_zeros() >= r {
                if !diffs.contains(&d) {
                    // eprintln!("{r}: new diff {d:10}");
                    diffs.push(d);

                    // Once we have found 2 possible differences, the last difference must always be either their sum of difference.
                    if diffs.len() == 2 {
                        possible_diffs = vec![
                            diffs[0],
                            diffs[1],
                            diffs[0] + diffs[1],
                            T::abs_diff(diffs[0], diffs[1]),
                        ];
                        possible_diffs.sort();
                        possible_diffs.dedup();
                    }
                }
                last += d;
                if last == 0 {
                    // eprintln!("WRAPPED; stopping");
                    break 'l;
                }
                continue 'l;
            }
        }
        panic!();
    }
    diffs.sort();
    diffs
}

/// Find the possible difference for a given `c` for all r from 0 to T::BITS.
fn find_diffs(c: T) -> Vec<Vec<T>> {
    let mut diffs = vec![vec![1]];
    for r in 1..T::BITS {
        diffs.push(next_possible_diffs(c, r, diffs.last().unwrap()));
    }
    diffs
}

/// Solve min_k { C*k = X ^ A : 0 <= A < 2^{64-r} } in $O(k)$.
fn find_inverse_bruteforce(x: T, r: u32) -> T {
    for k in 0.. {
        if (C.wrapping_mul(k) ^ x).leading_zeros() >= r {
            return k;
        }
    }
    panic!()
}

/// Solve min_k { C*k = X ^ A : 0 <= A < 2^{64-r} } in $O(r)$.
fn find_inverse_fast(x: T, r: u32, diffs: &Vec<Vec<T>>) -> T {
    let mut k = 0;
    let mut rr = (C.wrapping_mul(k) ^ x).leading_zeros();
    'rr: while rr < r {
        for &d in &diffs[rr as usize] {
            let new_rr = (C.wrapping_mul(k + d) ^ x).leading_zeros();
            if new_rr >= rr {
                k += d;
                rr = new_rr;
                // eprintln!(
                //     "k+={d:20} = {k:20}: {:064b} {:064b}  {rr:>2}",
                //     C.wrapping_mul(k),
                //     C.wrapping_mul(k) ^ X
                // );
                continue 'rr;
            }
        }
        unreachable!();
    }
    k
}

/// Solve min_k { C*k = X ^ A : 0 <= A < 2^{64-r} } in $O(r)$.
fn find_inverse_fast_with_test(
    x: T,
    r: u32,
    test: impl Fn(T) -> bool,
    mut k: u64,
    diffs: &Vec<Vec<T>>,
) -> T {
    let mut rr = (C.wrapping_mul(k) ^ x).leading_zeros();
    'rr: while rr < r {
        for &d in &diffs[rr as usize] {
            let new_rr = (C.wrapping_mul(k + d) ^ x).leading_zeros();
            if new_rr >= rr {
                k += d;
                rr = new_rr;
                // eprintln!(
                //     "k+={d:20} = {k:20}: {:064b} {:064b}  {rr:>2}",
                //     C.wrapping_mul(k),
                //     C.wrapping_mul(k) ^ x
                // );
                continue 'rr;
            }
        }
        unreachable!();
    }
    'l: loop {
        if test(k) {
            return k;
        }
        if r == T::BITS {
            return T::MAX;
        }
        for &d in &diffs[r as usize] {
            let new_r = (C.wrapping_mul(k + d) ^ x).leading_zeros();
            if new_r >= r {
                k += d;
                // eprintln!(
                //     "k+={d:20} = {k:20}: {:064b} {:064b}  {rr:>2}",
                //     C.wrapping_mul(k),
                //     C.wrapping_mul(k) ^ x
                // );
                continue 'l;
            }
        }
        panic!();
    }
}

/// Solve Reduce(hx ^ MH(k), n) == p by trying k = 0.. .
fn invert_fr64_bruteforce(hx: Hash, n: usize, p: usize) -> u64 {
    let r = FR64::new(n);
    for k in 0u64.. {
        if r.reduce(hx ^ MulHash::hash(&k, 0)) == p {
            return k;
        }
    }
    panic!()
}

/// Solve FastReduce(x ^ MH(k), n) == p efficiently.
fn invert_fr64_fast(x: Hash, n: usize, p: usize, diffs: &Vec<Vec<T>>) -> u64 {
    // low = 2^64 * p/n <= x^FR(k) < 2^64 * (p+1)/n = high+1
    let low = ((1u128 << 64) * p as u128 / n as u128) as u64;
    // high is an inclusive bound:  (2^64 * (p+1) - 1)/n
    let high = (((1u128 << 64) * (p + 1) as u128 - 1) / n as u128 - 1) as u64;

    // In this case the partitioning into two intervals doesn't work.
    if low == high {
        return find_inverse_fast_with_test(
            low ^ x.get(),
            64,
            |k| {
                let xck = x.get() ^ C.wrapping_mul(k);
                low <= xck && xck < high
            },
            0,
            diffs,
        );
    }

    let lcp = (low ^ high).leading_zeros();

    // let k0 = find_inverse_fast(low ^ x.get(), lcp, diffs);
    let k0 = 0;

    // Split [low, high) into two pieces that have (much) longer LCP.

    let low_end = low | ((1u64 << (63 - lcp)) - 1);
    let high_start = low_end + 1;

    let low_lcp = (low ^ low_end).leading_zeros();
    let high_lcp = (high_start ^ high).leading_zeros();

    // eprintln!("low                                             {low:064b}");
    // eprintln!("low_end                                         {low_end:064b}");
    // eprintln!("high_start                                      {high_start:064b}");
    // eprintln!("high                                            {high:064b}");
    // eprintln!(
    // "x                                               {:064b}",
    // x.get()
    // );
    // eprintln!();

    // eprintln!(
    // "low^x                                           {:064b}",
    // low ^ x.get()
    // );
    let low_k = find_inverse_fast_with_test(
        low ^ x.get(),
        low_lcp,
        |k| {
            let xck = x.get() ^ C.wrapping_mul(k);
            low <= xck && xck < high
        },
        k0,
        diffs,
    );
    // eprintln!("low_k      {low_k:>10}");

    // eprintln!(
    // "high^x                                          {:064b}",
    // high ^ x.get()
    // );
    let high_k = find_inverse_fast_with_test(
        high_start ^ x.get(),
        high_lcp,
        |k| low <= x.get() ^ C.wrapping_mul(k) && x.get() ^ C.wrapping_mul(k) < high,
        k0,
        diffs,
    );
    // eprintln!("high_k     {high_k:>10}");
    min(low_k, high_k)
}

fn find_inverse_statistics() {
    let diffs = &find_diffs(C);

    const B: u32 = T::BITS + 1;
    let mut min = [10.0f64; B as usize];
    let mut sum = [0.0f64; B as usize];
    let mut max = [0.0f64; B as usize];
    let mut cnt = [0; B as usize];
    let n = 100000000;
    for _ in 0..n {
        let x: T = random();
        let r = random::<u32>() % B;
        // eprintln!("x = {x:032b}");
        // eprintln!("r = {r:>2}");
        let k1 = find_inverse_fast(x, r, diffs);
        // eprintln!("{k1}");
        let ratio = k1 as f64 / 2.0f64.powi(r as _);
        // eprintln!("{}", ratio);
        min[r as usize] = min[r as usize].min(ratio);
        sum[r as usize] += ratio;
        cnt[r as usize] += 1;
        max[r as usize] = max[r as usize].max(ratio);
        // let k2 = find_inverse_bruteforce(x, r);
        // assert_eq!(k1, k2);
    }
    for r in 0..B as usize {
        eprintln!(
            "r = {r:>2}: {avg:>10.3} {max:>10.3}",
            r = r,
            avg = sum[r] / cnt[r] as f64,
            max = max[r],
        );
    }
}

fn test_invert_fr64() {
    let diffs = &find_diffs(C);

    for i in 0..100000000 {
        if i % 1000000 == 0 {
            eprintln!("{}", i);
        }
        let n = random::<usize>() % 10_000_000_000;
        let p = random::<usize>() % n;
        let x = Hash::new(random());
        // eprintln!("n = {n:>10}");
        // eprintln!("p = {p:>10}");
        // eprintln!("x = {:064b}", x.get());
        let k1 = invert_fr64_fast(x, n, p, diffs);
        // let k2 = invert_fr64_bruteforce(x, n, p);
        // assert_eq!(k1, k2);
    }
}

fn main() {
    test_invert_fr64();
}