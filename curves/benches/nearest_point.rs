/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use criterion::{criterion_group, criterion_main, Criterion};

use flo_curves::bezier::*;
use flo_curves::*;

fn criterion_benchmark(c: &mut Criterion) {
    let curve = bezier::Curve::from_points(
        Coord2(10.0, 100.0),
        (Coord2(90.0, 30.0), Coord2(40.0, 140.0)),
        Coord2(220.0, 220.0),
    );
    let point = Coord2(100.0, 130.0);

    c.bench_function("default_algorithm", |b| {
        b.iter(|| nearest_point_on_curve(&curve, &point))
    });
    c.bench_function("newton_raphson", |b| {
        b.iter(|| nearest_point_on_curve_newton_raphson(&curve, &point))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
