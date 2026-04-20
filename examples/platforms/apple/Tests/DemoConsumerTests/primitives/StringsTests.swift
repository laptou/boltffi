import Demo
import XCTest

final class StringsTests: XCTestCase {
    func testStringFns() {
        XCTAssertEqual(echoString(v: "hello 🌍"), "hello 🌍")
        XCTAssertEqual(concatStrings(a: "foo", b: "bar"), "foobar")
        XCTAssertEqual(stringLength(v: "café"), 5)
        XCTAssertEqual(stringIsEmpty(v: ""), true)
        XCTAssertEqual(repeatString(v: "ab", count: 3), "ababab")
        XCTAssertEqual(generateString(size: 4), "xxxx")
    }
}
