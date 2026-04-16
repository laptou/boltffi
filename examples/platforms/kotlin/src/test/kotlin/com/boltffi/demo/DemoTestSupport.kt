package com.boltffi.demo

import java.io.File
import java.util.concurrent.TimeUnit
import kotlin.math.abs
import kotlin.test.assertEquals
import kotlin.test.fail

internal fun assertDoubleEquals(expected: Double, actual: Double, epsilon: Double = 1e-9) {
    assert(abs(expected - actual) <= epsilon) {
        "expected <$expected> but was <$actual> with epsilon <$epsilon>"
    }
}

internal fun assertFloatEquals(expected: Float, actual: Float, epsilon: Float = 1e-6f) {
    assert(abs(expected - actual) <= epsilon) {
        "expected <$expected> but was <$actual> with epsilon <$epsilon>"
    }
}

internal fun assertPointEquals(expectedX: Double, expectedY: Double, point: Point, epsilon: Double = 1e-9) {
    assertDoubleEquals(expectedX, point.x, epsilon)
    assertDoubleEquals(expectedY, point.y, epsilon)
}

internal fun assertMessageContains(throwable: Throwable, expectedFragment: String) {
    val message = throwable.message ?: ""
    assert(message.contains(expectedFragment)) {
        "expected message to contain <$expectedFragment> but was <$message>"
    }
}

internal fun assertIsolatedCaseSucceeds(caseName: String) {
    assertJvmMainSucceeds("com.boltffi.demo.DemoIsolatedCasesKt", caseName)
}

internal fun assertJvmMainSucceeds(mainClass: String, vararg args: String) {
    val javaExecutable = File(System.getProperty("java.home"), "bin/java").absolutePath
    val classPath = System.getProperty("java.class.path")
    val libraryPath = System.getProperty("java.library.path")
    val command = mutableListOf(
        javaExecutable,
        "-Djava.library.path=$libraryPath",
        "-cp",
        classPath,
        mainClass,
    )
    command.addAll(args)
    val process = ProcessBuilder(command).redirectErrorStream(true).start()
    if (!process.waitFor(15, TimeUnit.SECONDS)) {
        process.destroyForcibly()
        fail("main class <$mainClass> timed out")
    }
    val output = process.inputStream.bufferedReader().use { it.readText() }
    val exitCode = process.exitValue()
    val description = if (args.isEmpty()) mainClass else "$mainClass ${args.joinToString(" ")}"
    assertEquals(0, exitCode, "main class <$description> failed with exit code <$exitCode>\n$output")
}
