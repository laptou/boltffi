package com.example.bench_compare;

import java.util.List;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.*;
import org.openjdk.jmh.infra.Blackhole;

import uniffi.bench_uniffi.BenchUniffi;
import uniffi.bench_uniffi.Location;
import uniffi.bench_uniffi.Trade;
import uniffi.bench_uniffi.Particle;
import uniffi.bench_uniffi.SensorReading;
import uniffi.bench_uniffi.UserProfile;
import uniffi.bench_uniffi.Direction;
import uniffi.bench_uniffi.Counter;
import uniffi.bench_uniffi.DataStore;
import uniffi.bench_uniffi.DataPoint;
import uniffi.bench_uniffi.Accumulator;

@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.NANOSECONDS)
@State(Scope.Thread)
public class UniffiJavaBench {
    private List<Location> locations1k;
    private List<Location> locations10k;
    private List<Trade> trades1k;
    private List<Trade> trades10k;
    private List<Particle> particles1k;
    private List<Particle> particles10k;
    private List<SensorReading> sensors1k;
    private List<SensorReading> sensors10k;
    private int[] i32Vec10k;
    private int[] i32Vec100k;
    private double[] f64Vec10k;
    private List<UserProfile> users100;
    private List<UserProfile> users1k;

    @Setup
    public void setup() {
        locations1k = BenchUniffi.generateLocations(1000);
        locations10k = BenchUniffi.generateLocations(10000);
        trades1k = BenchUniffi.generateTrades(1000);
        trades10k = BenchUniffi.generateTrades(10000);
        particles1k = BenchUniffi.generateParticles(1000);
        particles10k = BenchUniffi.generateParticles(10000);
        sensors1k = BenchUniffi.generateSensorReadings(1000);
        sensors10k = BenchUniffi.generateSensorReadings(10000);
        i32Vec10k = BenchUniffi.generateI32Vec(10000);
        i32Vec100k = BenchUniffi.generateI32Vec(100_000);
        f64Vec10k = BenchUniffi.generateF64Vec(10000);
        users100 = BenchUniffi.generateUserProfiles(100);
        users1k = BenchUniffi.generateUserProfiles(1000);
    }

    // --- Call Overhead ---

    @Benchmark
    public void uniffi_java_noop(Blackhole bh) {
        BenchUniffi.noop();
        bh.consume(0);
    }

    @Benchmark
    public void uniffi_java_echo_i32(Blackhole bh) {
        bh.consume(BenchUniffi.echoI32(42));
    }

    @Benchmark
    public void uniffi_java_add(Blackhole bh) {
        bh.consume(BenchUniffi.add(100, 200));
    }

    @Benchmark
    public void uniffi_java_inc_u64(Blackhole bh) {
        bh.consume(BenchUniffi.incU64(0L));
    }

    // --- Strings ---

    @Benchmark
    public void uniffi_java_echo_string_small(Blackhole bh) {
        bh.consume(BenchUniffi.echoString("hello"));
    }

    @Benchmark
    public void uniffi_java_echo_string_1k(Blackhole bh) {
        bh.consume(BenchUniffi.echoString("x".repeat(1000)));
    }

    // --- Struct Generation ---

    @Benchmark
    public void uniffi_java_generate_locations_1k(Blackhole bh) {
        bh.consume(BenchUniffi.generateLocations(1000));
    }

    @Benchmark
    public void uniffi_java_generate_locations_10k(Blackhole bh) {
        bh.consume(BenchUniffi.generateLocations(10000));
    }

    @Benchmark
    public void uniffi_java_generate_trades_1k(Blackhole bh) {
        bh.consume(BenchUniffi.generateTrades(1000));
    }

    @Benchmark
    public void uniffi_java_generate_trades_10k(Blackhole bh) {
        bh.consume(BenchUniffi.generateTrades(10000));
    }

    @Benchmark
    public void uniffi_java_generate_particles_1k(Blackhole bh) {
        bh.consume(BenchUniffi.generateParticles(1000));
    }

    @Benchmark
    public void uniffi_java_generate_particles_10k(Blackhole bh) {
        bh.consume(BenchUniffi.generateParticles(10000));
    }

    @Benchmark
    public void uniffi_java_generate_sensors_1k(Blackhole bh) {
        bh.consume(BenchUniffi.generateSensorReadings(1000));
    }

    @Benchmark
    public void uniffi_java_generate_sensors_10k(Blackhole bh) {
        bh.consume(BenchUniffi.generateSensorReadings(10000));
    }

    @Benchmark
    public void uniffi_java_generate_user_profiles_100(Blackhole bh) {
        bh.consume(BenchUniffi.generateUserProfiles(100));
    }

    @Benchmark
    public void uniffi_java_generate_user_profiles_1k(Blackhole bh) {
        bh.consume(BenchUniffi.generateUserProfiles(1000));
    }

    // --- Struct Consumption ---

    @Benchmark
    public void uniffi_java_sum_ratings_1k(Blackhole bh) {
        bh.consume(BenchUniffi.sumRatings(locations1k));
    }

