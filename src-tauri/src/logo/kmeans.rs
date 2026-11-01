//! K-means с фиксированным сидом: результат повторяем от запуска к запуску.

use crate::preprocess::lab::distance_sq;
use rayon::prelude::*;

const SEED: u64 = 0x9E37_79B9_7F4A_7C15;
const ATTEMPTS: usize = 6;
const MAX_ITERATIONS: usize = 60;
const EPSILON: f32 = 0.2;
/// Больше выборки качество не растёт, а время растёт линейно.
const MAX_SAMPLES: usize = 60_000;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

pub fn nearest(point: [f32; 3], centers: &[[f32; 3]]) -> (usize, f32) {
    let mut best = (0, f32::MAX);
    for (index, center) in centers.iter().enumerate() {
        let d = distance_sq(point, *center);
        if d < best.1 {
            best = (index, d);
        }
    }
    best
}

fn seed_centers(samples: &[[f32; 3]], k: usize, rng: &mut Rng) -> Vec<[f32; 3]> {
    let mut centers = vec![samples[rng.next() as usize % samples.len()]];
    let mut distances: Vec<f32> = samples.iter().map(|s| distance_sq(*s, centers[0])).collect();
    while centers.len() < k {
        let total: f32 = distances.iter().sum();
        if total <= 0.0 {
            // Различных цветов меньше, чем просили.
            centers.push(centers[0]);
            continue;
        }
        let mut target = rng.unit() * total;
        let mut chosen = samples.len() - 1;
        for (index, distance) in distances.iter().enumerate() {
            target -= distance;
            if target <= 0.0 {
                chosen = index;
                break;
            }
        }
        let center = samples[chosen];
        centers.push(center);
        for (slot, sample) in distances.iter_mut().zip(samples) {
            *slot = slot.min(distance_sq(*sample, center));
        }
    }
    centers
}

type Sums = (Vec<[f64; 3]>, Vec<u64>, f64);

fn refine(samples: &[[f32; 3]], mut centers: Vec<[f32; 3]>) -> (Vec<[f32; 3]>, f32) {
    let k = centers.len();
    let empty = || -> Sums { (vec![[0.0; 3]; k], vec![0; k], 0.0) };
    let mut compactness = f32::MAX;
    for _ in 0..MAX_ITERATIONS {
        let (sums, counts, total) = samples
            .par_iter()
            .fold(empty, |(mut sums, mut counts, total), sample| {
                let (index, distance) = nearest(*sample, &centers);
                (0..3).for_each(|i| sums[index][i] += sample[i] as f64);
                counts[index] += 1;
                (sums, counts, total + distance as f64)
            })
            .reduce(empty, |(mut sums, mut counts, total), (more_sums, more_counts, more)| {
                for index in 0..k {
                    (0..3).for_each(|i| sums[index][i] += more_sums[index][i]);
                    counts[index] += more_counts[index];
                }
                (sums, counts, total + more)
            });
        compactness = total as f32;

        let mut shift = 0f32;
        for index in 0..k {
            if counts[index] == 0 {
                continue;
            }
            let next = sums[index].map(|sum| (sum / counts[index] as f64) as f32);
            shift = shift.max(distance_sq(next, centers[index]));
            centers[index] = next;
        }
        if shift <= EPSILON * EPSILON {
            break;
        }
    }
    (centers, compactness)
}

pub fn kmeans(samples: &[[f32; 3]], k: usize) -> Vec<[f32; 3]> {
    let step = (samples.len() / MAX_SAMPLES).max(1);
    let thinned: Vec<[f32; 3]> = samples.iter().step_by(step).copied().collect();
    let mut rng = Rng(SEED);
    let mut best: Option<(Vec<[f32; 3]>, f32)> = None;
    for _ in 0..ATTEMPTS {
        let candidate = refine(&thinned, seed_centers(&thinned, k, &mut rng));
        if best.as_ref().is_none_or(|b| candidate.1 < b.1) {
            best = Some(candidate);
        }
    }
    best.expect("хотя бы одна попытка").0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples() -> Vec<[f32; 3]> {
        let mut out = Vec::new();
        for i in 0..300 {
            let jitter = (i % 5) as f32;
            out.push([40.0 + jitter, 10.0, 10.0]);
            out.push([200.0 + jitter, -30.0, 50.0]);
            out.push([120.0, 60.0 + jitter, -40.0]);
        }
        out
    }

    #[test]
    fn repeatable_and_finds_clusters() {
        let first = kmeans(&samples(), 3);
        assert_eq!(first, kmeans(&samples(), 3));
        for expected in [[42.0, 10.0, 10.0], [202.0, -30.0, 50.0], [120.0, 62.0, -40.0]] {
            let (_, distance) = nearest(expected, &first);
            assert!(distance < 1.0, "{expected:?} → {first:?}");
        }
    }

    #[test]
    fn survives_fewer_colours_than_requested() {
        assert_eq!(kmeans(&[[1.0, 2.0, 3.0]; 20], 4).len(), 4);
    }
}
