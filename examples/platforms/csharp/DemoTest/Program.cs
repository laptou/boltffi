using System;
using Demo;
using static Demo.Demo;

namespace BoltFFI.Demo.Tests;

public static class DemoTest
{
    public static int Main()
    {
        Console.WriteLine("Testing C# bindings...\n");
        TestBool();
        TestI8();
        TestU8();
        TestI16();
        TestU16();
        TestI32();
        TestU32();
        TestI64();
        TestU64();
        TestF32();
        TestF64();
        TestUsize();
        TestIsize();
        TestStrings();
        TestBlittableRecords();
        TestRecordsWithStrings();
        TestNestedRecords();
        Console.WriteLine("All tests passed!");
        return 0;
    }

    private static void TestBool()
    {
        Console.WriteLine("Testing bool...");
        Require(EchoBool(true), "echoBool(true)");
        Require(!EchoBool(false), "echoBool(false)");
        Require(!NegateBool(true), "negateBool(true)");
        Require(NegateBool(false), "negateBool(false)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestI8()
    {
        Console.WriteLine("Testing i8...");
        Require(EchoI8(42) == 42, "echoI8(42)");
        Require(EchoI8(-128) == -128, "echoI8(min)");
        Require(EchoI8(127) == 127, "echoI8(max)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestU8()
    {
        Console.WriteLine("Testing u8...");
        Require(EchoU8(0) == 0, "echoU8(0)");
        Require(EchoU8(255) == 255, "echoU8(max)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestI16()
    {
        Console.WriteLine("Testing i16...");
        Require(EchoI16(-32768) == -32768, "echoI16(min)");
        Require(EchoI16(32767) == 32767, "echoI16(max)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestU16()
    {
        Console.WriteLine("Testing u16...");
        Require(EchoU16(0) == 0, "echoU16(0)");
        Require(EchoU16(65535) == 65535, "echoU16(max)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestI32()
    {
        Console.WriteLine("Testing i32...");
        Require(EchoI32(42) == 42, "echoI32(42)");
        Require(EchoI32(-100) == -100, "echoI32(-100)");
        Require(AddI32(10, 20) == 30, "addI32(10, 20)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestU32()
    {
        Console.WriteLine("Testing u32...");
        Require(EchoU32(0u) == 0u, "echoU32(0)");
        Require(EchoU32(uint.MaxValue) == uint.MaxValue, "echoU32(max)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestI64()
    {
        Console.WriteLine("Testing i64...");
        Require(EchoI64(9999999999L) == 9999999999L, "echoI64(large)");
        Require(EchoI64(-9999999999L) == -9999999999L, "echoI64(negative large)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestU64()
    {
        Console.WriteLine("Testing u64...");
        Require(EchoU64(0UL) == 0UL, "echoU64(0)");
        Require(EchoU64(ulong.MaxValue) == ulong.MaxValue, "echoU64(max)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestF32()
    {
        Console.WriteLine("Testing f32...");
        Require(Math.Abs(EchoF32(3.14f) - 3.14f) < 0.001f, "echoF32(3.14)");
        Require(Math.Abs(AddF32(1.5f, 2.5f) - 4.0f) < 0.001f, "addF32(1.5, 2.5)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestF64()
    {
        Console.WriteLine("Testing f64...");
        Require(Math.Abs(EchoF64(3.14159265359) - 3.14159265359) < 0.0000001, "echoF64(pi)");
        Require(Math.Abs(AddF64(1.5, 2.5) - 4.0) < 0.0000001, "addF64(1.5, 2.5)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestUsize()
    {
        Console.WriteLine("Testing usize...");
        Require(EchoUsize((nuint)42) == (nuint)42, "echoUsize(42)");
        Require(EchoUsize((nuint)0) == (nuint)0, "echoUsize(0)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestIsize()
    {
        Console.WriteLine("Testing isize...");
        Require(EchoIsize((nint)42) == (nint)42, "echoIsize(42)");
        Require(EchoIsize((nint)(-100)) == (nint)(-100), "echoIsize(-100)");
        Console.WriteLine("  PASS\n");
    }

    private static void TestStrings()
    {
        Console.WriteLine("Testing strings...");
        Require(EchoString("hello") == "hello", "echoString(hello)");
        Require(EchoString("") == "", "echoString(empty)");
        Require(EchoString("café") == "café", "echoString(unicode)");
        Require(EchoString("日本語") == "日本語", "echoString(cjk)");
        Require(EchoString("hello 🌍 world") == "hello 🌍 world", "echoString(emoji)");

        Require(ConcatStrings("foo", "bar") == "foobar", "concatStrings(foo, bar)");
        Require(ConcatStrings("", "bar") == "bar", "concatStrings(empty, bar)");
        Require(ConcatStrings("foo", "") == "foo", "concatStrings(foo, empty)");
        Require(ConcatStrings("🎉", "🎊") == "🎉🎊", "concatStrings(emoji)");

        Require(StringLength("hello") == 5u, "stringLength(hello)");
        Require(StringLength("") == 0u, "stringLength(empty)");
        Require(StringLength("café") == 5u, "stringLength(utf8 bytes)");
        Require(StringLength("🌍") == 4u, "stringLength(emoji 4 bytes)");

        Require(StringIsEmpty(""), "stringIsEmpty(empty)");
        Require(!StringIsEmpty("x"), "stringIsEmpty(nonempty)");

        Require(RepeatString("ab", 3u) == "ababab", "repeatString(ab, 3)");
        Require(RepeatString("x", 0u) == "", "repeatString(x, 0)");
        Require(RepeatString("🌟", 2u) == "🌟🌟", "repeatString(emoji, 2)");
        Console.WriteLine("  PASS\n");
    }

    /// <summary>
    /// Blittable records (Point, Color) cross the ABI as direct struct
    /// values via [StructLayout(Sequential)] — no WireWriter / WireReader
    /// involvement. These tests exercise the zero-copy fast path.
    /// </summary>
    private static void TestBlittableRecords()
    {
        Console.WriteLine("Testing blittable records (Point, Color)...");

        Point p = MakePoint(1.5, 2.5);
        Require(p.X == 1.5, "MakePoint.X");
        Require(p.Y == 2.5, "MakePoint.Y");

        Point echoed = EchoPoint(new Point(3.0, 4.0));
        Require(echoed == new Point(3.0, 4.0), "EchoPoint value equality");

        Point sum = AddPoints(new Point(1.0, 2.0), new Point(3.0, 4.0));
        Require(sum == new Point(4.0, 6.0), "AddPoints");

        Color c = MakeColor(10, 20, 30, 255);
        Require(c.R == 10 && c.G == 20 && c.B == 30 && c.A == 255, "MakeColor fields");

        Color echoedColor = EchoColor(new Color(255, 0, 0, 128));
        Require(echoedColor == new Color(255, 0, 0, 128), "EchoColor value equality");

        Console.WriteLine("  PASS\n");
    }

    /// <summary>
    /// Non-blittable records travel through the wire path: WireWriter on
    /// the way in, FfiBuf + WireReader + FreeBuf on the way out. Strings
    /// inside records exercise the per-field UTF-8 length prefix.
    /// </summary>
    private static void TestRecordsWithStrings()
    {
        Console.WriteLine("Testing records with strings (Person, Address)...");

        Person alice = MakePerson("Alice", 30);
        Require(alice.Name == "Alice", "MakePerson.Name");
        Require(alice.Age == 30u, "MakePerson.Age");

        Person echoed = EchoPerson(new Person("Bob", 42));
        Require(echoed == new Person("Bob", 42), "EchoPerson value equality");

        // Empty string boundary — the wire length prefix is 0.
        Person empty = EchoPerson(new Person("", 0));
        Require(empty.Name == "", "EchoPerson empty name");

        // Multi-byte UTF-8 boundary — one code point that encodes as 4 bytes.
        Person emoji = EchoPerson(new Person("\ud83c\udf89 Party", 25));
        Require(emoji.Name == "\ud83c\udf89 Party", "EchoPerson emoji round-trip");

        Require(
            GreetPerson(new Person("Alice", 30)) == "Hello, Alice! You are 30 years old.",
            "GreetPerson format"
        );

        // Address has three string fields back-to-back — exercises multiple
        // length-prefixed slices in one wire buffer.
        Address home = new Address("221B Baker Street", "London", "NW1 6XE");
        Address echoedAddress = EchoAddress(home);
        Require(echoedAddress == home, "EchoAddress round-trip");

        Require(
            FormatAddress(home) == "221B Baker Street, London, NW1 6XE",
            "FormatAddress concatenation"
        );

        Console.WriteLine("  PASS\n");
    }

    /// <summary>
    /// Nested records: Line holds two Points, Rect holds Point + Dimensions.
    /// Exercises the record-inside-record wire encode/decode path.
    /// </summary>
    private static void TestNestedRecords()
    {
        Console.WriteLine("Testing nested records (Line, Rect)...");

        Line line = MakeLine(0.0, 0.0, 3.0, 4.0);
        Require(line.Start == new Point(0.0, 0.0), "MakeLine.Start");
        Require(line.End == new Point(3.0, 4.0), "MakeLine.End");

        Line echoed = EchoLine(line);
        Require(echoed == line, "EchoLine round-trip");

        Require(Math.Abs(LineLength(line) - 5.0) < 1e-9, "LineLength 3-4-5");

        Rect rect = new Rect(
            new Point(1.0, 2.0),
            new Dimensions(10.0, 20.0)
        );
        Rect echoedRect = EchoRect(rect);
        Require(echoedRect == rect, "EchoRect round-trip");

        Require(Math.Abs(RectArea(rect) - 200.0) < 1e-9, "RectArea 10*20");

        Console.WriteLine("  PASS\n");
    }

    private static void Require(bool condition, string label)
    {
        if (!condition) throw new InvalidOperationException($"FAIL: {label}");
    }
}
