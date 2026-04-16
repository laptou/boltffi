package com.example.bench_compare

import com.example.bench_boltffi.*
import java.util.concurrent.TimeUnit
import org.openjdk.jmh.annotations.Benchmark
import org.openjdk.jmh.annotations.BenchmarkMode
import org.openjdk.jmh.annotations.Mode
import org.openjdk.jmh.annotations.OutputTimeUnit
import org.openjdk.jmh.annotations.Scope
import org.openjdk.jmh.annotations.Setup
import org.openjdk.jmh.annotations.State
import org.openjdk.jmh.infra.Blackhole
import uniffi.bench_uniffi.Direction as UniffiDirection

@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.NANOSECONDS)
@State(Scope.Thread)
open class BoltFFIVsUniffiBench {
    private lateinit var boltffiLocations1k: List<Location>
    private lateinit var boltffiLocations10k: List<Location>
    private lateinit var boltffiTrades1k: List<Trade>
    private lateinit var boltffiTrades10k: List<Trade>
    private lateinit var boltffiParticles1k: List<Particle>
    private lateinit var boltffiParticles10k: List<Particle>
    private lateinit var boltffiSensors1k: List<SensorReading>
    private lateinit var boltffiSensors10k: List<SensorReading>
    private lateinit var boltffiI32Vec10k: IntArray
    private lateinit var boltffiI32Vec100k: IntArray
    private lateinit var boltffiF64Vec10k: DoubleArray
    private lateinit var boltffiUsers100: List<UserProfile>
    private lateinit var boltffiUsers1k: List<UserProfile>

    private lateinit var uniffiLocations1k: List<uniffi.bench_uniffi.Location>
    private lateinit var uniffiLocations10k: List<uniffi.bench_uniffi.Location>
    private lateinit var uniffiTrades1k: List<uniffi.bench_uniffi.Trade>
    private lateinit var uniffiTrades10k: List<uniffi.bench_uniffi.Trade>
    private lateinit var uniffiParticles1k: List<uniffi.bench_uniffi.Particle>
    private lateinit var uniffiParticles10k: List<uniffi.bench_uniffi.Particle>
    private lateinit var uniffiSensors1k: List<uniffi.bench_uniffi.SensorReading>
    private lateinit var uniffiSensors10k: List<uniffi.bench_uniffi.SensorReading>
    private lateinit var uniffiI32Vec10k: List<Int>
    private lateinit var uniffiI32Vec100k: List<Int>
    private lateinit var uniffiF64Vec10k: List<Double>
    private lateinit var uniffiUsers100: List<uniffi.bench_uniffi.UserProfile>
    private lateinit var uniffiUsers1k: List<uniffi.bench_uniffi.UserProfile>

    @Setup
    open fun setup() {
        boltffiLocations1k = generateLocations(1000)
        boltffiLocations10k = generateLocations(10000)
        boltffiTrades1k = generateTrades(1000)
        boltffiTrades10k = generateTrades(10000)
        boltffiParticles1k = generateParticles(1000)
        boltffiParticles10k = generateParticles(10000)
        boltffiSensors1k = generateSensorReadings(1000)
        boltffiSensors10k = generateSensorReadings(10000)
        boltffiI32Vec10k = generateI32Vec(10000)
        boltffiI32Vec100k = generateI32Vec(100_000)
        boltffiF64Vec10k = generateF64Vec(10000)
        boltffiUsers100 = generateUserProfiles(100)
        boltffiUsers1k = generateUserProfiles(1000)

        uniffiLocations1k = uniffi.bench_uniffi.generateLocations(1000)
        uniffiLocations10k = uniffi.bench_uniffi.generateLocations(10000)
        uniffiTrades1k = uniffi.bench_uniffi.generateTrades(1000)
        uniffiTrades10k = uniffi.bench_uniffi.generateTrades(10000)
        uniffiParticles1k = uniffi.bench_uniffi.generateParticles(1000)
        uniffiParticles10k = uniffi.bench_uniffi.generateParticles(10000)
        uniffiSensors1k = uniffi.bench_uniffi.generateSensorReadings(1000)
        uniffiSensors10k = uniffi.bench_uniffi.generateSensorReadings(10000)
        uniffiI32Vec10k = uniffi.bench_uniffi.generateI32Vec(10000)
        uniffiI32Vec100k = uniffi.bench_uniffi.generateI32Vec(100_000)
        uniffiF64Vec10k = uniffi.bench_uniffi.generateF64Vec(10000)
        uniffiUsers100 = uniffi.bench_uniffi.generateUserProfiles(100)
        uniffiUsers1k = uniffi.bench_uniffi.generateUserProfiles(1000)
    }

