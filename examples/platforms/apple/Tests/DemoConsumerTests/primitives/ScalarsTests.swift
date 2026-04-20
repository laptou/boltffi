import Demo
import XCTest

final class ScalarsTests: XCTestCase {
    func testScalarFns() {
        XCTAssertEqual(echoBool(v: true), true)
        XCTAssertEqual(negateBool(v: false), true)
        XCTAssertEqual(echoI8(v: -7), -7)
        XCTAssertEqual(echoU8(v: 255), 255)
        XCTAssertEqual(echoI16(v: -1234), -1234)
        XCTAssertEqual(echoU16(v: 55_000), 55_000)
        XCTAssertEqual(echoI32(v: -42), -42)
        XCTAssertEqual(addI32(a: 10, b: 20), 30)
        XCTAssertEqual(echoU32(v: 4_000_000_000), 4_000_000_000)
        XCTAssertEqual(echoI64(v: -9_999_999_999), -9_999_999_999)
        XCTAssertEqual(echoU64(v: 9_999_999_999), 9_999_999_999)
        XCTAssertEqual(echoF32(v: 3.5), 3.5, accuracy: 1e-6)
        XCTAssertEqual(addF32(a: 1.5, b: 2.5), 4.0, accuracy: 1e-6)
        XCTAssertEqual(echoF64(v: 3.14159265359), 3.14159265359, accuracy: 1e-9)
        XCTAssertEqual(addF64(a: 1.5, b: 2.5), 4.0, accuracy: 1e-9)
        XCTAssertEqual(echoUsize(v: 123), 123)
        XCTAssertEqual(echoIsize(v: -123), -123)
        noop()
        XCTAssertEqual(Demo.add(a: 10, b: 20), 30)
        XCTAssertEqual(multiply(a: 1.5, b: 2.0), 3.0, accuracy: 1e-9)
    }
}
