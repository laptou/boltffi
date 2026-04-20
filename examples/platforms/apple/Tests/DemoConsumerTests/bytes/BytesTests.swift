import Demo
import Foundation
import XCTest

final class BytesTests: XCTestCase {
    func testBytesFns() {
        XCTAssertEqual(echoBytes(data: Data([1, 2, 3, 4])), Data([1, 2, 3, 4]))
        XCTAssertEqual(bytesLength(data: Data([9, 8, 7])), 3)
        XCTAssertEqual(bytesSum(data: Data([1, 2, 3, 4])), 10)
        XCTAssertEqual(makeBytes(len: 4), Data([0, 1, 2, 3]))
        XCTAssertEqual(reverseBytes(data: Data([1, 2, 3, 4])), Data([4, 3, 2, 1]))
        XCTAssertEqual(generateBytes(size: 4), Data([42, 42, 42, 42]))
    }
}