    @Benchmark
    public void uniffi_java_sum_ratings_10k(Blackhole bh) {
        bh.consume(BenchUniffi.sumRatings(locations10k));
    }

    @Benchmark
    public void uniffi_java_sum_trade_volumes_1k(Blackhole bh) {
        bh.consume(BenchUniffi.sumTradeVolumes(trades1k));
    }

    @Benchmark
    public void uniffi_java_sum_trade_volumes_10k(Blackhole bh) {
        bh.consume(BenchUniffi.sumTradeVolumes(trades10k));
    }

    @Benchmark
    public void uniffi_java_sum_particle_masses_1k(Blackhole bh) {
        bh.consume(BenchUniffi.sumParticleMasses(particles1k));
    }

    @Benchmark
    public void uniffi_java_sum_particle_masses_10k(Blackhole bh) {
        bh.consume(BenchUniffi.sumParticleMasses(particles10k));
    }

    @Benchmark
    public void uniffi_java_avg_sensor_temp_1k(Blackhole bh) {
        bh.consume(BenchUniffi.avgSensorTemperature(sensors1k));
    }

    @Benchmark
    public void uniffi_java_avg_sensor_temp_10k(Blackhole bh) {
        bh.consume(BenchUniffi.avgSensorTemperature(sensors10k));
    }

    @Benchmark
    public void uniffi_java_process_locations_1k(Blackhole bh) {
        bh.consume(BenchUniffi.processLocations(locations1k));
    }

    @Benchmark
    public void uniffi_java_process_locations_10k(Blackhole bh) {
        bh.consume(BenchUniffi.processLocations(locations10k));
    }

    @Benchmark
    public void uniffi_java_sum_user_scores_100(Blackhole bh) {
        bh.consume(BenchUniffi.sumUserScores(users100));
    }

    @Benchmark
    public void uniffi_java_sum_user_scores_1k(Blackhole bh) {
        bh.consume(BenchUniffi.sumUserScores(users1k));
    }

    @Benchmark
    public void uniffi_java_count_active_users_100(Blackhole bh) {
        bh.consume(BenchUniffi.countActiveUsers(users100));
    }

    @Benchmark
    public void uniffi_java_count_active_users_1k(Blackhole bh) {
        bh.consume(BenchUniffi.countActiveUsers(users1k));
    }

    // --- Primitive Vectors ---

    @Benchmark
    public void uniffi_java_generate_i32_vec_10k(Blackhole bh) {
        bh.consume(BenchUniffi.generateI32Vec(10000));
    }

    @Benchmark
    public void uniffi_java_generate_i32_vec_100k(Blackhole bh) {
        bh.consume(BenchUniffi.generateI32Vec(100_000));
    }

    @Benchmark
    public void uniffi_java_generate_f64_vec_10k(Blackhole bh) {
        bh.consume(BenchUniffi.generateF64Vec(10000));
    }

    @Benchmark
    public void uniffi_java_generate_bytes_64k(Blackhole bh) {
        bh.consume(BenchUniffi.generateBytes(65536));
    }

    @Benchmark
    public void uniffi_java_sum_i32_vec_10k(Blackhole bh) {
        bh.consume(BenchUniffi.sumI32Vec(i32Vec10k));
    }

    @Benchmark
    public void uniffi_java_sum_i32_vec_100k(Blackhole bh) {
        bh.consume(BenchUniffi.sumI32Vec(i32Vec100k));
    }

    @Benchmark
    public void uniffi_java_sum_f64_vec_10k(Blackhole bh) {
        bh.consume(BenchUniffi.sumF64Vec(f64Vec10k));
    }

    // --- Classes ---

    @Benchmark
    public void uniffi_java_counter_increment_mutex(Blackhole bh) {
        try (var counter = new Counter()) {
            for (int i = 0; i < 1000; i++) {
                counter.increment();
            }
            bh.consume(counter.get());
        }
    }

    @Benchmark
    public void uniffi_java_datastore_add(Blackhole bh) {
        try (var store = new DataStore()) {
            for (int i = 0; i < 1000; i++) {
                store.add(new DataPoint((double) i, (double) i * 2.0, (long) i));
            }
            bh.consume(store.len());
        }
    }

    @Benchmark
    public void uniffi_java_accumulator_mutex(Blackhole bh) {
        try (var acc = new Accumulator()) {
            for (int i = 0; i < 1000; i++) {
                acc.add((long) i);
            }
            bh.consume(acc.get());
            acc.reset();
        }
    }

    // --- Enums ---

    @Benchmark
    public void uniffi_java_simple_enum(Blackhole bh) {
        bh.consume(BenchUniffi.oppositeDirection(Direction.NORTH));
        bh.consume(BenchUniffi.directionToDegrees(Direction.EAST));
    }

    // --- Optional ---

    @Benchmark
    public void uniffi_java_find_even(Blackhole bh) {
        for (int i = 0; i < 100; i++) {
            bh.consume(BenchUniffi.findEven(i));
        }
    }
}