    @Benchmark
    open fun boltffi_noop(blackhole: Blackhole) {
        noop()
        blackhole.consume(0)
    }

    @Benchmark
    open fun uniffi_noop(blackhole: Blackhole) {
        uniffi.bench_uniffi.noop()
        blackhole.consume(0)
    }

    @Benchmark
    open fun boltffi_echo_i32(blackhole: Blackhole) {
        blackhole.consume(echoI32(42))
    }

    @Benchmark
    open fun uniffi_echo_i32(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.echoI32(42))
    }

    @Benchmark
    open fun boltffi_add(blackhole: Blackhole) {
        blackhole.consume(add(100, 200))
    }

    @Benchmark
    open fun uniffi_add(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.add(100, 200))
    }

    @Benchmark
    open fun boltffi_inc_u64(blackhole: Blackhole) {
        val arr = longArrayOf(0L)
        incU64(arr)
        blackhole.consume(arr[0])
    }

    @Benchmark
    open fun uniffi_inc_u64(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.incU64(0uL))
    }

    @Benchmark
    open fun boltffi_inc_u64_value(blackhole: Blackhole) {
        blackhole.consume(incU64Value(0uL))
    }

    @Benchmark
    open fun uniffi_inc_u64_value(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.incU64(0uL))
    }

    @Benchmark
    open fun boltffi_echo_string_small(blackhole: Blackhole) {
        blackhole.consume(echoString("hello"))
    }

    @Benchmark
    open fun uniffi_echo_string_small(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.echoString("hello"))
    }

    @Benchmark
    open fun boltffi_echo_string_1k(blackhole: Blackhole) {
        blackhole.consume(echoString("x".repeat(1000)))
    }

    @Benchmark
    open fun uniffi_echo_string_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.echoString("x".repeat(1000)))
    }

    @Benchmark
    open fun boltffi_generate_locations_1k(blackhole: Blackhole) {
        blackhole.consume(generateLocations(1000))
    }

    @Benchmark
    open fun uniffi_generate_locations_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateLocations(1000))
    }

    @Benchmark
    open fun boltffi_generate_locations_10k(blackhole: Blackhole) {
        blackhole.consume(generateLocations(10000))
    }

    @Benchmark
    open fun uniffi_generate_locations_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateLocations(10000))
    }

    @Benchmark
    open fun boltffi_generate_trades_1k(blackhole: Blackhole) {
        blackhole.consume(generateTrades(1000))
    }

    @Benchmark
    open fun uniffi_generate_trades_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateTrades(1000))
    }

    @Benchmark
    open fun boltffi_generate_trades_10k(blackhole: Blackhole) {
        blackhole.consume(generateTrades(10000))
    }

    @Benchmark
    open fun uniffi_generate_trades_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateTrades(10000))
    }

    @Benchmark
    open fun boltffi_generate_particles_1k(blackhole: Blackhole) {
        blackhole.consume(generateParticles(1000))
    }

    @Benchmark
    open fun uniffi_generate_particles_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateParticles(1000))
    }

    @Benchmark
    open fun boltffi_generate_particles_10k(blackhole: Blackhole) {
        blackhole.consume(generateParticles(10000))
    }

    @Benchmark
    open fun uniffi_generate_particles_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateParticles(10000))
    }

    @Benchmark
    open fun boltffi_generate_sensors_1k(blackhole: Blackhole) {
        blackhole.consume(generateSensorReadings(1000))
    }

    @Benchmark
    open fun uniffi_generate_sensors_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateSensorReadings(1000))
    }

    @Benchmark
    open fun boltffi_generate_sensors_10k(blackhole: Blackhole) {
        blackhole.consume(generateSensorReadings(10000))
    }

    @Benchmark
    open fun uniffi_generate_sensors_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateSensorReadings(10000))
    }

    @Benchmark
    open fun boltffi_generate_i32_vec_10k(blackhole: Blackhole) {
        blackhole.consume(generateI32Vec(10000))
    }

    @Benchmark
    open fun uniffi_generate_i32_vec_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateI32Vec(10000))
    }

    @Benchmark
    open fun boltffi_generate_i32_vec_100k(blackhole: Blackhole) {
        blackhole.consume(generateI32Vec(100_000))
    }

    @Benchmark
    open fun uniffi_generate_i32_vec_100k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateI32Vec(100_000))
    }

    @Benchmark
    open fun boltffi_generate_f64_vec_10k(blackhole: Blackhole) {
        blackhole.consume(generateF64Vec(10000))
    }

    @Benchmark
    open fun uniffi_generate_f64_vec_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateF64Vec(10000))
    }

    @Benchmark
    open fun boltffi_generate_bytes_64k(blackhole: Blackhole) {
        blackhole.consume(generateBytes(65536))
    }

    @Benchmark
    open fun uniffi_generate_bytes_64k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateBytes(65536))
    }

    @Benchmark
    open fun boltffi_sum_ratings_1k(blackhole: Blackhole) {
        blackhole.consume(sumRatings(boltffiLocations1k))
    }

    @Benchmark
    open fun uniffi_sum_ratings_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.sumRatings(uniffiLocations1k))
    }

    @Benchmark
    open fun boltffi_sum_ratings_10k(blackhole: Blackhole) {
        blackhole.consume(sumRatings(boltffiLocations10k))
    }

    @Benchmark
    open fun uniffi_sum_ratings_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.sumRatings(uniffiLocations10k))
    }

    @Benchmark
    open fun boltffi_sum_trade_volumes_1k(blackhole: Blackhole) {
        blackhole.consume(sumTradeVolumes(boltffiTrades1k))
    }

    @Benchmark
    open fun uniffi_sum_trade_volumes_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.sumTradeVolumes(uniffiTrades1k))
    }

    @Benchmark
    open fun boltffi_sum_trade_volumes_10k(blackhole: Blackhole) {
        blackhole.consume(sumTradeVolumes(boltffiTrades10k))
    }

    @Benchmark
    open fun uniffi_sum_trade_volumes_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.sumTradeVolumes(uniffiTrades10k))
    }

    @Benchmark
    open fun boltffi_sum_particle_masses_1k(blackhole: Blackhole) {
        blackhole.consume(sumParticleMasses(boltffiParticles1k))
    }

    @Benchmark
    open fun uniffi_sum_particle_masses_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.sumParticleMasses(uniffiParticles1k))
    }

    @Benchmark
    open fun boltffi_sum_particle_masses_10k(blackhole: Blackhole) {
        blackhole.consume(sumParticleMasses(boltffiParticles10k))
    }

    @Benchmark
    open fun uniffi_sum_particle_masses_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.sumParticleMasses(uniffiParticles10k))
    }

    @Benchmark
    open fun boltffi_avg_sensor_temp_1k(blackhole: Blackhole) {
        blackhole.consume(avgSensorTemperature(boltffiSensors1k))
    }

    @Benchmark
    open fun uniffi_avg_sensor_temp_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.avgSensorTemperature(uniffiSensors1k))
    }

    @Benchmark
    open fun boltffi_avg_sensor_temp_10k(blackhole: Blackhole) {
        blackhole.consume(avgSensorTemperature(boltffiSensors10k))
    }

    @Benchmark
    open fun uniffi_avg_sensor_temp_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.avgSensorTemperature(uniffiSensors10k))
    }

    @Benchmark
    open fun boltffi_sum_i32_vec_10k(blackhole: Blackhole) {
        blackhole.consume(sumI32Vec(boltffiI32Vec10k))
    }

    @Benchmark
    open fun uniffi_sum_i32_vec_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.sumI32Vec(uniffiI32Vec10k))
    }

    @Benchmark
    open fun boltffi_sum_i32_vec_100k(blackhole: Blackhole) {
        blackhole.consume(sumI32Vec(boltffiI32Vec100k))
    }

    @Benchmark
    open fun uniffi_sum_i32_vec_100k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.sumI32Vec(uniffiI32Vec100k))
    }

    @Benchmark
    open fun boltffi_sum_f64_vec_10k(blackhole: Blackhole) {
        blackhole.consume(sumF64Vec(boltffiF64Vec10k))
    }

    @Benchmark
    open fun uniffi_sum_f64_vec_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.sumF64Vec(uniffiF64Vec10k))
    }

    @Benchmark
    open fun boltffi_process_locations_1k(blackhole: Blackhole) {
        blackhole.consume(processLocations(boltffiLocations1k))
    }

    @Benchmark
    open fun uniffi_process_locations_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.processLocations(uniffiLocations1k))
    }

    @Benchmark
    open fun boltffi_process_locations_10k(blackhole: Blackhole) {
        blackhole.consume(processLocations(boltffiLocations10k))
    }

    @Benchmark
    open fun uniffi_process_locations_10k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.processLocations(uniffiLocations10k))
    }

    @Benchmark
    open fun boltffi_counter_increment_mutex(blackhole: Blackhole) {
        Counter().use { counter ->
            repeat(1000) { counter.increment() }
            blackhole.consume(counter.get())
        }
    }

    @Benchmark
    open fun uniffi_counter_increment_mutex(blackhole: Blackhole) {
        uniffi.bench_uniffi.Counter().use { counter ->
            repeat(1000) { counter.increment() }
            blackhole.consume(counter.get())
        }
    }

    @Benchmark
    open fun boltffi_counter_increment_single_threaded(blackhole: Blackhole) {
        CounterSingleThreaded().use { counter ->
            repeat(1000) { counter.increment() }
            blackhole.consume(counter.get())
        }
    }

    @Benchmark
    open fun boltffi_datastore_add(blackhole: Blackhole) {
        DataStore().use { store ->
            repeat(1000) { index ->
                store.add(DataPoint(index.toDouble(), index.toDouble() * 2.0, index.toLong()))
            }
            blackhole.consume(store.len())
        }
    }

    @Benchmark
    open fun uniffi_datastore_add(blackhole: Blackhole) {
        uniffi.bench_uniffi.DataStore().use { store ->
            repeat(1000) { index ->
                store.add(uniffi.bench_uniffi.DataPoint(index.toDouble(), index.toDouble() * 2.0, index.toLong()))
            }
            blackhole.consume(store.len())
        }
    }

    @Benchmark
    open fun boltffi_accumulator_mutex(blackhole: Blackhole) {
        Accumulator().use { accumulator ->
            repeat(1000) { index -> accumulator.add(index.toLong()) }
            blackhole.consume(accumulator.get())
            accumulator.reset()
        }
    }

    @Benchmark
    open fun uniffi_accumulator_mutex(blackhole: Blackhole) {
        uniffi.bench_uniffi.Accumulator().use { accumulator ->
            repeat(1000) { index -> accumulator.add(index.toLong()) }
            blackhole.consume(accumulator.get())
            accumulator.reset()
        }
    }

    @Benchmark
    open fun boltffi_accumulator_single_threaded(blackhole: Blackhole) {
        AccumulatorSingleThreaded().use { accumulator ->
            repeat(1000) { index -> accumulator.add(index.toLong()) }
            blackhole.consume(accumulator.get())
            accumulator.reset()
        }
    }

    @Benchmark
    open fun boltffi_simple_enum(blackhole: Blackhole) {
        blackhole.consume(oppositeDirection(Direction.NORTH))
        blackhole.consume(directionToDegrees(Direction.EAST))
    }

    @Benchmark
    open fun uniffi_simple_enum(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.oppositeDirection(UniffiDirection.NORTH))
        blackhole.consume(uniffi.bench_uniffi.directionToDegrees(UniffiDirection.EAST))
    }

    @Benchmark
    open fun boltffi_find_even(blackhole: Blackhole) {
        repeat(100) { index -> blackhole.consume(findEven(index)) }
    }

    @Benchmark
    open fun uniffi_find_even(blackhole: Blackhole) {
        repeat(100) { index -> blackhole.consume(uniffi.bench_uniffi.findEven(index)) }
    }

    @Benchmark
    open fun boltffi_generate_user_profiles_100(blackhole: Blackhole) {
        blackhole.consume(generateUserProfiles(100))
    }

    @Benchmark
    open fun uniffi_generate_user_profiles_100(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateUserProfiles(100))
    }

    @Benchmark
    open fun boltffi_generate_user_profiles_1k(blackhole: Blackhole) {
        blackhole.consume(generateUserProfiles(1000))
    }

    @Benchmark
    open fun uniffi_generate_user_profiles_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.generateUserProfiles(1000))
    }

    @Benchmark
    open fun boltffi_sum_user_scores_100(blackhole: Blackhole) {
        blackhole.consume(sumUserScores(boltffiUsers100))
    }

    @Benchmark
    open fun uniffi_sum_user_scores_100(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.sumUserScores(uniffiUsers100))
    }

    @Benchmark
    open fun boltffi_sum_user_scores_1k(blackhole: Blackhole) {
        blackhole.consume(sumUserScores(boltffiUsers1k))
    }

    @Benchmark
    open fun uniffi_sum_user_scores_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.sumUserScores(uniffiUsers1k))
    }

    @Benchmark
    open fun boltffi_count_active_users_100(blackhole: Blackhole) {
        blackhole.consume(countActiveUsers(boltffiUsers100))
    }

    @Benchmark
    open fun uniffi_count_active_users_100(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.countActiveUsers(uniffiUsers100))
    }

    @Benchmark
    open fun boltffi_count_active_users_1k(blackhole: Blackhole) {
        blackhole.consume(countActiveUsers(boltffiUsers1k))
    }

    @Benchmark
    open fun uniffi_count_active_users_1k(blackhole: Blackhole) {
        blackhole.consume(uniffi.bench_uniffi.countActiveUsers(uniffiUsers1k))
    }

    @Benchmark
    open fun boltffi_data_enum_input(blackhole: Blackhole) {
        blackhole.consume(getStatusProgress(TaskStatus.InProgress(50)))
        blackhole.consume(isStatusComplete(TaskStatus.Completed(100)))
    }
}
